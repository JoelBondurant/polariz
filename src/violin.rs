use crate::plot::{CoordinateTransformer, PlotKernel, PlotLayout};
use iced::advanced::mouse::Cursor;
use iced::widget::canvas::{self, Frame};
use iced::widget::image;
use iced::{Color, Point, Rectangle};
use polars::prelude::*;
use rand::RngExt;
use rand_distr::{Distribution, Normal};
use wgpu::util::DeviceExt;

pub struct ViolinPlotKernel {
	pub layout_cache: PlotLayout,
	pub image_cache: Option<image::Handle>,
	pub medians: Vec<f32>,
}

impl PlotKernel for ViolinPlotKernel {
	fn layout(&self) -> PlotLayout {
		self.layout_cache.clone()
	}

	fn draw_data(&self, frame: &mut Frame, bounds: Rectangle, _transform: &CoordinateTransformer) {
		if let Some(handle) = &self.image_cache {
			frame.draw_image(bounds, &handle.clone());
		}
	}

	fn draw_interaction(
		&self,
		frame: &mut Frame,
		_bounds: Rectangle,
		transform: &CoordinateTransformer,
		cursor: Cursor,
	) {
		if let Some(cursor_pos) = cursor.position()
			&& let PlotLayout::CategoricalX { categories, .. } = &self.layout_cache {
			for (i, cat) in categories.iter().enumerate() {
				let (center_point, band_width) = transform.categorical(i, 0.0);
				let left_edge = center_point.x - (band_width / 2.0);
				let right_edge = center_point.x + (band_width / 2.0);
				if cursor_pos.x >= left_edge && cursor_pos.x <= right_edge {
					if let Some(&median_val) = self.medians.get(i) {
						let (median_px, _) = transform.categorical(i, median_val);
						let path = canvas::Path::circle(median_px, 4.0);
						frame.fill(&path, Color::from_rgb(1.0, 0.2, 0.2));
						frame.fill_text(canvas::Text {
							content: format!("{}: Median {:.2}", cat, median_val),
							position: Point::new(cursor_pos.x + 10.0, cursor_pos.y - 15.0),
							color: Color::WHITE,
							size: iced::Pixels(14.0),
							..Default::default()
						});
					}
					break;
				}
			}
		}
	}
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct ScreenUniform {
	num_groups: u32,
	y_min: f32,
	y_max: f32,
	width_scale: f32,
}

fn compute_kde(
	y_vals: &[f32],
	num_bins: usize,
	y_min: f32,
	y_max: f32,
	bandwidth: f32,
) -> Vec<f32> {
	let mut density = vec![0.0; num_bins];
	let step = (y_max - y_min) / (num_bins as f32 - 1.0);
	let inv_bw = 1.0 / bandwidth;
	let norm_factor = 1.0 / (y_vals.len() as f32 * bandwidth * (2.0 * std::f32::consts::PI).sqrt());
	for (i, d) in density.iter_mut().enumerate() {
		let y = y_min + (i as f32) * step;
		let mut sum = 0.0;
		for &val in y_vals {
			let diff = (y - val) * inv_bw;
			sum += (-0.5 * diff * diff).exp();
		}
		*d = sum * norm_factor;
	}
	let max_d = density.iter().cloned().fold(f32::MIN, f32::max);
	if max_d > 0.0 {
		for d in density.iter_mut() {
			*d /= max_d;
		}
	}
	density
}

pub async fn generate_violin_plot(
	df: &DataFrame,
	cat_col: &str,
	val_col: &str,
	width: u32,
	height: u32,
	manual_range: Option<(f32, f32)>,
) -> (u32, u32, Vec<u8>) {
	let (y_min, y_max) = match manual_range {
		Some(r) => r,
		None => {
			let col = df
				.column(val_col)
				.unwrap()
				.as_materialized_series()
				.f32()
				.unwrap();
			let (y_min, y_max) = (col.min().unwrap(), col.max().unwrap());
			let pad = (y_max - y_min) * 0.002;
			(y_min - pad, y_max + pad)
		}
	};
	let group_data = df
		.clone()
		.lazy()
		.group_by([col(cat_col)])
		.agg([
			col(val_col).median().alias("median"),
			col(val_col).alias("values"),
		])
		.sort([cat_col], Default::default())
		.collect()
		.expect("Failed to aggregate data");
	let num_violins = group_data.height();
	let medians_series = group_data
		.column("median")
		.unwrap()
		.as_materialized_series()
		.f32()
		.unwrap();
	let values_list = group_data
		.column("values")
		.unwrap()
		.as_materialized_series()
		.list()
		.unwrap();
	let tex_height_bins = 1024;
	let mut tex_data = vec![0.0f32; num_violins * tex_height_bins];
	let mut medians_vec = Vec::with_capacity(num_violins);
	for i in 0..num_violins {
		medians_vec.push(medians_series.get(i).unwrap());
		let series = values_list.get_as_series(i).unwrap();
		let y_slice: Vec<f32> = series.f32().unwrap().into_no_null_iter().collect();
		let bandwidth = (y_max - y_min) * 0.03;
		let density = compute_kde(&y_slice, tex_height_bins, y_min, y_max, bandwidth);
		for bin in 0..tex_height_bins {
			tex_data[bin * num_violins + i] = density[bin];
		}
	}
	let instance = wgpu::Instance::default();
	let adapter = instance
		.request_adapter(&wgpu::RequestAdapterOptions::default())
		.await
		.unwrap();
	let (device, queue) = adapter
		.request_device(&wgpu::DeviceDescriptor::default())
		.await
		.unwrap();
	let kde_texture = device.create_texture_with_data(
		&queue,
		&wgpu::TextureDescriptor {
			label: None,
			size: wgpu::Extent3d {
				width: num_violins as u32,
				height: tex_height_bins as u32,
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
		bytemuck::cast_slice(&tex_data),
	);
	let screen_uniform = ScreenUniform {
		num_groups: num_violins as u32,
		y_min,
		y_max,
		width_scale: 0.4,
	};
	let uniform_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
		label: None,
		contents: bytemuck::bytes_of(&screen_uniform),
		usage: wgpu::BufferUsages::UNIFORM,
	});
	let medians_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
		label: None,
		contents: bytemuck::cast_slice(&medians_vec),
		usage: wgpu::BufferUsages::STORAGE,
	});
	let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
			wgpu::BindGroupLayoutEntry {
				binding: 2,
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
		layout: &bgl,
		entries: &[
			wgpu::BindGroupEntry {
				binding: 0,
				resource: uniform_buf.as_entire_binding(),
			},
			wgpu::BindGroupEntry {
				binding: 1,
				resource: wgpu::BindingResource::TextureView(
					&kde_texture.create_view(&Default::default()),
				),
			},
			wgpu::BindGroupEntry {
				binding: 2,
				resource: medians_buf.as_entire_binding(),
			},
		],
		label: None,
	});
	let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
		label: None,
		source: wgpu::ShaderSource::Wgsl(include_str!("violin.wgsl").into()),
	});
	let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
		label: None,
		layout: Some(
			&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
				label: None,
				bind_group_layouts: &[&bgl],
				immediate_size: 0,
			}),
		),
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
					load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
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
	let data = buffer_slice.get_mapped_range();
	let bytes_per_pixel = 4;
	let unpadded_bytes_per_row = width * bytes_per_pixel;
	let mut pixel_data = Vec::with_capacity((unpadded_bytes_per_row * height) as usize);
	for chunk in data.chunks(bytes_per_row as usize) {
		pixel_data.extend_from_slice(&chunk[..unpadded_bytes_per_row as usize]);
	}
	drop(data);
	output_buffer.unmap();
	(width, height, pixel_data)
}

