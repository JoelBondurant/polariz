use crate::plot::common::{AxisType, CoordinateTransformer, PlotKernel, PlotLayout, PlotSettings, format_label, polars_type_to_axis_type};
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
	fn layout(&self, settings: PlotSettings) -> PlotLayout {
		PlotLayout::Cartesian {
			x_range: (
				settings.x_min.unwrap_or(self.prepared_data.x_range.0),
				settings.x_max.unwrap_or(self.prepared_data.x_range.1),
			),
			y_range: (
				settings.y_min.unwrap_or(self.prepared_data.y_range.0),
				settings.y_max.unwrap_or(self.prepared_data.y_range.1),
			),
			x_axis_type: self.prepared_data.x_axis_type,
			y_axis_type: self.prepared_data.y_axis_type,
		}
	}

	fn plot(
		&self,
		frame: &mut Frame,
		_bounds: Rectangle,
		transform: &CoordinateTransformer,
		_cursor: Cursor,
		settings: PlotSettings,
	) {
		let num_bins = self.prepared_data.bin_counts.len();
		let (x_min, x_max) = self.prepared_data.x_range;
		let bin_width_data = (x_max - x_min) / num_bins as f64;
		let max_count = self.prepared_data.max_count as f64;
		for (i, &count) in self.prepared_data.bin_counts.iter().enumerate() {
			if count == 0 { continue; }
			let bin_start_x = x_min + i as f64 * bin_width_data;
			let bin_end_x = bin_start_x + bin_width_data;
			let p_top_left = transform.cartesian(bin_start_x, count as f64);
			let p_bottom_right = transform.cartesian(bin_end_x, 0.0);
			let rect = Rectangle {
				x: p_top_left.x,
				y: p_top_left.y,
				width: (p_bottom_right.x - p_top_left.x).max(1.0),
				height: (p_bottom_right.y - p_top_left.y).max(1.0),
			};
			let t = count as f32 / max_count as f32;
			let color = settings.color_theme.get_color(t);
			frame.fill_rectangle(rect.position(), rect.size(), color);
			frame.stroke(&iced::widget::canvas::Path::rectangle(rect.position(), rect.size()), iced::widget::canvas::Stroke {
				style: iced::widget::canvas::Style::Solid(Color::from_rgba(0.0, 0.0, 0.0, 0.2)),
				width: 0.5,
				..Default::default()
			});
		}
	}

	fn draw_legend(&self, frame: &mut Frame, bounds: Rectangle, settings: PlotSettings) {
		let max_count = self.prepared_data.max_count;
		let legend_width = 60.0;
		let legend_height = 200.0;
		let legend_padding = 10.0;
		let x = bounds.x + (bounds.width - legend_width) * settings.legend_x;
		let y = bounds.y + (bounds.height - legend_height) * settings.legend_y;
		frame.fill_rectangle(
			iced::Point::new(x, y),
			iced::Size::new(legend_width, legend_height),
			Color { a: 0.6, ..settings.background_color }
		);
		let bar_width = 15.0;
		let bar_height = legend_height - 55.0;
		let bar_x = x + legend_padding;
		let bar_y = y + 35.0;
		let steps = 50;
		for i in 0..steps {
			let t = i as f32 / (steps - 1) as f32;
			let color = settings.color_theme.get_color(t);
			let step_height = bar_height / steps as f32;
			let step_y = bar_y + bar_height - (i as f32 + 1.0) * step_height;
			frame.fill_rectangle(
				iced::Point::new(bar_x, step_y),
				iced::Size::new(bar_width, step_height + 0.5),
				color
			);
		}
		frame.stroke(
			&iced::widget::canvas::Path::rectangle(iced::Point::new(bar_x, bar_y), iced::Size::new(bar_width, bar_height)),
			iced::widget::canvas::Stroke {
				style: iced::widget::canvas::Style::Solid(settings.decoration_color),
				width: 1.0,
				..Default::default()
			}
		);
		let label_x = bar_x + bar_width + 5.0;
		frame.fill_text(iced::widget::canvas::Text {
			content: format!("{}", max_count),
			position: iced::Point::new(label_x, bar_y),
			color: settings.decoration_color,
			size: iced::Pixels(settings.legend_size),
			align_y: iced::alignment::Vertical::Top,
			..Default::default()
		});
		frame.fill_text(iced::widget::canvas::Text {
			content: "0".to_string(),
			position: iced::Point::new(label_x, bar_y + bar_height),
			color: settings.decoration_color,
			size: iced::Pixels(settings.legend_size),
			align_y: iced::alignment::Vertical::Bottom,
			..Default::default()
		});
		frame.fill_text(iced::widget::canvas::Text {
			content: "Frequency".to_string(),
			position: iced::Point::new(x + legend_width / 2.0, y + 10.0),
			color: settings.decoration_color,
			size: iced::Pixels(settings.legend_size),
			align_x: iced::alignment::Horizontal::Center.into(),
			align_y: iced::alignment::Vertical::Top,
			..Default::default()
		});
	}

	fn hover(&self, transform: &CoordinateTransformer, cursor: Cursor) -> Option<String> {
		if let Some(cursor_pos) = cursor.position()
			&& let Some((x, y)) = transform.pixel_to_cartesian(cursor_pos) {
			let num_bins = self.prepared_data.bin_counts.len();
			let (x_min, x_max) = self.prepared_data.x_range;
			let bin_width = (x_max - x_min) / num_bins as f64;
			if x >= x_min && x <= x_max {
				let bin_idx = if bin_width > 0.0 {
					((x - x_min) / bin_width).floor() as usize
				} else {
					0
				};
				let bin_idx = bin_idx.min(num_bins - 1);
				let count = self.prepared_data.bin_counts[bin_idx];
				let bin_start = x_min + bin_idx as f64 * bin_width;
				let bin_end = bin_start + bin_width;
				return Some(format!(
					"Range: [{}, {}]\nCount: {}\nY-Value: {:.2}",
					format_label(bin_start, self.prepared_data.x_axis_type),
					format_label(bin_end, self.prepared_data.x_axis_type),
					count, y
				));
			}
		}
		None
	}

	fn x_label(&self) -> String {
		self.prepared_data.x_label.clone()
	}

	fn y_label(&self) -> String {
		self.prepared_data.y_label.clone()
	}
}

