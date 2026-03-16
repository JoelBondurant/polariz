use crate::colors;
use crate::plot::{CoordinateTransformer, PlotKernel, PlotLayout};
use iced::advanced::mouse::Cursor;
use iced::widget::canvas::{self, Frame, Path, Stroke, Style};
use iced::{Color, Rectangle};
use polars::prelude::*;
use rand::RngExt;
use rand_distr::{Distribution, Normal};
use std::sync::Arc;

pub struct ViolinPlotKernel {
	pub prepared_data: Arc<ViolinPreparedData>,
}

impl PlotKernel for ViolinPlotKernel {
	fn layout(&self) -> PlotLayout {
		PlotLayout::CategoricalX {
			categories: self.prepared_data.categories.clone(),
			y_range: self.prepared_data.y_range,
		}
	}

	fn plot(
		&self,
		frame: &mut Frame,
		_bounds: Rectangle,
		transform: &CoordinateTransformer,
		cursor: Cursor,
	) {
		let num_violins = self.prepared_data.categories.len();
		let tex_height_bins = self.prepared_data.tex_height_bins;
		let (y_min, y_max) = self.prepared_data.y_range;
		let y_step = (y_max - y_min) / (tex_height_bins as f32 - 1.0);
		for i in 0..num_violins {
			let (_center, band_width) = transform.categorical(i, 0.0);
			let width_scale = band_width * 0.4;
			let t = if num_violins > 1 { i as f32 / (num_violins - 1) as f32 } else { 0.5 };
			let color = colors::viridis(t);
			let mut first_bin = 0;
			for bin in 0..tex_height_bins {
				if self.prepared_data.kde_data[i * tex_height_bins + bin] > 0.01 {
					first_bin = bin;
					break;
				}
			}
			let mut last_bin = tex_height_bins - 1;
			for bin in (0..tex_height_bins).rev() {
				if self.prepared_data.kde_data[i * tex_height_bins + bin] > 0.01 {
					last_bin = bin;
					break;
				}
			}
			if first_bin >= last_bin { continue; }
			let violin_path = Path::new(|builder| {
				for bin in first_bin..=last_bin {
					let data_y = y_min + bin as f32 * y_step;
					let density = self.prepared_data.kde_data[i * tex_height_bins + bin];
					let (p, _) = transform.categorical(i, data_y);
					if bin == first_bin {
						builder.move_to(iced::Point::new(p.x - density * width_scale, p.y));
					} else {
						builder.line_to(iced::Point::new(p.x - density * width_scale, p.y));
					}
				}
				for bin in (first_bin..=last_bin).rev() {
					let data_y = y_min + bin as f32 * y_step;
					let density = self.prepared_data.kde_data[i * tex_height_bins + bin];
					let (p, _) = transform.categorical(i, data_y);
					builder.line_to(iced::Point::new(p.x + density * width_scale, p.y));
				}
				builder.close();
			});
			frame.fill(&violin_path, color);
			let border_color = colors::viridis(1.0 - t);
			let border_stroke = Stroke {
				style: Style::Solid(border_color),
				width: 2.5,
				..Default::default()
			};
			frame.stroke(&violin_path, border_stroke);
			if let Some(&median_val) = self.prepared_data.medians.get(i) {
				let (median_px, _) = transform.categorical(i, median_val);
				let bin_idx = (((median_val - y_min) / (y_max - y_min)) * (tex_height_bins as f32 - 1.0))
					.floor() as usize;
				let bin_idx = bin_idx.min(tex_height_bins - 1);
				let density = self.prepared_data.kde_data[i * tex_height_bins + bin_idx];
				let line_half_width = density * width_scale;
				let median_path = Path::new(|builder| {
					builder.move_to(iced::Point::new(median_px.x - line_half_width, median_px.y));
					builder.line_to(iced::Point::new(median_px.x + line_half_width, median_px.y));
				});
				frame.stroke(&median_path, Stroke {
					style: Style::Solid(Color::WHITE),
					width: 4.0,
					..Default::default()
				});
			}
		}
		if let Some(cursor_pos) = cursor.position() {
			for i in 0..num_violins {
				let (center, band_width) = transform.categorical(i, 0.0);
				let left_edge = center.x - (band_width / 2.0);
				let right_edge = center.x + (band_width / 2.0);
				if cursor_pos.x >= left_edge && cursor_pos.x <= right_edge {
					if let Some(&median_val) = self.prepared_data.medians.get(i) {
						let (median_px, _) = transform.categorical(i, median_val);
						let path = canvas::Path::circle(median_px, 5.0);
						frame.stroke(&path, Stroke {
							style: Style::Solid(Color::from_rgb(1.0, 0.2, 0.2)),
							width: 2.0,
							..Default::default()
						});
					}
					break;
				}
			}
		}
	}

	fn hover(&self, transform: &CoordinateTransformer, cursor: Cursor) -> Option<String> {
		if let Some(cursor_pos) = cursor.position()
			&& let PlotLayout::CategoricalX { categories, .. } = self.layout() {
			for (i, cat) in categories.iter().enumerate() {
				let (center_point, band_width) = transform.categorical(i, 0.0);
				let left_edge = center_point.x - (band_width / 2.0);
				let right_edge = center_point.x + (band_width / 2.0);
				if cursor_pos.x >= left_edge && cursor_pos.x <= right_edge
					&& let Some(&median_val) = self.prepared_data.medians.get(i) {
					return Some(format!("{}: Median {:.2}", cat, median_val));
				}
			}
		}
		None
	}
}

pub struct ViolinPreparedData {
	pub categories: Vec<String>,
	pub y_range: (f32, f32),
	pub medians: Vec<f32>,
	pub kde_data: Vec<f32>, // [violin_idx * tex_height_bins + bin_idx]
	pub tex_height_bins: usize,
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

pub fn prepare_violin_data(
	df: &DataFrame,
	cat_col: &str,
	val_col: &str,
	manual_range: Option<(f32, f32)>,
) -> ViolinPreparedData {
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
			let pad = (y_max - y_min) * 0.1;
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
	let categories_series = group_data.column(cat_col).unwrap().as_materialized_series();
	let categories: Vec<String> = if let Ok(ca) = categories_series.i32() {
		ca.into_no_null_iter().map(|i| i.to_string()).collect()
	} else {
		categories_series.iter().map(|v| {
			if let AnyValue::String(s) = v { s.to_string() } else { v.to_string() }
		}).collect()
	};
	let tex_height_bins = 256;
	let mut kde_data = vec![0.0f32; num_violins * tex_height_bins];
	let mut medians = Vec::with_capacity(num_violins);
	for i in 0..num_violins {
		medians.push(medians_series.get(i).unwrap());
		let series = values_list.get_as_series(i).unwrap();
		let y_slice: Vec<f32> = series.f32().unwrap().into_no_null_iter().collect();
		let bandwidth = (y_max - y_min) * 0.03;
		let density = compute_kde(&y_slice, tex_height_bins, y_min, y_max, bandwidth);
		for bin in 0..tex_height_bins {
			kde_data[i * tex_height_bins + bin] = density[bin];
		}
	}
	ViolinPreparedData {
		categories,
		y_range: (y_min, y_max),
		medians,
		kde_data,
		tex_height_bins,
	}
}

pub fn generate_sample_data() -> DataFrame {
	let num_violins = 12;
	let n_per_group = 10_000;
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
