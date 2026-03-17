use crate::colors;
use crate::plot::{CoordinateTransformer, PlotKernel, PlotLayout};
use iced::advanced::mouse::Cursor;
use iced::widget::canvas::{Frame, Path, Stroke, Style};
use iced::{Color, Rectangle};
use polars::prelude::*;
use rand::RngExt;
use std::sync::Arc;

pub struct ParallelPlotKernel {
	pub prepared_data: Arc<ParallelPreparedData>,
}

impl PlotKernel for ParallelPlotKernel {
	fn layout(&self) -> PlotLayout {
		PlotLayout::Parallel {
			dimensions: self.prepared_data.dimensions.clone(),
			ranges: self.prepared_data.ranges.clone(),
		}
	}

	fn plot(
		&self,
		frame: &mut Frame,
		_bounds: Rectangle,
		transform: &CoordinateTransformer,
		_cursor: Cursor,
	) {
		let num_dims = self.prepared_data.dimensions.len();
		if num_dims < 2 {
			return;
		}
		for (i, row_data) in self.prepared_data.data_matrix.iter().enumerate() {
			let cat_idx = self.prepared_data.row_categories[i];
			let t = if self.prepared_data.num_categories > 1 {
				cat_idx as f32 / (self.prepared_data.num_categories - 1) as f32
			} else {
				0.5
			};
			let color = colors::viridis(t);
			let stroke = Stroke {
				style: Style::Solid(color),
				width: 1.5,
				..Default::default()
			};
			let path = Path::new(|builder| {
				for (d, &val) in row_data.iter().enumerate() {
					let (p, _) = transform.categorical(d, val);
					if d == 0 {
						builder.move_to(p);
					} else {
						builder.line_to(p);
					}
				}
			});
			frame.stroke(&path, stroke);
		}
	}

	fn draw_legend(
		&self,
		frame: &mut Frame,
		bounds: Rectangle,
		settings: crate::plot::PlotSettings,
	) {
		let num_cats = self.prepared_data.category_names.len();
		if num_cats == 0 {
			return;
		}
		let max_rows = settings.max_legend_rows.max(1) as usize;
		let num_cols = num_cats.div_ceil(max_rows);
		let actual_rows = num_cats.min(max_rows);
		let item_height = 25.0;
		let legend_padding = 10.0;
		let rect_size = 15.0;
		let col_width = 150.0;
		let legend_width = num_cols as f32 * col_width + legend_padding * 2.0;
		let legend_height = actual_rows as f32 * item_height + legend_padding * 2.0;
		let x = bounds.x + (bounds.width - legend_width) * settings.legend_x;
		let y = bounds.y + (bounds.height - legend_height) * settings.legend_y;
		frame.fill_rectangle(
			iced::Point::new(x, y),
			iced::Size::new(legend_width, legend_height),
			Color::from_rgba(0.0, 0.0, 0.0, 0.6),
		);
		for (i, name) in self.prepared_data.category_names.iter().enumerate() {
			let t = if num_cats > 1 {
				i as f32 / (num_cats - 1) as f32
			} else {
				0.5
			};
			let color = colors::viridis(t);
			let col = i / max_rows;
			let row = i % max_rows;
			let item_x = x + legend_padding + col as f32 * col_width;
			let item_y = y + legend_padding + row as f32 * item_height;
			frame.fill_rectangle(
				iced::Point::new(item_x, item_y + (item_height - rect_size) / 2.0),
				iced::Size::new(rect_size, rect_size),
				color,
			);
			frame.fill_text(iced::widget::canvas::Text {
				content: name.clone(),
				position: iced::Point::new(item_x + rect_size + 10.0, item_y + item_height / 2.0),
				color: Color::WHITE,
				size: iced::Pixels(14.0),
				align_x: iced::alignment::Horizontal::Left.into(),
				align_y: iced::alignment::Vertical::Center,
				..Default::default()
			});
		}
	}

	fn hover(&self, transform: &CoordinateTransformer, cursor: Cursor) -> Option<String> {
		if let Some(cursor_pos) = cursor.position() {
			let num_dims = self.prepared_data.dimensions.len();
			if num_dims < 2 {
				return None;
			}
			let spacing = transform.bounds.width / (num_dims - 1) as f32;
			let axis_idx =
				((cursor_pos.x - transform.bounds.x + spacing / 2.0) / spacing).floor() as i32;
			if axis_idx >= 0 && (axis_idx as usize) < num_dims {
				let i = axis_idx as usize;
				let range = self.prepared_data.ranges[i];
				let y_scale = (range.1 - range.0) / transform.bounds.height as f64;
				let val = range.1 - (cursor_pos.y - transform.bounds.y) as f64 * y_scale;
				return Some(format!("{}: {:.2}", self.prepared_data.dimensions[i], val));
			}
		}
		None
	}

