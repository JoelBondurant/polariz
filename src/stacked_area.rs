use crate::colors;
use crate::message::Message;
use crate::plot::{CoordinateTransformer, PlotKernel, PlotLayout};
use iced::advanced::mouse::Cursor;
use iced::widget::canvas::Frame;
use iced::widget::image;
use iced::{Rectangle, Task};
use polars::prelude::*;
use rand::RngExt;
use std::collections::HashMap;
use std::sync::Arc;
use wgpu::util::DeviceExt;

pub struct StackedAreaPlotKernel {
	pub prepared_data: Arc<StackedAreaPreparedData>,
	pub image_cache: Option<image::Handle>,
}

impl PlotKernel for StackedAreaPlotKernel {
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
			let (x_min, x_max) = self.prepared_data.x_range;
			if x < x_min || x > x_max { return None; }
			let xs = &self.prepared_data.unique_xs;
			if xs.len() < 2 { return None; }
			let idx = match xs.binary_search_by(|val| val.partial_cmp(&x).unwrap()) {
				Ok(i) => i,
				Err(i) => {
					if i == 0 { 0 }
					else if i == xs.len() { xs.len() - 1 }
					else if (xs[i] - x).abs() < (xs[i-1] - x).abs() { i } else { i - 1 }
				}
			};
			let actual_x = xs[idx];
			let mut current_stack_y = 0.0;
			for (j, cat_vals) in self.prepared_data.category_values.iter().enumerate() {
				let val = cat_vals[idx];
				if y >= current_stack_y && y <= current_stack_y + val {
					return Some(format!(
						"X: {:.2}\n{}: {:.2}\nTotal: {:.2}",
						actual_x, self.prepared_data.categories[j], val, current_stack_y + val
					));
				}
				current_stack_y += val;
			}
			return Some(format!("X: {:.2}, Total Sum: {:.2}", actual_x, current_stack_y));
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
	data: Arc<StackedAreaPreparedData>,
	width: u32,
	height: u32,
) -> (u32, u32, Vec<u8>) {
	rasterize_stacked_area_plot_internal(&data, width, height).await
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct AreaVertex {
	position: [f32; 4],
	color: [f32; 4],
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct AreaUniforms {
	x_min: f32,
	x_max: f32,
	y_min: f32,
	y_max: f32,
}

pub struct StackedAreaPreparedData {
	vertices: Vec<AreaVertex>,
	pub categories: Vec<String>,
	pub unique_xs: Vec<f32>,
	pub category_values: Vec<Vec<f32>>,
	pub x_range: (f32, f32),
	pub y_range: (f32, f32),
}

pub fn prepare_stacked_area_data(df: &DataFrame, cat_col: &str, x_col: &str, y_col: &str) -> StackedAreaPreparedData {
	let categories_series = df.column(cat_col).unwrap().unique().unwrap().sort(Default::default()).unwrap();
	let categories: Vec<String> = categories_series.as_materialized_series().iter().map(|v| {
		if let AnyValue::String(s) = v { s.to_string() } else { v.to_string() }
	}).collect();
	let unique_xs_series = df.column(x_col).unwrap().unique().unwrap().sort(Default::default()).unwrap();
	let unique_xs_f32 = unique_xs_series.cast(&DataType::Float32).unwrap();
	let unique_xs: Vec<f32> = unique_xs_f32.f32().unwrap().into_no_null_iter().collect();
	let num_cats = categories.len();
	let num_xs = unique_xs.len();
	if num_xs < 2 || num_cats == 0 {
		return StackedAreaPreparedData {
			vertices: Vec::new(),
			categories,
			unique_xs,
			category_values: Vec::new(),
			x_range: (0.0, 1.0),
			y_range: (0.0, 1.0),
		};
	}
	let aggregated = df.clone().lazy()
		.group_by([col(x_col), col(cat_col)])
		.agg([col(y_col).sum().alias("y_sum")])
		.collect()
		.unwrap();
	let mut category_values = vec![vec![0.0f32; num_xs]; num_cats];
	let cat_to_idx: HashMap<String, usize> = categories.iter().enumerate().map(|(i, s)| (s.clone(), i)).collect();
	let x_to_idx: HashMap<u32, usize> = unique_xs.iter().enumerate().map(|(i, &x)| (x.to_bits(), i)).collect();
	let binding_x = aggregated.column(x_col).unwrap().cast(&DataType::Float32).unwrap();
	let p_x = binding_x.f32().unwrap();
	let p_cat = aggregated.column(cat_col).unwrap();
	let binding_y = aggregated.column("y_sum").unwrap().cast(&DataType::Float32).unwrap();
	let p_y = binding_y.f32().unwrap();
	for i in 0..aggregated.height() {
		let x = p_x.get(i).unwrap();
		let cat_val = p_cat.get(i).unwrap();
		let cat_str = if let AnyValue::String(s) = cat_val { s.to_string() } else { cat_val.to_string() };
		let y = p_y.get(i).unwrap();
		if let (Some(&xi), Some(&ci)) = (x_to_idx.get(&x.to_bits()), cat_to_idx.get(&cat_str)) {
			category_values[ci][xi] = y;
		}
	}
	let mut max_sum = 0.0f32;
	for x_idx in 0..num_xs {
		let mut current_sum = 0.0f32;
		for cat_idx in 0..num_cats {
			current_sum += category_values[cat_idx][x_idx];
		}
		if current_sum > max_sum {
			max_sum = current_sum;
		}
	}
	let x_range = (unique_xs[0], unique_xs[num_xs - 1]);
	let y_range = (0.0, max_sum.max(0.001) * 1.05);
	let mut vertices = Vec::new();
	let mut prev_stacked_ys = vec![0.0f32; num_xs];
	let mut current_stacked_ys = vec![0.0f32; num_xs];
	for (cat_idx, cat_val) in category_values.iter().enumerate() {
		let t = if num_cats > 1 { cat_idx as f32 / (num_cats - 1) as f32 } else { 0.5 };
		let c = colors::viridis(t);
		let color = [c[0], c[1], c[2], 1.0];
		for x_idx in 0..num_xs {
			current_stacked_ys[x_idx] = prev_stacked_ys[x_idx] + cat_val[x_idx];
		}
		for x_idx in 0..num_xs - 1 {
			let x1 = unique_xs[x_idx];
			let x2 = unique_xs[x_idx + 1];
			let y1_low = prev_stacked_ys[x_idx];
			let y2_low = prev_stacked_ys[x_idx + 1];
			let y1_high = current_stacked_ys[x_idx];
			let y2_high = current_stacked_ys[x_idx + 1];
			vertices.push(AreaVertex { position: [x1, y1_low, 0.0, 1.0], color });
			vertices.push(AreaVertex { position: [x2, y2_low, 0.0, 1.0], color });
			vertices.push(AreaVertex { position: [x2, y2_high, 0.0, 1.0], color });
			vertices.push(AreaVertex { position: [x1, y1_low, 0.0, 1.0], color });
			vertices.push(AreaVertex { position: [x2, y2_high, 0.0, 1.0], color });
			vertices.push(AreaVertex { position: [x1, y1_high, 0.0, 1.0], color });
		}
		prev_stacked_ys.copy_from_slice(&current_stacked_ys);
	}
	StackedAreaPreparedData {
		vertices,
		categories,
		unique_xs,
		category_values,
		x_range,
		y_range,
	}
}

pub async fn rasterize_stacked_area_plot_internal(
	data: &StackedAreaPreparedData,
	width: u32,
	height: u32,
) -> (u32, u32, Vec<u8>) {
	if data.vertices.is_empty() {
		return (width, height, vec![0; (width * height * 4) as usize]);
	}
	let instance = wgpu::Instance::default();
	let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions::default()).await.unwrap();
	let (device, queue) = adapter.request_device(&wgpu::DeviceDescriptor::default()).await.unwrap();
	let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
		label: None,
		source: wgpu::ShaderSource::Wgsl(include_str!("stacked_area.wgsl").into()),
	});
	let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
		label: Some("Vertex Buffer"),
		contents: bytemuck::cast_slice(&data.vertices),
		usage: wgpu::BufferUsages::VERTEX,
	});
	let uniforms = AreaUniforms {
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
				array_stride: std::mem::size_of::<AreaVertex>() as wgpu::BufferAddress,
				step_mode: wgpu::VertexStepMode::Vertex,
				attributes: &wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4],
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
		size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
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
		wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
	);
	queue.submit(Some(encoder.finish()));
	let buffer_slice = output_buffer.slice(..);
	let (tx, rx) = std::sync::mpsc::channel();
	buffer_slice.map_async(wgpu::MapMode::Read, move |res| { let _ = tx.send(res); });
	loop {
		device.poll(wgpu::PollType::Wait { submission_index: None, timeout: None }).unwrap();
		if let Ok(res) = rx.try_recv() {
			res.unwrap();
			break;
		}
	}
	let data_map = buffer_slice.get_mapped_range();
	let mut pixel_data = Vec::with_capacity((width * height * 4) as usize);
	for chunk in data_map.chunks(bytes_per_row as usize) {
		pixel_data.extend_from_slice(&chunk[..(width * 4) as usize]);
	}
	drop(data_map);
	output_buffer.unmap();
	(width, height, pixel_data)
}

pub fn generate_sample_stacked_area_data() -> DataFrame {
	let num_cats = 12;
	let num_xs = 200;
	let total_n = num_cats * num_xs;
	let mut cats = Vec::with_capacity(total_n);
	let mut xs = Vec::with_capacity(total_n);
	let mut ys = Vec::with_capacity(total_n);
	let mut rng = rand::rng();
	for i in 0..num_cats {
		let cat = format!("Series {}", i);
		for j in 0..num_xs {
			cats.push(cat.clone());
			xs.push(j as f32);
			let trend = (j as f32 / 20.0).sin() + 2.0;
			let noise: f32 = rng.random_range(0.0..1.0f32);
			ys.push(trend + noise);
		}
	}
	DataFrame::new(total_n, vec![Column::new("cat".into(), cats), Column::new("x".into(), xs), Column::new("y".into(), ys)]).unwrap()
}
