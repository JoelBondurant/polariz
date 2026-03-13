use crate::message::Message;
use crate::plot::{CoordinateTransformer, PlotKernel, PlotLayout};
use iced::advanced::mouse::Cursor;
use iced::widget::canvas::Frame;
use iced::widget::image;
use iced::{Rectangle, Task};
use polars::prelude::*;
use rand::RngExt;
use std::sync::Arc;
use wgpu::util::DeviceExt;

pub struct LinePlotKernel {
	pub prepared_data: Arc<LinePreparedData>,
	pub image_cache: Option<image::Handle>,
}

impl PlotKernel for LinePlotKernel {
	fn layout(&self) -> PlotLayout {
		PlotLayout::Cartesian {
			x_range: self.prepared_data.x_range,
			y_range: self.prepared_data.y_range,
		}
	}

	fn draw_raster(
		&self,
		frame: &mut Frame,
		bounds: Rectangle,
		_transform: &CoordinateTransformer,
	) {
		if let Some(handle) = &self.image_cache {
			frame.draw_image(bounds, &handle.clone());
		}
	}

	fn draw_overlay(
		&self,
		_frame: &mut Frame,
		_bounds: Rectangle,
		_transform: &CoordinateTransformer,
		_cursor: Cursor,
	) {
	}

	fn hover(&self, transform: &CoordinateTransformer, cursor: Cursor) -> Option<String> {
		if let Some(cursor_pos) = cursor.position() {
			if let Some((x, y)) = transform.pixel_to_cartesian(cursor_pos) {
				return Some(format!("X: {:.2}, Y: {:.2}", x, y));
			}
		}
		None
	}

	fn rasterize(&self, width: u32, height: u32) -> Task<Message> {
		let data = self.prepared_data.clone();
		Task::perform(rasterize_task(data, width, height), |(w, h, pixels)| {
			Message::RasterizationResult(w, h, pixels)
		})
	}

	fn update_raster(&mut self, width: u32, height: u32, pixels: Vec<u8>) {
		self.image_cache = Some(image::Handle::from_rgba(width, height, pixels));
	}
}

