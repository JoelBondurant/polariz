use crate::colors;
use crate::plot::{CoordinateTransformer, PlotKernel, PlotLayout};
use iced::advanced::mouse::Cursor;
use iced::widget::canvas::{Frame, Path, Stroke, Style};
use iced::Rectangle;
use polars::prelude::*;
use rand::RngExt;
use std::sync::Arc;

pub struct LinePlotKernel {
	pub prepared_data: Arc<LinePreparedData>,
}

impl PlotKernel for LinePlotKernel {
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
		for series in &self.prepared_data.series {
			let color = colors::viridis(series.color_t);
			let stroke = Stroke {
				style: Style::Solid(color),
				width: 2.0,
				..Default::default()
			};
			let path = Path::new(|builder| {
				for (i, p) in series.points.iter().enumerate() {
					let pixel_p = transform.cartesian(p[0], p[1]);
					if i == 0 {
						builder.move_to(pixel_p);
					} else {
						builder.line_to(pixel_p);
					}
				}
			});
			frame.stroke(&path, stroke);
		}
	}

	fn hover(&self, transform: &CoordinateTransformer, cursor: Cursor) -> Option<String> {
		if let Some(cursor_pos) = cursor.position()
			&& let Some((x, y)) = transform.pixel_to_cartesian(cursor_pos) {
			return Some(format!("X: {:.2}, Y: {:.2}", x, y));
		}
		None
	}
}

#[allow(dead_code)]
pub struct SeriesData {
	pub name: String,
	pub points: Vec<[f32; 2]>,
	pub color_t: f32,
}

pub struct LinePreparedData {
	pub series: Vec<SeriesData>,
	pub x_range: (f32, f32),
	pub y_range: (f32, f32),
}

pub fn prepare_line_data(
	df: &DataFrame,
	cat_col: &str,
	x_col: &str,
	y_col: &str,
) -> LinePreparedData {
	let x_col_series = df.column(x_col).unwrap().cast(&DataType::Float32).unwrap();
	let y_col_series = df.column(y_col).unwrap().cast(&DataType::Float32).unwrap();
	let x_series = x_col_series.f32().unwrap();
	let y_series = y_col_series.f32().unwrap();
	let x_range = (x_series.min().unwrap_or(0.0), x_series.max().unwrap_or(1.0));
	let y_series_min = y_series.min().unwrap_or(0.0);
	let y_series_max = y_series.max().unwrap_or(1.0);
	let y_range = (y_series_min, y_series_max);
	let x_pad = (x_range.1 - x_range.0) * 0.05;
	let y_pad = (y_range.1 - y_range.0) * 0.05;
	let x_range = (x_range.0 - x_pad, x_range.1 + x_pad);
	let y_range = (y_range.0 - y_pad, y_range.1 + y_pad);
	let mut series_list = Vec::new();
	let partitions = df.partition_by([cat_col], true).unwrap();
	let num_partitions = partitions.len();
	for (i, group_df) in partitions.into_iter().enumerate() {
		let cat_name = group_df.column(cat_col).unwrap().get(0).unwrap().to_string();
		let xs_col = group_df.column(x_col).unwrap().cast(&DataType::Float32).unwrap();
		let ys_col = group_df.column(y_col).unwrap().cast(&DataType::Float32).unwrap();
		let xs = xs_col.f32().unwrap();
		let ys = ys_col.f32().unwrap();
		let t = if num_partitions > 1 {
			i as f32 / (num_partitions - 1) as f32
		} else {
			0.5
		};
		let mut points = Vec::with_capacity(group_df.height());
		for j in 0..group_df.height() {
			points.push([xs.get(j).unwrap(), ys.get(j).unwrap()]);
		}
		series_list.push(SeriesData {
			name: cat_name,
			points,
			color_t: t,
		});
	}
	LinePreparedData {
		series: series_list,
		x_range,
		y_range,
	}
}

pub fn generate_sample_line_data() -> DataFrame {
	let num_series = 5;
	let n_per_series = 10000;
	let total_n = num_series * n_per_series;
	let mut rng = rand::rng();
	let mut cats = Vec::with_capacity(total_n);
	let mut xs = Vec::with_capacity(total_n);
	let mut ys = Vec::with_capacity(total_n);
	for i in 0..num_series {
		let cat = format!("Series {}", i);
		let mut current_y: f32 = rng.random_range(0.0..10.0f32);
		for j in 0..n_per_series {
			cats.push(cat.clone());
			xs.push(j as f32);
			current_y += rng.random_range(-1.0..1.0f32);
			ys.push(current_y);
		}
	}
	DataFrame::new(
		total_n,
		vec![
			Column::new("cat".into(), cats),
			Column::new("x".into(), xs),
			Column::new("y".into(), ys),
		],
	)
	.unwrap()
}
