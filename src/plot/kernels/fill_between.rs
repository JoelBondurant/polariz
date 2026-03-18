use crate::plot::common::{AxisType, CoordinateTransformer, PlotKernel, PlotLayout, PlotSettings, format_label, polars_type_to_axis_type};
use iced::advanced::mouse::Cursor;
use iced::widget::canvas::{Frame, Path, Stroke, Style};
use iced::Rectangle;
use polars::prelude::*;
use rand::RngExt;
use std::sync::Arc;

pub struct FillBetweenPlotKernel {
	pub prepared_data: Arc<FillBetweenPreparedData>,
}

impl PlotKernel for FillBetweenPlotKernel {
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
		if self.prepared_data.x.is_empty() {
			return;
		}
		let line_color = settings.color_theme.get_color(0.9);
		let band_color = {
			let mut c = settings.color_theme.get_color(0.1);
			c.a = 0.9;
			c
		};
		let band_path = Path::new(|builder| {
			for (i, &x) in self.prepared_data.x.iter().enumerate() {
				let p = transform.cartesian(x, self.prepared_data.y_upper[i]);
				if i == 0 {
					builder.move_to(p);
				} else {
					builder.line_to(p);
				}
			}
			for (i, &x) in self.prepared_data.x.iter().enumerate().rev() {
				let p = transform.cartesian(x, self.prepared_data.y_lower[i]);
				builder.line_to(p);
			}
			builder.close();
		});
		frame.fill(&band_path, band_color);
		let line_path = Path::new(|builder| {
			for (i, &x) in self.prepared_data.x.iter().enumerate() {
				let p = transform.cartesian(x, self.prepared_data.y_mid[i]);
				if i == 0 {
					builder.move_to(p);
				} else {
					builder.line_to(p);
				}
			}
		});
		let stroke = Stroke {
			style: Style::Solid(line_color),
			width: 2.5,
			..Default::default()
		};
		frame.stroke(&line_path, stroke);
	}

	fn hover(&self, transform: &CoordinateTransformer, cursor: Cursor) -> Option<String> {
		if let Some(cursor_pos) = cursor.position()
			&& let Some((x, y)) = transform.pixel_to_cartesian(cursor_pos)
		{
			let xs = &self.prepared_data.x;
			if xs.is_empty() {
				return None;
			}
			let idx = match xs.binary_search_by(|val| val.partial_cmp(&x).unwrap()) {
				Ok(i) => i,
				Err(i) => {
					if i == 0 {
						0
					} else if i == xs.len() {
						xs.len() - 1
					} else if (xs[i] - x).abs() < (xs[i - 1] - x).abs() {
						i
					} else {
						i - 1
					}
				}
			};
			return Some(format!(
				"X: {}\nMid: {:.2}\nRange: [{:.2}, {:.2}]\nCursor Y: {:.2}",
				format_label(xs[idx], self.prepared_data.x_axis_type),
				self.prepared_data.y_mid[idx],
				self.prepared_data.y_lower[idx],
				self.prepared_data.y_upper[idx],
				y
			));
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

pub struct FillBetweenPreparedData {
	pub x: Vec<f64>,
	pub y_mid: Vec<f64>,
	pub y_lower: Vec<f64>,
	pub y_upper: Vec<f64>,
	pub x_range: (f64, f64),
	pub y_range: (f64, f64),
	pub x_axis_type: AxisType,
	pub y_axis_type: AxisType,
	pub x_label: String,
	pub y_label: String,
}

pub fn prepare_fill_between_data(
	df: &DataFrame,
	x_col: &str,
	y_mid_col: &str,
	y_lower_col: &str,
	y_upper_col: &str,
) -> FillBetweenPreparedData {
	let x_dtype = df.column(x_col).unwrap().dtype();
	let y_mid_dtype = df.column(y_mid_col).unwrap().dtype();
	let x_axis_type = polars_type_to_axis_type(x_dtype);
	let y_axis_type = polars_type_to_axis_type(y_mid_dtype);

	let x_series = df
		.column(x_col)
		.unwrap()
		.cast(&DataType::Float64)
		.unwrap()
		.f64()
		.unwrap()
		.into_no_null_iter()
		.collect::<Vec<_>>();
	let y_mid = df
		.column(y_mid_col)
		.unwrap()
		.cast(&DataType::Float64)
		.unwrap()
		.f64()
		.unwrap()
		.into_no_null_iter()
		.collect::<Vec<_>>();
	let y_lower = df
		.column(y_lower_col)
		.unwrap()
		.cast(&DataType::Float64)
		.unwrap()
		.f64()
		.unwrap()
		.into_no_null_iter()
		.collect::<Vec<_>>();
	let y_upper = df
		.column(y_upper_col)
		.unwrap()
		.cast(&DataType::Float64)
		.unwrap()
		.f64()
		.unwrap()
		.into_no_null_iter()
		.collect::<Vec<_>>();
	let x_min = x_series.iter().copied().fold(f64::INFINITY, f64::min);
	let x_max = x_series.iter().copied().fold(f64::NEG_INFINITY, f64::max);
	let y_min = y_lower.iter().copied().fold(f64::INFINITY, f64::min);
	let y_max = y_upper.iter().copied().fold(f64::NEG_INFINITY, f64::max);
	let x_pad = (x_max - x_min) * 0.001;
	let y_pad = (y_max - y_min) * 0.001;
	FillBetweenPreparedData {
		x: x_series,
		y_mid,
		y_lower,
		y_upper,
		x_range: (x_min - x_pad, x_max + x_pad),
		y_range: (y_min - y_pad, y_max + y_pad),
		x_axis_type,
		y_axis_type,
		x_label: x_col.to_string(),
		y_label: y_mid_col.to_string(),
	}
}

pub fn generate_sample_fill_between_data() -> DataFrame {
	let n = 600;
	let mut x = Vec::with_capacity(n);
	let mut y_mid = Vec::with_capacity(n);
	let mut y_lower = Vec::with_capacity(n);
	let mut y_upper = Vec::with_capacity(n);
	let mut rng = rand::rng();
	let mut current_y = 50.0f32;
	for i in 0..n {
		x.push(i as f32);
		current_y += rng.random_range(-1.0..1.0f32);
		y_mid.push(current_y);
		let spread = rng.random_range(4.0..8.0f32);
		y_lower.push(current_y - spread);
		y_upper.push(current_y + spread);
	}
	DataFrame::new(
		n,
		vec![
			Column::new("x".into(), x),
			Column::new("y_mid".into(), y_mid),
			Column::new("y_lower".into(), y_lower),
			Column::new("y_upper".into(), y_upper),
		],
	)
	.unwrap()
}
