use crate::message::Message;
use crate::plot::{CoordinateTransformer, PlotKernel, PlotLayout};
use iced::advanced::mouse::Cursor;
use iced::widget::canvas::Frame;
use iced::widget::image;
use iced::{Rectangle, Task};
use polars::lazy::prelude::*;
use polars::prelude::*;
use rand_distr::{Distribution, Normal};
use std::sync::Arc;
use wgpu::util::DeviceExt;

pub struct HexbinPlotKernel {
	pub prepared_data: Arc<HexbinPreparedData>,
	pub image_cache: Option<image::Handle>,
}

impl PlotKernel for HexbinPlotKernel {
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
		if let Some(cursor_pos) = cursor.position()
			&& let Some((x, y)) = transform.pixel_to_cartesian(cursor_pos) {
			return Some(format!("X: {:.2}, Y: {:.2}", x, y));
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
	data: Arc<HexbinPreparedData>,
	width: u32,
	height: u32,
) -> (u32, u32, Vec<u8>) {
	rasterize_hexbin_plot_internal(&data, width, height).await
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct ScreenUniform {
	aspect_ratio: f32,
	max_count: f32,
	radius: f32,
	min_q: i32,
	min_r: i32,
	_pad: [u32; 3],
}

pub struct HexbinPreparedData {
	pub tex_data: Vec<f32>,
	pub min_q: i32,
	pub min_r: i32,
	pub tex_width: u32,
	pub tex_height: u32,
	pub max_count: u32,
	pub radius: f32,
	pub x_range: (f32, f32),
	pub y_range: (f32, f32),
}

fn bin_data_to_hex(df: DataFrame, radius: f32) -> PolarsResult<DataFrame> {
	let sqrt_3 = 3.0f32.sqrt();
	let with_frac = df.lazy().with_columns([
		((lit(sqrt_3 / 3.0) * col("x") - lit(1.0f32 / 3.0) * col("y")) / lit(radius))
			.alias("q_frac"),
		((lit(2.0f32 / 3.0) * col("y")) / lit(radius)).alias("r_frac"),
	]);
	let with_rounded = with_frac.with_columns([
		col("q_frac").round(0, RoundMode::HalfToEven).alias("q"),
		col("r_frac").round(0, RoundMode::HalfToEven).alias("r"),
		(-col("q_frac") - col("r_frac"))
			.round(0, RoundMode::HalfToEven)
			.alias("s"),
	]);
	let with_diffs = with_rounded.with_columns([
		(col("q_frac") - col("q")).abs().alias("q_diff"),
		(col("r_frac") - col("r")).abs().alias("r_diff"),
		((-col("q_frac") - col("r_frac")) - col("s"))
			.abs()
			.alias("s_diff"),
	]);
	let with_corrected = with_diffs.with_columns([
		when(
			col("q_diff")
				.gt(col("r_diff"))
				.and(col("q_diff").gt(col("s_diff"))),
		)
		.then(-col("r") - col("s"))
		.otherwise(col("q"))
		.cast(DataType::Int32)
		.alias("q"),
		when(
			col("q_diff")
				.gt(col("r_diff"))
				.and(col("q_diff").gt(col("s_diff")))
				.not()
				.and(col("r_diff").gt(col("s_diff"))),
		)
		.then(-col("q") - col("s"))
		.otherwise(col("r"))
		.cast(DataType::Int32)
		.alias("r"),
	]);
	with_corrected
		.group_by([col("q"), col("r")])
		.agg([len().alias("count")])
		.collect()
}

pub fn prepare_hexbin_data(df: &DataFrame, radius: f32) -> HexbinPreparedData {
	let x_col = df.column("x").unwrap().f32().unwrap();
	let y_col = df.column("y").unwrap().f32().unwrap();
	let x_range = (x_col.min().unwrap_or(0.0), x_col.max().unwrap_or(1.0));
	let y_range = (y_col.min().unwrap_or(0.0), y_col.max().unwrap_or(1.0));

	let binned = bin_data_to_hex(df.clone(), radius).unwrap();
	let q_col = binned.column("q").unwrap().i32().unwrap();
	let r_col = binned.column("r").unwrap().i32().unwrap();
	let count_col = binned.column("count").unwrap().u32().unwrap();
	let min_q = q_col.min().unwrap_or(0);
	let min_r = r_col.min().unwrap_or(0);
	let max_q = q_col.max().unwrap_or(0);
	let max_r = r_col.max().unwrap_or(0);
	let tex_width = (max_q - min_q + 1) as u32;
	let tex_height = (max_r - min_r + 1) as u32;
	let mut tex_data = vec![0.0f32; (tex_width * tex_height) as usize];
	let mut max_count = 0;
	for i in 0..binned.height() {
		let q = q_col.get(i).unwrap();
		let r = r_col.get(i).unwrap();
		let count = count_col.get(i).unwrap();
		let x = (q - min_q) as u32;
		let y = (r - min_r) as u32;
		tex_data[(y * tex_width + x) as usize] = count as f32;
		if count > max_count {
			max_count = count;
		}
	}
	HexbinPreparedData {
		tex_data,
		min_q,
		min_r,
		tex_width,
		tex_height,
		max_count,
		radius,
		x_range,
		y_range,
	}
}

pub async fn rasterize_hexbin_plot_internal(
	data: &HexbinPreparedData,
	width: u32,
	height: u32,
) -> (u32, u32, Vec<u8>) {
	let aspect_ratio = width as f32 / height as f32;
	let instance = wgpu::Instance::default();
	let adapter = instance
		.request_adapter(&wgpu::RequestAdapterOptions::default())
		.await
		.unwrap();
	let (device, queue) = adapter
		.request_device(&wgpu::DeviceDescriptor::default())
		.await
		.unwrap();
	let density_texture = device.create_texture_with_data(
		&queue,
		&wgpu::TextureDescriptor {
			label: Some("Density Texture"),
			size: wgpu::Extent3d {
				width: data.tex_width,
				height: data.tex_height,
				depth_or_array_layers: 1,
			},
			mip_level_count: 1,
			sample_count: 1,
			dimension: wgpu::TextureDimension::D2,
			format: wgpu::TextureFormat::R32Float,
			usage: wgpu::TextureUsages::TEXTURE_BINDING,
			view_formats: &[],
		},
		wgpu::util::TextureDataOrder::MipMajor,
		bytemuck::cast_slice(&data.tex_data),
	);
	let density_view = density_texture.create_view(&wgpu::TextureViewDescriptor::default());
	let screen_uniform = ScreenUniform {
		aspect_ratio,
		max_count: data.max_count as f32,
		radius: data.radius,
		min_q: data.min_q,
		min_r: data.min_r,
		_pad: [0; 3],
	};
	let screen_uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
		label: None,
		contents: bytemuck::bytes_of(&screen_uniform),
		usage: wgpu::BufferUsages::UNIFORM,
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
				ty: wgpu::BindingType::Texture {
					sample_type: wgpu::TextureSampleType::Float { filterable: false },
					view_dimension: wgpu::TextureViewDimension::D2,
					multisampled: false,
				},
				count: None,
			},
		],
	});
	let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
		label: None,
		layout: &bind_group_layout,
		entries: &[
			wgpu::BindGroupEntry {
				binding: 0,
				resource: screen_uniform_buffer.as_entire_binding(),
			},
			wgpu::BindGroupEntry {
				binding: 1,
				resource: wgpu::BindingResource::TextureView(&density_view),
			},
		],
	});
	let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
		label: None,
		source: wgpu::ShaderSource::Wgsl(include_str!("hexbin.wgsl").into()),
	});
	let render_texture_desc = wgpu::TextureDescriptor {
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
	};
	let render_texture = device.create_texture(&render_texture_desc);
	let render_view = render_texture.create_view(&wgpu::TextureViewDescriptor::default());
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
				format: render_texture_desc.format,
				blend: None,
				write_mask: wgpu::ColorWrites::ALL,
			})],
			compilation_options: Default::default(),
		}),
		primitive: wgpu::PrimitiveState::default(),
		depth_stencil: None,
		multisample: wgpu::MultisampleState::default(),
		multiview_mask: None,
		cache: None,
	});
	let mut encoder =
		device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
	{
		let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
			label: None,
			color_attachments: &[Some(wgpu::RenderPassColorAttachment {
				view: &render_view,
				resolve_target: None,
				ops: wgpu::Operations {
					load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
					store: wgpu::StoreOp::Store,
				},
				depth_slice: None,
			})],
			depth_stencil_attachment: None,
			timestamp_writes: None,
			occlusion_query_set: None,
			multiview_mask: None,
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
		render_texture_desc.size,
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
	let data = buffer_slice.get_mapped_range();
	let mut pixel_data = Vec::with_capacity((width * height * 4) as usize);
	for chunk in data.chunks(bytes_per_row as usize) {
		pixel_data.extend_from_slice(&chunk[..(width * 4) as usize]);
	}
	drop(data);
	output_buffer.unmap();
	(width, height, pixel_data)
}

pub fn generate_sample_hex_data(width: u32, height: u32) -> DataFrame {
	let aspect_ratio = width as f32 / height as f32;
	let n = 1_000_000usize;
	let mut rng = rand::rng();
	let normal_x = Normal::new(0.5 * aspect_ratio, 0.12).unwrap();
	let normal_y = Normal::new(0.5, 0.12).unwrap();
	let xs: Vec<f32> = (0..n).map(|_| normal_x.sample(&mut rng)).collect();
	let ys: Vec<f32> = (0..n).map(|_| normal_y.sample(&mut rng)).collect();
	DataFrame::new(
		n,
		vec![Column::new("x".into(), xs), Column::new("y".into(), ys)],
	)
	.unwrap()
}