async fn rasterize_task(
	data: Arc<LinePreparedData>,
	width: u32,
	height: u32,
) -> (u32, u32, Vec<u8>) {
	rasterize_line_plot_internal(&data, width, height).await
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct LineVertex {
	position: [f32; 2],
	color: [f32; 3],
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct LineUniforms {
	x_min: f32,
	x_max: f32,
	y_min: f32,
	y_max: f32,
}

pub struct LinePreparedData {
	vertices: Vec<LineVertex>,
	pub x_range: (f32, f32),
	pub y_range: (f32, f32),
}

fn viridis(t: f32) -> [f32; 3] {
	[
		0.184455
			+ t * (0.107708
				+ t * (-0.327241
					+ t * (-4.599932 + t * (6.203736 + t * (4.751787 + t * -5.432077))))),
		0.005768
			+ t * (1.396470
				+ t * (0.214814
					+ t * (-5.758238 + t * (14.153965 + t * (-13.749439 + t * 4.641571))))),
		0.267511
			+ t * (0.073383
				+ t * (15.657724
					+ t * (-90.257825 + t * (202.560788 + t * (-202.603108 + t * 74.394908))))),
	]
}

pub fn prepare_line_data(
	df: &DataFrame,
	cat_col: &str,
	x_col: &str,
	y_col: &str,
) -> LinePreparedData {
	let x_col_series = df.column(x_col).unwrap().cast(&DataType::Float32).unwrap();
	let y_col_series = df.column(y_col).unwrap().cast(&DataType::Float32).unwrap();
	let x_series = x_col_series.f32().unwrap();
	let y_series = y_col_series.f32().unwrap();
	let x_range = (x_series.min().unwrap_or(0.0), x_series.max().unwrap_or(1.0));
	let y_series_min = y_series.min().unwrap_or(0.0);
	let y_series_max = y_series.max().unwrap_or(1.0);
	let y_range = (y_series_min, y_series_max);
	let x_pad = (x_range.1 - x_range.0) * 0.05;
	let y_pad = (y_range.1 - y_range.0) * 0.05;
	let x_range = (x_range.0 - x_pad, x_range.1 + x_pad);
	let y_range = (y_range.0 - y_pad, y_range.1 + y_pad);
	let mut vertices = Vec::new();
	let partitions = df.partition_by([cat_col], true).unwrap();
	let num_partitions = partitions.len();
	for (i, group_df) in partitions.into_iter().enumerate() {
		let xs_col = group_df
			.column(x_col)
			.unwrap()
			.cast(&DataType::Float32)
			.unwrap();
		let ys_col = group_df
			.column(y_col)
			.unwrap()
			.cast(&DataType::Float32)
			.unwrap();
		let xs = xs_col.f32().unwrap();
		let ys = ys_col.f32().unwrap();
		let t = if num_partitions > 1 {
			i as f32 / (num_partitions - 1) as f32
		} else {
			0.5
		};
		let color = viridis(t);
		let mut prev_point: Option<[f32; 2]> = None;
		for j in 0..group_df.height() {
			let p = [xs.get(j).unwrap(), ys.get(j).unwrap()];
			if let Some(prev) = prev_point {
				vertices.push(LineVertex {
					position: prev,
					color,
				});
				vertices.push(LineVertex { position: p, color });
			}
			prev_point = Some(p);
		}
	}
	LinePreparedData {
		vertices,
		x_range,
		y_range,
	}
}

pub async fn rasterize_line_plot_internal(
	data: &LinePreparedData,
	width: u32,
	height: u32,
) -> (u32, u32, Vec<u8>) {
	let instance = wgpu::Instance::default();
	let adapter = instance
		.request_adapter(&wgpu::RequestAdapterOptions::default())
		.await
		.unwrap();
	let (device, queue) = adapter
		.request_device(&wgpu::DeviceDescriptor::default())
		.await
		.unwrap();

	let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
		label: None,
		source: wgpu::ShaderSource::Wgsl(include_str!("line.wgsl").into()),
	});
	let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
		label: Some("Vertex Buffer"),
		contents: bytemuck::cast_slice(&data.vertices),
		usage: wgpu::BufferUsages::VERTEX,
	});
	let uniforms = LineUniforms {
		x_min: data.x_range.0,
		x_max: data.x_range.1,
		y_min: data.y_range.0,
		y_max: data.y_range.1,
	};
	let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
		label: Some("Uniform Buffer"),
		contents: bytemuck::bytes_of(&uniforms),
		usage: wgpu::BufferUsages::UNIFORM,
	});
	let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
		label: None,
		entries: &[wgpu::BindGroupLayoutEntry {
			binding: 0,
			visibility: wgpu::ShaderStages::VERTEX,
			ty: wgpu::BindingType::Buffer {
				ty: wgpu::BufferBindingType::Uniform,
				has_dynamic_offset: false,
				min_binding_size: None,
			},
			count: None,
		}],
	});
	let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
		layout: &bind_group_layout,
		entries: &[wgpu::BindGroupEntry {
			binding: 0,
			resource: uniform_buffer.as_entire_binding(),
		}],
		label: None,
	});
	let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
		label: None,
		bind_group_layouts: &[&bind_group_layout],
		immediate_size: 0,
	});
	let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
		label: None,
		layout: Some(&pipeline_layout),
		vertex: wgpu::VertexState {
			module: &shader,
			entry_point: Some("vs_main"),
			buffers: &[wgpu::VertexBufferLayout {
				array_stride: std::mem::size_of::<LineVertex>() as wgpu::BufferAddress,
				step_mode: wgpu::VertexStepMode::Vertex,
				attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x3],
			}],
			compilation_options: Default::default(),
		},
		fragment: Some(wgpu::FragmentState {
			module: &shader,
			entry_point: Some("fs_main"),
			targets: &[Some(wgpu::ColorTargetState {
				format: wgpu::TextureFormat::Rgba8Unorm,
				blend: Some(wgpu::BlendState::ALPHA_BLENDING),
				write_mask: wgpu::ColorWrites::ALL,
			})],
			compilation_options: Default::default(),
		}),
		primitive: wgpu::PrimitiveState {
			topology: wgpu::PrimitiveTopology::LineList,
			..Default::default()
		},
		depth_stencil: None,
		multisample: wgpu::MultisampleState::default(),
		multiview_mask: None,
		cache: None,
	});
	let render_texture = device.create_texture(&wgpu::TextureDescriptor {
		size: wgpu::Extent3d {
			width,
			height,
			depth_or_array_layers: 1,
		},
		mip_level_count: 1,
		sample_count: 1,
		dimension: wgpu::TextureDimension::D2,
		format: wgpu::TextureFormat::Rgba8Unorm,
		usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
		label: None,
		view_formats: &[],
	});
	let mut encoder = device.create_command_encoder(&Default::default());
	{
		let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
			label: None,
			color_attachments: &[Some(wgpu::RenderPassColorAttachment {
				view: &render_texture.create_view(&Default::default()),
				resolve_target: None,
				ops: wgpu::Operations {
					load: wgpu::LoadOp::Clear(wgpu::Color {
						r: 0.01,
						g: 0.01,
						b: 0.03,
						a: 1.0,
					}),
					store: wgpu::StoreOp::Store,
				},
				depth_slice: None,
			})],
			..Default::default()
		});
		rpass.set_pipeline(&pipeline);
		rpass.set_bind_group(0, &bind_group, &[]);
		rpass.set_vertex_buffer(0, vertex_buffer.slice(..));
		rpass.draw(0..data.vertices.len() as u32, 0..1);
	}
	let bytes_per_row = (4 * width + 255) & !255;
	let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
		size: (bytes_per_row * height) as u64,
		usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
		label: None,
		mapped_at_creation: false,
	});
	encoder.copy_texture_to_buffer(
		render_texture.as_image_copy(),
		wgpu::TexelCopyBufferInfo {
			buffer: &output_buffer,
			layout: wgpu::TexelCopyBufferLayout {
				offset: 0,
				bytes_per_row: Some(bytes_per_row),
				rows_per_image: Some(height),
			},
		},
		wgpu::Extent3d {
			width,
			height,
			depth_or_array_layers: 1,
		},
	);
	queue.submit(Some(encoder.finish()));
	let buffer_slice = output_buffer.slice(..);
	let (tx, rx) = std::sync::mpsc::channel();
	buffer_slice.map_async(wgpu::MapMode::Read, move |res| tx.send(res).unwrap());
	device
		.poll(wgpu::PollType::Wait {
			submission_index: None,
			timeout: None,
		})
		.unwrap();
	rx.recv().unwrap().unwrap();
	let data_map = buffer_slice.get_mapped_range();
	let mut pixel_data = Vec::with_capacity((width * height * 4) as usize);
	for chunk in data_map.chunks(bytes_per_row as usize) {
		pixel_data.extend_from_slice(&chunk[..(width * 4) as usize]);
	}
	drop(data_map);
	output_buffer.unmap();
	(width, height, pixel_data)
}

pub fn generate_sample_line_data() -> DataFrame {
	let num_series = 5;
	let n_per_series = 1000;
	let total_n = num_series * n_per_series;
	let mut rng = rand::rng();
	let mut cats = Vec::with_capacity(total_n);
	let mut xs = Vec::with_capacity(total_n);
	let mut ys = Vec::with_capacity(total_n);
	for i in 0..num_series {
		let cat = format!("Series {}", i);
		let mut current_y: f32 = rng.random_range(0.0..10.0f32);
		for j in 0..n_per_series {
			cats.push(cat.clone());
			xs.push(j as f32);
			current_y += rng.random_range(-1.0..1.0f32);
			ys.push(current_y);
		}
	}
	DataFrame::new(
		total_n,
		vec![
			Column::new("cat".into(), cats),
			Column::new("x".into(), xs),
			Column::new("y".into(), ys),
		],
	)
	.unwrap()
}
