use crate::plot::{CoordinateTransformer, PlotKernel, PlotLayout, AxisType, polars_type_to_axis_type};
use iced::advanced::mouse::Cursor;
use iced::widget::canvas::{Frame, Path, Stroke, Style};
use iced::{Point, Rectangle, Size};
use polars::prelude::*;
use rand::RngExt;
use std::sync::Arc;

pub struct CandlestickPlotKernel {
	pub prepared_data: Arc<CandlestickPreparedData>,
}

impl PlotKernel for CandlestickPlotKernel {
	fn layout(&self) -> PlotLayout {
		PlotLayout::Cartesian {
			x_range: self.prepared_data.x_range,
			y_range: self.prepared_data.y_range,
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
		settings: crate::plot::PlotSettings,
	) {
		let n = self.prepared_data.x.len();
		if n == 0 { return; }
		let x_delta = if n > 1 {
			(self.prepared_data.x[1] - self.prepared_data.x[0]).abs()
		} else {
			1.0
		};
		let x_scale = transform.bounds.width as f64 / (self.prepared_data.x_range.1 - self.prepared_data.x_range.0);
		let candle_width = (x_scale * x_delta * 0.7).max(1.0) as f32;
		let bullish_color = settings.color_theme.get_color(1.0);
		let bearish_color = settings.color_theme.get_color(0.0);
		for i in 0..n {
			let x = self.prepared_data.x[i];
			let open = self.prepared_data.open[i];
			let high = self.prepared_data.high[i];
			let low = self.prepared_data.low[i];
			let close = self.prepared_data.close[i];
			let p_high = transform.cartesian(x, high);
			let p_low = transform.cartesian(x, low);
			let p_open = transform.cartesian(x, open);
			let p_close = transform.cartesian(x, close);
			let is_bullish = close >= open;
			let color = if is_bullish { bullish_color } else { bearish_color };
			let wick_path = Path::new(|builder| {
				builder.move_to(p_low);
				builder.line_to(p_high);
			});
			frame.stroke(&wick_path, Stroke {
				style: Style::Solid(color),
				width: 3.0,
				..Default::default()
			});
			let body_top = p_open.y.min(p_close.y);
			let body_bottom = p_open.y.max(p_close.y);
			let body_height = (body_bottom - body_top).max(1.0);
			let body_x = p_open.x - candle_width / 2.0;
			frame.fill_rectangle(
				Point::new(body_x, body_top),
				Size::new(candle_width, body_height),
				color,
			);
			let marks_path = Path::new(|builder| {
				builder.move_to(Point::new(p_open.x - candle_width / 2.0, p_open.y));
				builder.line_to(Point::new(p_open.x, p_open.y));
				builder.move_to(Point::new(p_close.x, p_close.y));
				builder.line_to(Point::new(p_close.x + candle_width / 2.0, p_close.y));
			});
			frame.stroke(&marks_path, Stroke {
				style: Style::Solid(settings.decoration_color),
				width: 2.0,
				..Default::default()
			});
		}
	}

	fn hover(&self, transform: &CoordinateTransformer, cursor: Cursor) -> Option<String> {
		if let Some(cursor_pos) = cursor.position()
			&& let Some((x, _y)) = transform.pixel_to_cartesian(cursor_pos) {
			let xs = &self.prepared_data.x;
			if xs.is_empty() { return None; }
			let idx = match xs.binary_search_by(|val| val.partial_cmp(&x).unwrap()) {
				Ok(i) => i,
				Err(i) => {
					if i == 0 { 0 }
					else if i == xs.len() { xs.len() - 1 }
					else if (xs[i] - x).abs() < (xs[i-1] - x).abs() { i } else { i - 1 }
				}
			};
			let x_scale = transform.bounds.width as f64 / (self.prepared_data.x_range.1 - self.prepared_data.x_range.0);
			let dist_px = (xs[idx] - x).abs() * x_scale;
			if dist_px > 10.0 { return None; }
			return Some(format!(
				"X: {}\nOpen: {:.2}\nHigh: {:.2}\nLow: {:.2}\nClose: {:.2}",
				crate::plot::format_label(xs[idx], self.prepared_data.x_axis_type),
				self.prepared_data.open[idx],
				self.prepared_data.high[idx],
				self.prepared_data.low[idx],
				self.prepared_data.close[idx]
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

pub struct CandlestickPreparedData {
	pub x: Vec<f64>,
	pub open: Vec<f64>,
	pub high: Vec<f64>,
	pub low: Vec<f64>,
	pub close: Vec<f64>,
	pub x_range: (f64, f64),
	pub y_range: (f64, f64),
	pub x_axis_type: AxisType,
	pub y_axis_type: AxisType,
	pub x_label: String,
	pub y_label: String,
}

pub fn prepare_candlestick_data(
	df: &DataFrame,
	x_col: &str,
	open_col: &str,
	high_col: &str,
	low_col: &str,
	close_col: &str,
) -> CandlestickPreparedData {
	let x_dtype = df.column(x_col).unwrap().dtype();
	let x_axis_type = polars_type_to_axis_type(x_dtype);
	let y_axis_type = AxisType::Linear;
	let x = df.column(x_col).unwrap().cast(&DataType::Float64).unwrap().f64().unwrap().into_no_null_iter().collect::<Vec<_>>();
	let open = df.column(open_col).unwrap().cast(&DataType::Float64).unwrap().f64().unwrap().into_no_null_iter().collect::<Vec<_>>();
	let high = df.column(high_col).unwrap().cast(&DataType::Float64).unwrap().f64().unwrap().into_no_null_iter().collect::<Vec<_>>();
	let low = df.column(low_col).unwrap().cast(&DataType::Float64).unwrap().f64().unwrap().into_no_null_iter().collect::<Vec<_>>();
	let close = df.column(close_col).unwrap().cast(&DataType::Float64).unwrap().f64().unwrap().into_no_null_iter().collect::<Vec<_>>();
	let x_min = x.iter().copied().fold(f64::INFINITY, f64::min);
	let x_max = x.iter().copied().fold(f64::NEG_INFINITY, f64::max);
	let y_min = low.iter().copied().fold(f64::INFINITY, f64::min);
	let y_max = high.iter().copied().fold(f64::NEG_INFINITY, f64::max);
	let x_pad = (x_max - x_min) * 0.05;
	let y_pad = (y_max - y_min) * 0.05;
	CandlestickPreparedData {
		x,
		open,
		high,
		low,
		close,
		x_range: (x_min - x_pad, x_max + x_pad),
		y_range: (y_min - y_pad, y_max + y_pad),
		x_axis_type,
		y_axis_type,
		x_label: x_col.to_string(),
		y_label: "Value".to_string(),
	}
}

pub fn generate_sample_candlestick_data() -> DataFrame {
	let n = 100;
	let mut x_vals = Vec::with_capacity(n);
	let mut open = Vec::with_capacity(n);
	let mut high = Vec::with_capacity(n);
	let mut low = Vec::with_capacity(n);
	let mut close = Vec::with_capacity(n);
	let mut rng = rand::rng();
	let mut current_price = 100.0f64;
	let start_time = chrono::Utc::now().naive_utc().and_utc().timestamp_millis();
	let hour_ms = 3600 * 1000;
	for i in 0..n {
		x_vals.push(start_time + (i as i64 * hour_ms));
		let o = current_price;
		let c = o + rng.random_range(-5.0..5.0f64);
		let h = o.max(c) + rng.random_range(0.0..3.0f64);
		let l = o.min(c) - rng.random_range(0.0..3.0f64);
		open.push(o);
		close.push(c);
		high.push(h);
		low.push(l);
		current_price = c;
	}
	let x_series = Series::new("x".into(), x_vals)
		.cast(&DataType::Datetime(polars::prelude::TimeUnit::Milliseconds, None))
		.unwrap();
	DataFrame::new(
		n,
		vec![
			Column::from(x_series),
			Column::new("open".into(), open),
			Column::new("high".into(), high),
			Column::new("low".into(), low),
			Column::new("close".into(), close),
		],
	)
	.unwrap()
}
