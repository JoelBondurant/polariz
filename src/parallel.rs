use crate::colors;
use crate::plot::{CoordinateTransformer, PlotKernel, PlotLayout};
use iced::advanced::mouse::Cursor;
use iced::widget::canvas::{Frame, Path, Stroke, Style};
use iced::Rectangle;
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
				for (d, val) in row_data.iter().enumerate() {
					let (p, _) = transform.categorical(d, *val);
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
				let y_scale = (range.1 - range.0) / transform.bounds.height;
				let val = range.1 - (cursor_pos.y - transform.bounds.y) * y_scale;
				return Some(format!("{}: {:.2}", self.prepared_data.dimensions[i], val));
			}
		}
		None
	}
}

pub struct ParallelPreparedData {
	pub dimensions: Vec<String>,
	pub ranges: Vec<(f32, f32)>,
	pub data_matrix: Vec<Vec<f32>>,
	pub row_categories: Vec<usize>,
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
		let col = df.column(dim).unwrap().cast(&DataType::Float32).unwrap();
		let series = col.f32().unwrap().clone();
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
	let mut cat_to_idx = std::collections::HashMap::new();
	for (i, v) in categories_mat.iter().enumerate() {
		cat_to_idx.insert(v.to_string(), i);
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

		let cat_val = cat_vals.get(i).unwrap().to_string();
		row_categories.push(cat_to_idx.get(&cat_val).copied().unwrap_or(0));
	}
	ParallelPreparedData {
		dimensions: dims.to_vec(),
		ranges,
		data_matrix,
		row_categories,
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
				d1.push(rng.random_range(0.0..1.0f32));
				d2.push(rng.random_range(5.0..6.0f32));
				d3.push(rng.random_range(2.0..3.0f32));
				d4.push(rng.random_range(8.0..9.0f32));
			}
			1 => {
				d1.push(rng.random_range(5.0..6.0f32));
				d2.push(rng.random_range(0.0..2.0f32));
				d3.push(rng.random_range(6.0..8.0f32));
				d4.push(rng.random_range(7.0..7.3f32));
			}
			_ => {
				d1.push(rng.random_range(3.0..3.2f32));
				d2.push(rng.random_range(3.0..3.5f32));
				d3.push(rng.random_range(6.0..6.8f32));
				d4.push(rng.random_range(9.0..9.2f32));
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
