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

pub struct HistogramPlotKernel {
	pub prepared_data: Arc<HistogramPreparedData>,
	pub image_cache: Option<image::Handle>,
}

impl PlotKernel for HistogramPlotKernel {
	fn layout(&self) -> PlotLayout {
		PlotLayout::Cartesian {
			x_range: self.prepared_data.x_range,
			y_range: self.prepared_data.y_range,
		}
	}

	fn draw_raster(&self, frame: &mut Frame, bounds: Rectangle, _transform: &CoordinateTransformer) {
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
		if let Some(cursor_pos) = cursor.position()
			&& let Some((x, y)) = transform.pixel_to_cartesian(cursor_pos) {
			let num_bins = self.prepared_data.bin_counts.len();
			let (x_min, x_max) = self.prepared_data.x_range;
			let bin_width = (x_max - x_min) / num_bins as f32;
			if x >= x_min && x <= x_max {
				let bin_idx = ((x - x_min) / bin_width).floor() as usize;
				let bin_idx = bin_idx.min(num_bins - 1);
				let count = self.prepared_data.bin_counts[bin_idx];
				let bin_start = x_min + bin_idx as f32 * bin_width;
				let bin_end = bin_start + bin_width;
				return Some(format!(
					"Range: [{:.2}, {:.2}]\nCount: {}\nY-Value: {:.2}",
					bin_start, bin_end, count, y
				));
			}
		}
		None
	}

	fn rasterize(&self, width: u32, height: u32) -> Task<Message> {
		let data = self.prepared_data.clone();
		Task::perform(
			rasterize_task(data, width, height),
			|(w, h, pixels)| Message::RasterizationResult(w, h, pixels),
		)
	}

	fn update_raster(&mut self, width: u32, height: u32, pixels: Vec<u8>) {
		self.image_cache = Some(image::Handle::from_rgba(width, height, pixels));
	}
}

async fn rasterize_task(
	data: Arc<HistogramPreparedData>,
	width: u32,
	height: u32,
) -> (u32, u32, Vec<u8>) {
	rasterize_histogram_plot_internal(&data, width, height).await
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct HistogramVertex {
	position: [f32; 2],
	color: [f32; 3],
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct HistogramUniforms {
	y_min: f32,
	y_max: f32,
}

pub struct HistogramPreparedData {
	vertices: Vec<HistogramVertex>,
	pub bin_counts: Vec<u32>,
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
			+ t * (1.39647
				+ t * (0.214814
					+ t * (-5.758238 + t * (14.153965 + t * (-13.749439 + t * 4.641571))))),
		0.267511
			+ t * (0.073383
				+ t * (15.657724
					+ t * (-90.25783 + t * (202.56079 + t * (-202.603 + t * 74.394908))))),
	]
}

pub fn prepare_histogram_data(df: &DataFrame, val_col: &str, num_bins: usize) -> HistogramPreparedData {
	let vals = df.column(val_col).unwrap().cast(&DataType::Float32).unwrap();
	let v = vals.f32().unwrap();
	let x_min = v.min().unwrap_or(0.0);
	let x_max = v.max().unwrap_or(1.0);
	let x_range = (x_min, x_max);
	let mut bin_counts = vec![0u32; num_bins];
	let bin_width = (x_max - x_min) / num_bins as f32;
	for val in v.into_no_null_iter() {
		let bin_idx = if bin_width > 0.0 {
			((val - x_min) / bin_width).floor() as usize
		} else {
			0
		};
		let bin_idx = bin_idx.min(num_bins - 1);
		bin_counts[bin_idx] += 1;
	}
	let y_max = bin_counts.iter().cloned().max().unwrap_or(1) as f32;
	let y_min = 0.0f32;
	let y_range = (y_min, y_max * 1.1);
	let mut vertices = Vec::with_capacity(num_bins * 6);
	let bin_clip_width = 2.0 / num_bins as f32;
	for (i, count) in bin_counts.iter().enumerate() {
		if count == &0 { continue; }
		let t = i as f32 / num_bins as f32;
		let color = viridis(t);
		let x_start = -1.0 + (i as f32 * bin_clip_width);
		let x_end = x_start + bin_clip_width;
		let y_start = y_min;
		let y_end = *count as f32;
		vertices.push(HistogramVertex { position: [x_start, y_start], color });
		vertices.push(HistogramVertex { position: [x_end, y_start], color });
		vertices.push(HistogramVertex { position: [x_end, y_end], color });
		vertices.push(HistogramVertex { position: [x_start, y_start], color });
		vertices.push(HistogramVertex { position: [x_end, y_end], color });
		vertices.push(HistogramVertex { position: [x_start, y_end], color });
	}
	HistogramPreparedData {
		vertices,
		bin_counts,
		x_range,
		y_range,
	}
}

pub async fn rasterize_histogram_plot_internal(
	data: &HistogramPreparedData,
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
		source: wgpu::ShaderSource::Wgsl(include_str!("histogram.wgsl").into()),
	});
	let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
		label: Some("Vertex Buffer"),
		contents: bytemuck::cast_slice(&data.vertices),
		usage: wgpu::BufferUsages::VERTEX,
	});
	let uniforms = HistogramUniforms {
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
				array_stride: std::mem::size_of::<HistogramVertex>() as wgpu::BufferAddress,
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
			topology: wgpu::PrimitiveTopology::TriangleList,
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
					load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.01, g: 0.01, b: 0.03, a: 1.0 }),
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
	device.poll(wgpu::PollType::Wait {
		submission_index: None,
		timeout: None,
	}).unwrap();
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

pub fn generate_sample_histogram_data() -> DataFrame {
	let n = 100_000usize;
	let mut rng = rand::rng();
	use rand_distr::{Distribution, Normal};
	let d1 = Normal::new(2.5, 0.8).unwrap();
	let d2 = Normal::new(6.5, 1.2).unwrap();
	let mut vals = Vec::with_capacity(n);
	for _ in 0..n {
		if rng.random_bool(0.4) {
			vals.push(d1.sample(&mut rng));
		} else {
			vals.push(d2.sample(&mut rng));
		}
	}
	DataFrame::new(
		n,
		vec![
			Column::new("val".into(), vals),
		],
	)
	.unwrap()
}
