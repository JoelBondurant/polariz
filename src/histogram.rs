use crate::colors;
use crate::plot::{CoordinateTransformer, PlotKernel, PlotLayout};
use iced::advanced::mouse::Cursor;
use iced::widget::canvas::Frame;
use iced::{Color, Rectangle};
use polars::prelude::*;
use rand::RngExt;
use std::sync::Arc;

pub struct HistogramPlotKernel {
	pub prepared_data: Arc<HistogramPreparedData>,
}

impl PlotKernel for HistogramPlotKernel {
	fn layout(&self) -> PlotLayout {
		PlotLayout::Cartesian {
			x_range: self.prepared_data.x_range,
			y_range: self.prepared_data.y_range,
		}
	}

	fn plot(
		&self,
		frame: &mut Frame,
		_bounds: Rectangle,
		transform: &CoordinateTransformer,
		_cursor: Cursor,
	) {
		let num_bins = self.prepared_data.bin_counts.len();
		let (x_min, x_max) = self.prepared_data.x_range;
		let bin_width_data = (x_max - x_min) / num_bins as f32;
		for (i, &count) in self.prepared_data.bin_counts.iter().enumerate() {
			if count == 0 { continue; }
			let bin_start_x = x_min + i as f32 * bin_width_data;
			let bin_end_x = bin_start_x + bin_width_data;
			let p_top_left = transform.cartesian(bin_start_x, count as f32);
			let p_bottom_right = transform.cartesian(bin_end_x, 0.0);
			let rect = Rectangle {
				x: p_top_left.x,
				y: p_top_left.y,
				width: (p_bottom_right.x - p_top_left.x).max(1.0),
				height: (p_bottom_right.y - p_top_left.y).max(1.0),
			};
			let t = i as f32 / num_bins as f32;
			let color = colors::viridis(t);
			frame.fill_rectangle(rect.position(), rect.size(), color);
			frame.stroke(&iced::widget::canvas::Path::rectangle(rect.position(), rect.size()), iced::widget::canvas::Stroke {
				style: iced::widget::canvas::Style::Solid(Color::from_rgba(0.0, 0.0, 0.0, 0.2)),
				width: 0.5,
				..Default::default()
			});
		}
	}

	fn hover(&self, transform: &CoordinateTransformer, cursor: Cursor) -> Option<String> {
		if let Some(cursor_pos) = cursor.position()
			&& let Some((x, y)) = transform.pixel_to_cartesian(cursor_pos) {
			let num_bins = self.prepared_data.bin_counts.len();
			let (x_min, x_max) = self.prepared_data.x_range;
			let bin_width = (x_max - x_min) / num_bins as f32;
			if x >= x_min && x <= x_max {
				let bin_idx = if bin_width > 0.0 {
					((x - x_min) / bin_width).floor() as usize
				} else {
					0
				};
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
}

pub struct HistogramPreparedData {
	pub bin_counts: Vec<u32>,
	pub x_range: (f32, f32),
	pub y_range: (f32, f32),
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
	HistogramPreparedData {
		bin_counts,
		x_range,
		y_range,
	}
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
