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

pub struct PiePlotKernel {
	pub prepared_data: Arc<PiePreparedData>,
	pub image_cache: Option<image::Handle>,
}

impl PlotKernel for PiePlotKernel {
	fn layout(&self) -> PlotLayout {
		PlotLayout::Radial
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
			let center = transform.bounds.center();
			let dx = (cursor_pos.x - center.x) * (transform.bounds.width / transform.bounds.height);
			let dy = -(cursor_pos.y - center.y); // Invert Y to match clip space
			let dist = (dx * dx + dy * dy).sqrt() / (transform.bounds.height / 2.0);
			if dist > 0.3 && dist < 0.8 {
				let pi = std::f32::consts::PI;
				let mut angle = dx.atan2(dy);
				if angle < 0.0 {
					angle += 2.0 * pi;
				}
				let normalized_angle = angle / (2.0 * pi);
				for (i, &limit) in self.prepared_data.cumulative_angles.iter().enumerate() {
					if normalized_angle < limit {
						let cat = &self.prepared_data.categories[i];
						let val = self.prepared_data.values[i];
						return Some(format!(
							"{}: {:.2} ({:.1}%)",
							cat,
							val,
							val / self.prepared_data.total_sum * 100.0
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
	data: Arc<PiePreparedData>,
	width: u32,
	height: u32,
) -> (u32, u32, Vec<u8>) {
	rasterize_pie_plot_internal(&data, width, height).await
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct PieUniforms {
	aspect_ratio: f32,
	num_sectors: u32,
}

pub struct PiePreparedData {
	pub categories: Vec<String>,
	pub values: Vec<f32>,
	pub cumulative_angles: Vec<f32>,
	pub total_sum: f32,
}

pub fn prepare_pie_data(df: &DataFrame, cat_col: &str, val_col: &str) -> PiePreparedData {
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
	let mut values = Vec::with_capacity(categories.len());
	let mut total_sum = 0.0f32;
	for cat_val in categories_series.as_materialized_series().iter() {
		let lit_val = match cat_val {
			AnyValue::String(s) => lit(s),
			AnyValue::Int32(i) => lit(i),
			AnyValue::Int64(i) => lit(i),
			_ => lit(cat_val.to_string()),
		};
		let filtered = df
			.clone()
			.lazy()
			.filter(col(cat_col).eq(lit_val))
			.select([col(val_col).sum()])
			.collect()
			.unwrap();
		let val = filtered
			.column(val_col)
			.unwrap()
			.cast(&DataType::Float32)
			.unwrap()
			.f32()
			.unwrap()
			.get(0)
			.unwrap_or(0.0);
		values.push(val);
		total_sum += val;
	}
	let mut cumulative_angles = Vec::with_capacity(values.len());
	let mut current_sum = 0.0f32;
	for &val in &values {
		current_sum += val;
		cumulative_angles.push(current_sum / total_sum);
	}
	PiePreparedData {
		categories,
		values,
		cumulative_angles,
		total_sum,
	}
}

pub async fn rasterize_pie_plot_internal(
	data: &PiePreparedData,
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
		source: wgpu::ShaderSource::Wgsl(include_str!("pie.wgsl").into()),
	});
	let uniforms = PieUniforms {
		aspect_ratio: width as f32 / height as f32,
		num_sectors: data.cumulative_angles.len() as u32,
	};
	let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
		label: Some("Uniform Buffer"),
		contents: bytemuck::bytes_of(&uniforms),
		usage: wgpu::BufferUsages::UNIFORM,
	});
	let angles_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
		label: Some("Angles Buffer"),
		contents: bytemuck::cast_slice(&data.cumulative_angles),
		usage: wgpu::BufferUsages::STORAGE,
	});
	let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
		label: None,
		entries: &[
			wgpu::BindGroupLayoutEntry {
				binding: 0,
				visibility: wgpu::ShaderStages::FRAGMENT,
				ty: wgpu::BindingType::Buffer {
					ty: wgpu::BufferBindingType::Uniform,
					has_dynamic_offset: false,
					min_binding_size: None,
				},
				count: None,
			},
			wgpu::BindGroupLayoutEntry {
				binding: 1,
				visibility: wgpu::ShaderStages::FRAGMENT,
				ty: wgpu::BindingType::Buffer {
					ty: wgpu::BufferBindingType::Storage { read_only: true },
					has_dynamic_offset: false,
					min_binding_size: None,
				},
				count: None,
			},
		],
	});
	let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
		layout: &bind_group_layout,
		entries: &[
			wgpu::BindGroupEntry {
				binding: 0,
				resource: uniform_buffer.as_entire_binding(),
			},
			wgpu::BindGroupEntry {
				binding: 1,
				resource: angles_buffer.as_entire_binding(),
			},
		],
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
			buffers: &[],
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
						r: 0.0,
						g: 0.0,
						b: 0.0,
						a: 0.0,
					}),
					store: wgpu::StoreOp::Store,
				},
				depth_slice: None,
			})],
			..Default::default()
		});
		rpass.set_pipeline(&pipeline);
		rpass.set_bind_group(0, &bind_group, &[]);
		rpass.draw(0..3, 0..1);
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

pub fn generate_sample_pie_data() -> DataFrame {
	let num_cats = 6;
	let mut cats = Vec::with_capacity(num_cats);
	let mut vals = Vec::with_capacity(num_cats);
	let mut rng = rand::rng();
	for i in 0..num_cats {
		cats.push(format!("Category {}", i + 1));
		vals.push(rng.random_range(10.0..100.0f32));
	}
	DataFrame::new(
		num_cats,
		vec![
			Column::new("cat".into(), cats),
			Column::new("val".into(), vals),
		],
	)
	.unwrap()
}
