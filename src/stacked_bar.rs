use crate::colors;
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
use std::collections::HashMap;

pub struct StackedBarPlotKernel {
	pub prepared_data: Arc<StackedBarPreparedData>,
	pub image_cache: Option<image::Handle>,
}

impl PlotKernel for StackedBarPlotKernel {
	fn layout(&self) -> PlotLayout {
		PlotLayout::CategoricalX {
			categories: self.prepared_data.categories.clone(),
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
			&& let PlotLayout::CategoricalX { categories, y_range } = self.layout() {
			for (i, cat_name) in categories.iter().enumerate() {
				let (center, band_width) = transform.categorical(i, 0.0);
				let left = center.x - band_width / 2.0;
				let right = center.x + band_width / 2.0;
				if cursor_pos.x >= left && cursor_pos.x <= right {
					let bar_width = band_width * 0.8;
					let bar_offset = (band_width - bar_width) / 2.0;
					let bar_left = left + bar_offset;
					let bar_right = left + bar_offset + bar_width;
					if cursor_pos.x >= bar_left && cursor_pos.x <= bar_right {
						let y_scale = transform.bounds.height / (y_range.1 - y_range.0);
						let data_y = y_range.0 + (transform.bounds.y + transform.bounds.height - cursor_pos.y) / y_scale;
						let mut current_sum = 0.0;
						for (j, &val) in self.prepared_data.category_values[i].iter().enumerate() {
							if data_y >= current_sum && data_y <= current_sum + val {
								let group_name = &self.prepared_data.group_names[j];
								return Some(format!("{}: {} (Value: {:.2}, Total: {:.2})", cat_name, group_name, val, current_sum + val));
							}
							current_sum += val;
						}
						return Some(format!("{}: Total {:.2}", cat_name, current_sum));
					}
				}
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
	data: Arc<StackedBarPreparedData>,
	width: u32,
	height: u32,
) -> (u32, u32, Vec<u8>) {
	rasterize_stacked_bar_plot_internal(&data, width, height).await
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct BarVertex {
	position: [f32; 2],
	color: [f32; 3],
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct BarUniforms {
	y_min: f32,
	y_max: f32,
}

pub struct StackedBarPreparedData {
	vertices: Vec<BarVertex>,
	pub categories: Vec<String>,
	pub group_names: Vec<String>,
	pub category_values: Vec<Vec<f32>>, // For hover detection
	pub y_range: (f32, f32),
}

pub fn prepare_stacked_bar_data(df: &DataFrame, cat_col: &str, group_col: &str, val_col: &str) -> StackedBarPreparedData {
	let categories_series = df.column(cat_col).unwrap().unique().unwrap().sort(Default::default()).unwrap();
	let categories: Vec<String> = categories_series.as_materialized_series().iter().map(|v| {
		if let AnyValue::String(s) = v { s.to_string() } else { v.to_string() }
	}).collect();
	let groups_series = df.column(group_col).unwrap().unique().unwrap().sort(Default::default()).unwrap();
	let groups_series_mat = groups_series.as_materialized_series();
	let group_names: Vec<String> = groups_series_mat.iter().map(|v| {
		if let AnyValue::String(s) = v { s.to_string() } else { v.to_string() }
	}).collect();
	let group_idx_map: HashMap<AnyValue, usize> = groups_series_mat.iter().enumerate().map(|(i, v)| (v.into_static(), i)).collect();
	let num_cats = categories.len();
	let num_groups = group_names.len();
	let mut category_values = vec![vec![0.0f32; num_groups]; num_cats];
	let mut max_sum = 0.0f32;
	let partitions = df.partition_by([cat_col], true).unwrap();
	for (i, group_df) in partitions.into_iter().enumerate() {
		let group_partitions = group_df.partition_by([group_col], true).unwrap();
		let mut current_cat_sum = 0.0f32;
		for sub_group_df in group_partitions {
			let group_val = sub_group_df.column(group_col).unwrap().get(0).unwrap();
			if let Some(&group_idx) = group_idx_map.get(&group_val) {
				let val = sub_group_df.column(val_col).unwrap().cast(&DataType::Float32).unwrap().f32().unwrap().get(0).unwrap_or(0.0);
				category_values[i][group_idx] = val;
				current_cat_sum += val;
			}
		}
		if current_cat_sum > max_sum {
			max_sum = current_cat_sum;
		}
	}
	let y_min = 0.0f32;
	let y_range = (y_min, max_sum * 1.1);
	let mut vertices = Vec::with_capacity(num_cats * num_groups * 6);
	let group_colors: Vec<[f32; 3]> = (0..num_groups).map(|i| {
		let t = if num_groups > 1 { i as f32 / (num_groups - 1) as f32 } else { 0.5 };
		colors::viridis(t)
	}).collect();
	let total_band_width = 2.0 / num_cats as f32;
	let bar_width = total_band_width * 0.8;
	let bar_offset = (total_band_width - bar_width) / 2.0;
	for (i, cat_val) in category_values.iter().enumerate() {
		let cat_left = -1.0 + (i as f32 * total_band_width) + bar_offset;
		let cat_right = cat_left + bar_width;
		let mut current_y = 0.0f32;
		for j in 0..num_groups {
			let val = cat_val[j];
			if val <= 0.0 { continue; }
			let color = group_colors[j];
			let y_start = current_y;
			let y_end = current_y + val;
			vertices.push(BarVertex { position: [cat_left, y_start], color });
			vertices.push(BarVertex { position: [cat_right, y_start], color });
			vertices.push(BarVertex { position: [cat_right, y_end], color });
			vertices.push(BarVertex { position: [cat_left, y_start], color });
			vertices.push(BarVertex { position: [cat_right, y_end], color });
			vertices.push(BarVertex { position: [cat_left, y_end], color });
			current_y = y_end;
		}
	}
	StackedBarPreparedData {
		vertices,
		categories,
		group_names,
		category_values,
		y_range,
	}
}

pub async fn rasterize_stacked_bar_plot_internal(
	data: &StackedBarPreparedData,
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
		source: wgpu::ShaderSource::Wgsl(include_str!("stacked_bar.wgsl").into()),
	});
	let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
		label: Some("Vertex Buffer"),
		contents: bytemuck::cast_slice(&data.vertices),
		usage: wgpu::BufferUsages::VERTEX,
	});
	let uniforms = BarUniforms {
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
				array_stride: std::mem::size_of::<BarVertex>() as wgpu::BufferAddress,
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

pub fn generate_sample_stacked_bar_data() -> DataFrame {
	let num_cats = 8;
	let num_groups = 5;
	let total_n = num_cats * num_groups;
	let mut cats = Vec::with_capacity(total_n);
	let mut groups = Vec::with_capacity(total_n);
	let mut vals = Vec::with_capacity(total_n);
	let mut rng = rand::rng();
	for i in 0..num_cats {
		let cat = format!("Cat {}", i);
		for j in 0..num_groups {
			let group = format!("Group {}", j);
			cats.push(cat.clone());
			groups.push(group);
			vals.push(rng.random_range(5.0..25.0f32));
		}
	}
	DataFrame::new(
		total_n,
		vec![
			Column::new("cat".into(), cats),
			Column::new("group".into(), groups),
			Column::new("val".into(), vals),
		],
	)
	.unwrap()
}