pub fn generate_sample_data() -> DataFrame {
	let num_violins = 12;
	let n_per_group = 100_000;
	let total_n = n_per_group * num_violins;
	let mut rng = rand::rng();
	let mut xs = Vec::with_capacity(total_n);
	let mut ys = Vec::with_capacity(total_n);
	for group in 0..num_violins {
		for _ in 0..n_per_group {
			xs.push(group as i32);
			let val = match group % 4 {
				0 => {
					if rng.random_bool(0.4) {
						Normal::new(2.5, 0.6 + 0.1 * group as f32)
							.unwrap()
							.sample(&mut rng)
					} else {
						Normal::new(6.5, 0.8 + 0.1 * group as f32)
							.unwrap()
							.sample(&mut rng)
					}
				}
				1 => rng.random::<f32>().powf(2.0) * 8.0,
				2 => Normal::new(4.6, 0.3 + 0.1 * group as f32)
					.unwrap()
					.sample(&mut rng),
				_ => Normal::new(4.4, 1.8 + 0.1 * group as f32)
					.unwrap()
					.sample(&mut rng),
			};
			ys.push(val);
		}
	}
	DataFrame::new(
		total_n,
		vec![Column::new("group".into(), xs), Column::new("y".into(), ys)],
	)
	.unwrap()
}