pub struct HistogramPreparedData {
	pub bin_counts: Vec<u32>,
	pub x_range: (f64, f64),
	pub y_range: (f64, f64),
	pub x_axis_type: AxisType,
	pub y_axis_type: AxisType,
	pub max_count: u32,
	pub x_label: String,
	pub y_label: String,
}

pub fn prepare_histogram_data(df: &DataFrame, val_col: &str, num_bins: usize) -> HistogramPreparedData {
	let val_dtype = df.column(val_col).unwrap().dtype();
	let x_axis_type = polars_type_to_axis_type(val_dtype);
	let y_axis_type = AxisType::Linear;
	let vals = df.column(val_col).unwrap().cast(&DataType::Float64).unwrap();
	let v = vals.f64().unwrap();
	let x_min = v.min().unwrap_or(0.0);
	let x_max = v.max().unwrap_or(1.0);
	let x_range = (x_min, x_max);
	let mut bin_counts = vec![0u32; num_bins];
	let bin_width = (x_max - x_min) / num_bins as f64;
	for val in v.into_no_null_iter() {
		let bin_idx = if bin_width > 0.0 {
			((val - x_min) / bin_width).floor() as usize
		} else {
			0
		};
		let bin_idx = bin_idx.min(num_bins - 1);
		bin_counts[bin_idx] += 1;
	}
	let actual_max = bin_counts.iter().cloned().max().unwrap_or(1);
	let y_max = actual_max as f64;
	let y_min = 0.0f64;
	let y_range = (y_min, y_max * 1.1);
	HistogramPreparedData {
		bin_counts,
		x_range,
		y_range,
		x_axis_type,
		y_axis_type,
		max_count: actual_max,
		x_label: val_col.to_string(),
		y_label: "Frequency".to_string(),
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
