use crate::colors;
use crate::plot::{CoordinateTransformer, PlotKernel, PlotLayout, AxisType, TimeUnit};
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
			return Some(format!("X: {}, Y: {}", 
				crate::plot::format_label(x, self.prepared_data.x_axis_type),
				crate::plot::format_label(y, self.prepared_data.y_axis_type)));
		}
		None
	}

	fn draw_legend(&self, frame: &mut Frame, bounds: Rectangle, settings: crate::plot::PlotSettings) {
		let num_series = self.prepared_data.series.len();
		if num_series == 0 { return; }
		let max_rows = settings.max_legend_rows.max(1) as usize;
		let num_cols = num_series.div_ceil(max_rows);
		let actual_rows = num_series.min(max_rows);
		let item_height = 25.0;
		let legend_padding = 10.0;
		let line_width = 20.0;
		let col_width = 150.0;
		let legend_width = num_cols as f32 * col_width + legend_padding * 2.0;
		let legend_height = actual_rows as f32 * item_height + legend_padding * 2.0;
		let x = bounds.x + (bounds.width - legend_width) * settings.legend_x;
		let y = bounds.y + (bounds.height - legend_height) * settings.legend_y;
		frame.fill_rectangle(
			iced::Point::new(x, y),
			iced::Size::new(legend_width, legend_height),
			iced::Color::from_rgba(0.0, 0.0, 0.0, 0.6)
		);
		for (i, series) in self.prepared_data.series.iter().enumerate() {
			let color = colors::viridis(series.color_t);
			let col = i / max_rows;
			let row = i % max_rows;
			let item_x = x + legend_padding + col as f32 * col_width;
			let item_y = y + legend_padding + row as f32 * item_height;
			let stroke = Stroke {
				style: Style::Solid(color),
				width: 3.0,
				..Default::default()
			};
			let line_path = Path::new(|builder| {
				builder.move_to(iced::Point::new(item_x, item_y + item_height / 2.0));
				builder.line_to(iced::Point::new(item_x + line_width, item_y + item_height / 2.0));
			});
			frame.stroke(&line_path, stroke);
			frame.fill_text(iced::widget::canvas::Text {
				content: series.name.clone(),
				position: iced::Point::new(item_x + line_width + 10.0, item_y + item_height / 2.0),
				color: iced::Color::WHITE,
				size: iced::Pixels(14.0),
				align_x: iced::alignment::Horizontal::Left.into(),
				align_y: iced::alignment::Vertical::Center,
				..Default::default()
			});
		}
	}

	fn x_label(&self) -> String {
		self.prepared_data.x_label.clone()
	}

	fn y_label(&self) -> String {
		self.prepared_data.y_label.clone()
	}
}

#[allow(dead_code)]
pub struct SeriesData {
	pub name: String,
	pub points: Vec<[f64; 2]>,
	pub color_t: f32,
}

pub struct LinePreparedData {
	pub series: Vec<SeriesData>,
	pub x_range: (f64, f64),
	pub y_range: (f64, f64),
	pub x_axis_type: AxisType,
	pub y_axis_type: AxisType,
	pub x_label: String,
	pub y_label: String,
}

fn polars_type_to_axis_type(dt: &DataType) -> AxisType {
	match dt {
		DataType::Date => AxisType::Date,
		DataType::Datetime(unit, _) => {
			let tu = match unit {
				polars::prelude::TimeUnit::Nanoseconds => TimeUnit::Nanoseconds,
				polars::prelude::TimeUnit::Microseconds => TimeUnit::Microseconds,
				polars::prelude::TimeUnit::Milliseconds => TimeUnit::Milliseconds,
			};
			AxisType::Datetime(tu)
		}
		DataType::Time => AxisType::Time,
		_ => AxisType::Linear,
	}
}

pub fn prepare_line_data(
	df: &DataFrame,
	cat_col: &str,
	x_col: &str,
	y_col: &str,
) -> LinePreparedData {
	let x_dtype = df.column(x_col).unwrap().dtype();
	let y_dtype = df.column(y_col).unwrap().dtype();
	let x_axis_type = polars_type_to_axis_type(x_dtype);
	let y_axis_type = polars_type_to_axis_type(y_dtype);
	let x_col_series = df.column(x_col).unwrap().cast(&DataType::Float64).unwrap();
	let y_col_series = df.column(y_col).unwrap().cast(&DataType::Float64).unwrap();
	let x_series = x_col_series.f64().unwrap();
	let y_series = y_col_series.f64().unwrap();
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
		let cat_val = group_df.column(cat_col).unwrap().get(0).unwrap();
		let cat_name = if let AnyValue::String(s) = cat_val {
			s.to_string()
		} else {
			cat_val.to_string().replace("\"", "")
		};
		let xs_col = group_df.column(x_col).unwrap().cast(&DataType::Float64).unwrap();
		let ys_col = group_df.column(y_col).unwrap().cast(&DataType::Float64).unwrap();
		let xs = xs_col.f64().unwrap();
		let ys = ys_col.f64().unwrap();
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
		x_axis_type,
		y_axis_type,
		x_label: x_col.to_string(),
		y_label: y_col.to_string(),
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
