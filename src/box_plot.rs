use crate::colors;
use crate::plot::{CoordinateTransformer, PlotKernel, PlotLayout};
use iced::advanced::mouse::Cursor;
use iced::widget::canvas::{Frame, Path, Stroke, Style};
use iced::{Color, Rectangle};
use polars::prelude::*;
use std::sync::Arc;

pub struct BoxPlotKernel {
	pub prepared_data: Arc<BoxPlotPreparedData>,
}

impl PlotKernel for BoxPlotKernel {
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
		_cursor: Cursor,
	) {
		let num_cats = self.prepared_data.categories.len();
		for (i, stats) in self.prepared_data.stats.iter().enumerate() {
			let (center, band_width) = transform.categorical(i, 0.0);
			let box_width = band_width * 0.6;
			let left = center.x - box_width / 2.0;
			let right = center.x + box_width / 2.0;
			let t = if num_cats > 1 { i as f32 / (num_cats - 1) as f32 } else { 0.5 };
			let color = colors::viridis(t);
			let color_iced = color;
			let line_color = Color::WHITE;
			let (q1_px, _) = transform.categorical(i, stats.q1);
			let (q3_px, _) = transform.categorical(i, stats.q3);
			let box_rect = Rectangle {
				x: left,
				y: q3_px.y,
				width: box_width,
				height: (q1_px.y - q3_px.y).abs().max(1.0),
			};
			frame.fill_rectangle(box_rect.position(), box_rect.size(), color_iced);
			let outline_stroke = Stroke {
				style: Style::Solid(line_color),
				width: 1.0,
				..Default::default()
			};
			let box_path = Path::rectangle(box_rect.position(), box_rect.size());
			frame.stroke(&box_path, outline_stroke);
			let (median_px, _) = transform.categorical(i, stats.median);
			let median_stroke = Stroke {
				style: Style::Solid(line_color),
				width: 3.5,
				..Default::default()
			};
			let median_path = Path::new(|builder| {
				builder.move_to(iced::Point::new(left, median_px.y));
				builder.line_to(iced::Point::new(right, median_px.y));
			});
			frame.stroke(&median_path, median_stroke);
			let whisker_stroke = Stroke {
				style: Style::Solid(line_color),
				width: 1.0,
				..Default::default()
			};
			let (min_px, _) = transform.categorical(i, stats.min);
			let (max_px, _) = transform.categorical(i, stats.max);
			let whiskers_path = Path::new(|builder| {
				// Lower whisker
				builder.move_to(iced::Point::new(center.x, min_px.y));
				builder.line_to(iced::Point::new(center.x, q1_px.y));
				// Upper whisker
				builder.move_to(iced::Point::new(center.x, q3_px.y));
				builder.line_to(iced::Point::new(center.x, max_px.y));
			});
			frame.stroke(&whiskers_path, whisker_stroke);
			let cap_width = box_width * 0.4;
			let cap_stroke = Stroke {
				style: Style::Solid(line_color),
				width: 3.5,
				..Default::default()
			};
			let caps_path = Path::new(|builder| {
				builder.move_to(iced::Point::new(center.x - cap_width / 2.0, min_px.y));
				builder.line_to(iced::Point::new(center.x + cap_width / 2.0, min_px.y));
				builder.move_to(iced::Point::new(center.x - cap_width / 2.0, max_px.y));
				builder.line_to(iced::Point::new(center.x + cap_width / 2.0, max_px.y));
			});
			frame.stroke(&caps_path, cap_stroke);
		}
	}

	fn hover(&self, transform: &CoordinateTransformer, cursor: Cursor) -> Option<String> {
		if let Some(cursor_pos) = cursor.position()
			&& let PlotLayout::CategoricalX {
				categories,
				y_range,
			} = self.layout() {
			for (i, category) in categories.iter().enumerate() {
				let (center, band_width) = transform.categorical(i, 0.0);
				let left = center.x - band_width / 2.0;
				let right = center.x + band_width / 2.0;
				if cursor_pos.x >= left && cursor_pos.x <= right {
					let stats = &self.prepared_data.stats[i];
					let y_scale = transform.bounds.height / (y_range.1 - y_range.0);
					let data_y = y_range.0
						+ (transform.bounds.y + transform.bounds.height - cursor_pos.y)
							/ y_scale;
					if data_y >= stats.min && data_y <= stats.max {
						return Some(format!(
							"{}\nMax: {:.2}\nQ3: {:.2}\nMedian: {:.2}\nQ1: {:.2}\nMin: {:.2}",
							category,
							stats.max,
							stats.q3,
							stats.median,
							stats.q1,
							stats.min
						));
					}
				}
			}
		}
		None
	}
}

pub struct BoxStats {
	pub min: f32,
	pub q1: f32,
	pub median: f32,
	pub q3: f32,
	pub max: f32,
}

pub struct BoxPlotPreparedData {
	pub categories: Vec<String>,
	pub stats: Vec<BoxStats>,
	pub y_range: (f32, f32),
}

pub fn prepare_box_plot_data(
	df: &DataFrame,
	cat_col: &str,
	val_col: &str,
) -> BoxPlotPreparedData {
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
	let num_cats = categories.len();
	let mut stats = Vec::with_capacity(num_cats);
	let mut y_min_all = f32::MAX;
	let mut y_max_all = f32::MIN;
	for i in 0..num_cats {
		let cat_val = categories_series.as_materialized_series().get(i).unwrap();
		let lit_val = match cat_val {
			AnyValue::String(s) => lit(s),
			AnyValue::Int32(i) => lit(i),
			AnyValue::Int64(i) => lit(i),
			_ => lit(cat_val.to_string()),
		};
		let group_df = df
			.clone()
			.lazy()
			.filter(col(cat_col).eq(lit_val))
			.collect()
			.unwrap();
		let vals = group_df
			.column(val_col)
			.unwrap()
			.cast(&DataType::Float32)
			.unwrap();
		let v = vals.f32().unwrap();
		let mut sorted_v: Vec<f32> = v.into_no_null_iter().collect();
		if sorted_v.is_empty() {
			stats.push(BoxStats {
				min: 0.0,
				q1: 0.0,
				median: 0.0,
				q3: 0.0,
				max: 0.0,
			});
			continue;
		}
		sorted_v.sort_by(|a, b| a.partial_cmp(b).unwrap());
		let n = sorted_v.len();
		let min = sorted_v[0];
		let max = sorted_v[n - 1];
		let q1 = sorted_v[n / 4];
		let median = sorted_v[n / 2];
		let q3 = sorted_v[3 * n / 4];
		stats.push(BoxStats {
			min,
			q1,
			median,
			q3,
			max,
		});
		if min < y_min_all {
			y_min_all = min;
		}
		if max > y_max_all {
			y_max_all = max;
		}
	}
	if y_min_all == f32::MAX {
		y_min_all = 0.0;
		y_max_all = 1.0;
	}
	let pad = (y_max_all - y_min_all).max(0.001) * 0.1;
	let y_range = (y_min_all - pad, y_max_all + pad);
	BoxPlotPreparedData {
		categories,
		stats,
		y_range,
	}
}
