use crate::colors;
use crate::message::Message;
use crate::plot::{CoordinateTransformer, PlotKernel, PlotLayout};
use iced::advanced::mouse::Cursor;
use iced::widget::canvas::Frame;
use iced::widget::image;
use iced::{Rectangle, Task};
use polars::prelude::*;
use std::sync::Arc;
use wgpu::util::DeviceExt;

pub struct BoxPlotKernel {
	pub prepared_data: Arc<BoxPlotPreparedData>,
	pub image_cache: Option<image::Handle>,
}

impl PlotKernel for BoxPlotKernel {
	fn layout(&self) -> PlotLayout {
		PlotLayout::CategoricalX {
			categories: self.prepared_data.categories.clone(),
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
		if let Some(cursor_pos) = cursor.position()
			&& let PlotLayout::CategoricalX {
				categories,
				y_range,
			} = self.layout() {
			for (i, category) in categories.iter().enumerate() {
				let (center, band_width) = transform.categorical(i, 0.0);
				let left = center.x - band_width / 2.0;
				let right = center.x + band_width / 2.0;
				if cursor_pos.x >= left && cursor_pos.x <= right {
					let stats = &self.prepared_data.stats[i];
					let y_scale = transform.bounds.height / (y_range.1 - y_range.0);
					let data_y = y_range.0
						+ (transform.bounds.y + transform.bounds.height - cursor_pos.y)
							/ y_scale;
					if data_y >= stats.min && data_y <= stats.max {
						return Some(format!(
							"{}\nMax: {:.2}\nQ3: {:.2}\nMedian: {:.2}\nQ1: {:.2}\nMin: {:.2}",
							category,
							stats.max,
							stats.q3,
							stats.median,
							stats.q1,
							stats.min
						));
					}
				}
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
	data: Arc<BoxPlotPreparedData>,
	width: u32,
	height: u32,
) -> (u32, u32, Vec<u8>) {
	rasterize_box_plot_internal(&data, width, height).await
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct BoxVertex {
	position: [f32; 2],
	color: [f32; 3],
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct BoxUniforms {
	y_min: f32,
	y_max: f32,
}

pub struct BoxStats {
	pub min: f32,
	pub q1: f32,
	pub median: f32,
	pub q3: f32,
	pub max: f32,
}

pub struct BoxPlotPreparedData {
	vertices: Vec<BoxVertex>,
	pub categories: Vec<String>,
	pub stats: Vec<BoxStats>,
	pub y_range: (f32, f32),
}

pub fn prepare_box_plot_data(
	df: &DataFrame,
	cat_col: &str,
	val_col: &str,
	height: u32,
) -> BoxPlotPreparedData {
	let categories_series = df
		.column(cat_col)
		.unwrap()
		.unique()
		.unwrap()
		.sort(Default::default())
		.unwrap();
	let categories: Vec<String> = categories_series
		.as_materialized_series()
		.iter()
		.map(|v| {
			if let AnyValue::String(s) = v {
				s.to_string()
			} else {
				v.to_string()
			}
		})
		.collect();
	let num_cats = categories.len();
	let mut stats = Vec::with_capacity(num_cats);
	let mut y_min_all = f32::MAX;
	let mut y_max_all = f32::MIN;
	for i in 0..num_cats {
		let cat_val = categories_series.as_materialized_series().get(i).unwrap();
		let lit_val = match cat_val {
			AnyValue::String(s) => lit(s),
			AnyValue::Int32(i) => lit(i),
			AnyValue::Int64(i) => lit(i),
			_ => lit(cat_val.to_string()),
		};
		let group_df = df
			.clone()
			.lazy()
			.filter(col(cat_col).eq(lit_val))
			.collect()
			.unwrap();
		let vals = group_df
			.column(val_col)
			.unwrap()
			.cast(&DataType::Float32)
			.unwrap();
		let v = vals.f32().unwrap();
		let mut sorted_v: Vec<f32> = v.into_no_null_iter().collect();
		if sorted_v.is_empty() {
			stats.push(BoxStats {
				min: 0.0,
				q1: 0.0,
				median: 0.0,
				q3: 0.0,
				max: 0.0,
			});
			continue;
		}
		sorted_v.sort_by(|a, b| a.partial_cmp(b).unwrap());
		let n = sorted_v.len();
		let min = sorted_v[0];
		let max = sorted_v[n - 1];
		let q1 = sorted_v[n / 4];
		let median = sorted_v[n / 2];
		let q3 = sorted_v[3 * n / 4];
		stats.push(BoxStats {
			min,
			q1,
			median,
			q3,
			max,
		});
		if min < y_min_all {
			y_min_all = min;
		}
		if max > y_max_all {
			y_max_all = max;
		}
	}
	if y_min_all == f32::MAX {
		y_min_all = 0.0;
		y_max_all = 1.0;
	}
	let pad = (y_max_all - y_min_all).max(0.001) * 0.1;
	let y_range = (y_min_all - pad, y_max_all + pad);
	let data_height = y_range.1 - y_range.0;
	let mut vertices = Vec::new();
	let total_band_width = 2.0 / num_cats as f32;
	let box_width = total_band_width * 0.6;
	let bar_offset = (total_band_width - box_width) / 2.0;
	let line_clip_width = 0.003f32;
	let line_data_height = (3.5 * data_height) / height as f32;
	for (i, s) in stats.iter().enumerate() {
		let cat_left = -1.0 + (i as f32 * total_band_width) + bar_offset;
		let cat_right = cat_left + box_width;
		let cat_center = (cat_left + cat_right) / 2.0;
		let t = if num_cats > 1 {
			i as f32 / (num_cats - 1) as f32
		} else {
			0.5
		};
		let color = colors::viridis(t);
		let line_color = [1.0, 1.0, 1.0]; // White lines for contrast
		add_rect(&mut vertices, cat_left, s.q1, cat_right, s.q3, color);
		add_rect(
			&mut vertices,
			cat_left,
			s.median - line_data_height / 2.0,
			cat_right,
			s.median + line_data_height / 2.0,
			line_color,
		);
		add_rect(
			&mut vertices,
			cat_center - line_clip_width,
			s.min,
			cat_center + line_clip_width,
			s.q1,
			line_color,
		);
		add_rect(
			&mut vertices,
			cat_center - line_clip_width,
			s.q3,
			cat_center + line_clip_width,
			s.max,
			line_color,
		);
		let cap_width = box_width * 0.4;
		add_rect(
			&mut vertices,
			cat_center - cap_width,
			s.min - line_data_height / 2.0,
			cat_center + cap_width,
			s.min + line_data_height / 2.0,
			line_color,
		);
		add_rect(
			&mut vertices,
			cat_center - cap_width,
			s.max - line_data_height / 2.0,
			cat_center + cap_width,
			s.max + line_data_height / 2.0,
			line_color,
		);
	}
	BoxPlotPreparedData {
		vertices,
		categories,
		stats,
		y_range,
	}
}

fn add_rect(vertices: &mut Vec<BoxVertex>, x1: f32, y1: f32, x2: f32, y2: f32, color: [f32; 3]) {
	vertices.push(BoxVertex {
		position: [x1, y1],
		color,
	});
	vertices.push(BoxVertex {
		position: [x2, y1],
		color,
	});
	vertices.push(BoxVertex {
		position: [x2, y2],
		color,
	});
	vertices.push(BoxVertex {
		position: [x1, y1],
		color,
	});
	vertices.push(BoxVertex {
		position: [x2, y2],
		color,
	});
	vertices.push(BoxVertex {
		position: [x1, y2],
		color,
	});
}

pub async fn rasterize_box_plot_internal(
	data: &BoxPlotPreparedData,
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
		source: wgpu::ShaderSource::Wgsl(include_str!("box_plot.wgsl").into()),
	});
	let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
		label: Some("Vertex Buffer"),
		contents: bytemuck::cast_slice(&data.vertices),
		usage: wgpu::BufferUsages::VERTEX,
	});
	let uniforms = BoxUniforms {
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
				array_stride: std::mem::size_of::<BoxVertex>() as wgpu::BufferAddress,
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