	fn x_label(&self) -> String {
		"Dimension".to_string()
	}

	fn y_label(&self) -> String {
		"Value".to_string()
	}
}

pub struct ParallelPreparedData {
	pub dimensions: Vec<String>,
	pub ranges: Vec<(f64, f64)>,
	pub data_matrix: Vec<Vec<f64>>,
	pub row_categories: Vec<usize>,
	pub category_names: Vec<String>,
	pub num_categories: usize,
}

pub fn prepare_parallel_data(
	df: &DataFrame,
	dims: &[String],
	cat_col: &str,
) -> ParallelPreparedData {
	let num_dims = dims.len();
	let mut ranges = Vec::with_capacity(num_dims);
	let mut dim_columns = Vec::with_capacity(num_dims);
	for dim in dims {
		let col = df.column(dim).unwrap().cast(&DataType::Float64).unwrap();
		let series = col.f64().unwrap().clone();
		let min = series.min().unwrap_or(0.0);
		let max = series.max().unwrap_or(1.0);
		ranges.push((min, max));
		dim_columns.push(series);
	}
	let categories_series = df
		.column(cat_col)
		.unwrap()
		.unique()
		.unwrap()
		.sort(Default::default())
		.unwrap();
	let categories_mat = categories_series.as_materialized_series();
	let num_cats = categories_mat.len();
	let mut category_names = Vec::with_capacity(num_cats);
	let mut cat_to_idx = std::collections::HashMap::new();
	for (i, v) in categories_mat.iter().enumerate() {
		let s = if let AnyValue::String(s) = v {
			s.to_string()
		} else {
			v.to_string().replace("\"", "")
		};
		category_names.push(s.clone());
		cat_to_idx.insert(s, i);
	}
	let cat_vals = df.column(cat_col).unwrap();
	let num_rows = df.height();
	let mut data_matrix = Vec::with_capacity(num_rows);
	let mut row_categories = Vec::with_capacity(num_rows);
	for i in 0..num_rows {
		let mut row = Vec::with_capacity(num_dims);
		for dim_col in dim_columns.iter() {
			row.push(dim_col.get(i).unwrap());
		}
		data_matrix.push(row);
		let cat_val_raw = cat_vals.get(i).unwrap();
		let cat_val_str = if let AnyValue::String(s) = cat_val_raw {
			s.to_string()
		} else {
			cat_val_raw.to_string().replace("\"", "")
		};
		row_categories.push(*cat_to_idx.get(&cat_val_str).unwrap_or(&0));
	}
	ParallelPreparedData {
		dimensions: dims.to_vec(),
		ranges,
		data_matrix,
		row_categories,
		category_names,
		num_categories: num_cats,
	}
}

pub fn generate_sample_parallel_data() -> DataFrame {
	let n = 200;
	let mut rng = rand::rng();
	let mut cats = Vec::with_capacity(n);
	let mut d1 = Vec::with_capacity(n);
	let mut d2 = Vec::with_capacity(n);
	let mut d3 = Vec::with_capacity(n);
	let mut d4 = Vec::with_capacity(n);
	for i in 0..n {
		let group = i % 3;
		cats.push(format!("Group {}", group));
		match group {
			0 => {
				d1.push(rng.random_range(0.0..1.0f64));
				d2.push(rng.random_range(5.0..6.0f64));
				d3.push(rng.random_range(2.0..3.0f64));
				d4.push(rng.random_range(8.0..9.0f64));
			}
			1 => {
				d1.push(rng.random_range(5.0..6.0f64));
				d2.push(rng.random_range(0.0..2.0f64));
				d3.push(rng.random_range(6.0..8.0f64));
				d4.push(rng.random_range(7.0..7.3f64));
			}
			_ => {
				d1.push(rng.random_range(3.0..3.2f64));
				d2.push(rng.random_range(3.0..3.5f64));
				d3.push(rng.random_range(6.0..6.8f64));
				d4.push(rng.random_range(9.0..9.2f64));
			}
		}
	}
	DataFrame::new(
		n,
		vec![
			Column::new("cat".into(), cats),
			Column::new("Dim A".into(), d1),
			Column::new("Dim B".into(), d2),
			Column::new("Dim C".into(), d3),
			Column::new("Dim D".into(), d4),
		],
	)
	.unwrap()
}
