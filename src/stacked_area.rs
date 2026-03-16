use crate::colors;
use crate::plot::{CoordinateTransformer, PlotKernel, PlotLayout};
use iced::advanced::mouse::Cursor;
use iced::widget::canvas::{Frame, Path, Stroke, Style};
use iced::{Color, Rectangle};
use polars::prelude::*;
use rand::RngExt;
use std::collections::HashMap;
use std::sync::Arc;

pub struct StackedAreaPlotKernel {
	pub prepared_data: Arc<StackedAreaPreparedData>,
}

impl PlotKernel for StackedAreaPlotKernel {
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
		let num_cats = self.prepared_data.categories.len();
		let num_xs = self.prepared_data.unique_xs.len();
		if num_xs < 2 { return; }
		let mut prev_stacked_ys = vec![0.0f32; num_xs];
		let mut current_stacked_ys = vec![0.0f32; num_xs];
		for cat_idx in 0..num_cats {
			let t = if num_cats > 1 { cat_idx as f32 / (num_cats - 1) as f32 } else { 0.5 };
			let color = colors::viridis(t);
			for x_idx in 0..num_xs {
				current_stacked_ys[x_idx] = prev_stacked_ys[x_idx] + self.prepared_data.category_values[cat_idx][x_idx];
			}
			let area_path = Path::new(|builder| {
				for (x_idx, csy) in current_stacked_ys.iter().enumerate() {
					let p = transform.cartesian(self.prepared_data.unique_xs[x_idx], *csy);
					if x_idx == 0 {
						builder.move_to(p);
					} else {
						builder.line_to(p);
					}
				}
				for x_idx in (0..num_xs).rev() {
					let p = transform.cartesian(self.prepared_data.unique_xs[x_idx], prev_stacked_ys[x_idx]);
					builder.line_to(p);
				}
				builder.close();
			});
			frame.fill(&area_path, color);
			let stroke = Stroke {
				style: Style::Solid(Color::from_rgba(0.0, 0.0, 0.0, 0.2)),
				width: 0.5,
				..Default::default()
			};
			frame.stroke(&area_path, stroke);
			prev_stacked_ys.copy_from_slice(&current_stacked_ys);
		}
	}

	fn hover(&self, transform: &CoordinateTransformer, cursor: Cursor) -> Option<String> {
		if let Some(cursor_pos) = cursor.position()
			&& let Some((x, y)) = transform.pixel_to_cartesian(cursor_pos) {
			let (x_min, x_max) = self.prepared_data.x_range;
			if x < x_min || x > x_max { return None; }
			let xs = &self.prepared_data.unique_xs;
			if xs.len() < 2 { return None; }
			let idx = match xs.binary_search_by(|val| val.partial_cmp(&x).unwrap()) {
				Ok(i) => i,
				Err(i) => {
					if i == 0 { 0 }
					else if i == xs.len() { xs.len() - 1 }
					else if (xs[i] - x).abs() < (xs[i-1] - x).abs() { i } else { i - 1 }
				}
			};
			let actual_x = xs[idx];
			let mut current_stack_y = 0.0;
			for (j, cat_vals) in self.prepared_data.category_values.iter().enumerate() {
				let val = cat_vals[idx];
				if y >= current_stack_y && y <= current_stack_y + val {
					return Some(format!(
						"X: {:.2}\n{}: {:.2}\nTotal: {:.2}",
						actual_x, self.prepared_data.categories[j], val, current_stack_y + val
					));
				}
				current_stack_y += val;
			}
			return Some(format!("X: {:.2}, Total Sum: {:.2}", actual_x, current_stack_y));
		}
		None
	}

	fn draw_legend(&self, frame: &mut Frame, bounds: Rectangle, settings: crate::plot::LegendSettings) {
		let num_cats = self.prepared_data.categories.len();
		if num_cats == 0 { return; }
		let max_rows = settings.max_rows.max(1) as usize;
		let num_cols = num_cats.div_ceil(max_rows);
		let actual_rows = num_cats.min(max_rows);
		let item_height = 25.0;
		let legend_padding = 10.0;
		let rect_size = 15.0;
		let col_width = 150.0;
		let legend_width = num_cols as f32 * col_width + legend_padding * 2.0;
		let legend_height = actual_rows as f32 * item_height + legend_padding * 2.0;
		let x = bounds.x + (bounds.width - legend_width) * settings.position_x;
		let y = bounds.y + (bounds.height - legend_height) * settings.position_y;
		frame.fill_rectangle(
			iced::Point::new(x, y),
			iced::Size::new(legend_width, legend_height),
			Color::from_rgba(0.0, 0.0, 0.0, 0.6)
		);
		for (i, name) in self.prepared_data.categories.iter().enumerate() {
			let t = if num_cats > 1 { i as f32 / (num_cats - 1) as f32 } else { 0.5 };
			let color = colors::viridis(t);
			let col = i / max_rows;
			let row = i % max_rows;
			let item_x = x + legend_padding + col as f32 * col_width;
			let item_y = y + legend_padding + row as f32 * item_height;
			frame.fill_rectangle(
				iced::Point::new(item_x, item_y + (item_height - rect_size) / 2.0),
				iced::Size::new(rect_size, rect_size),
				color
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
}

pub struct StackedAreaPreparedData {
	pub categories: Vec<String>,
	pub unique_xs: Vec<f32>,
	pub category_values: Vec<Vec<f32>>,
	pub x_range: (f32, f32),
	pub y_range: (f32, f32),
}

pub fn prepare_stacked_area_data(df: &DataFrame, cat_col: &str, x_col: &str, y_col: &str) -> StackedAreaPreparedData {
	let categories_series = df.column(cat_col).unwrap().unique().unwrap().sort(Default::default()).unwrap();
	let categories: Vec<String> = categories_series.as_materialized_series().iter().map(|v| {
		if let AnyValue::String(s) = v { s.to_string() } else { v.to_string().replace("\"", "") }
	}).collect();
	let unique_xs_series = df.column(x_col).unwrap().unique().unwrap().sort(Default::default()).unwrap();
	let unique_xs_f32 = unique_xs_series.cast(&DataType::Float32).unwrap();
	let unique_xs: Vec<f32> = unique_xs_f32.f32().unwrap().into_no_null_iter().collect();
	let num_cats = categories.len();
	let num_xs = unique_xs.len();
	if num_xs < 2 || num_cats == 0 {
		return StackedAreaPreparedData {
			categories,
			unique_xs,
			category_values: Vec::new(),
			x_range: (0.0, 1.0),
			y_range: (0.0, 1.0),
		};
	}
	let aggregated = df.clone().lazy()
		.group_by([col(x_col), col(cat_col)])
		.agg([col(y_col).sum().alias("y_sum")])
		.collect()
		.unwrap();
	let mut category_values = vec![vec![0.0f32; num_xs]; num_cats];
	let cat_to_idx: HashMap<String, usize> = categories.iter().enumerate().map(|(i, s)| (s.clone(), i)).collect();
	let x_to_idx: HashMap<u32, usize> = unique_xs.iter().enumerate().map(|(i, &x)| (x.to_bits(), i)).collect();
	let binding_x = aggregated.column(x_col).unwrap().cast(&DataType::Float32).unwrap();
	let p_x = binding_x.f32().unwrap();
	let p_cat = aggregated.column(cat_col).unwrap();
	let binding_y = aggregated.column("y_sum").unwrap().cast(&DataType::Float32).unwrap();
	let p_y = binding_y.f32().unwrap();
	for i in 0..aggregated.height() {
		let x = p_x.get(i).unwrap();
		let cat_val = p_cat.get(i).unwrap();
		let cat_str = if let AnyValue::String(s) = cat_val { s.to_string() } else { cat_val.to_string().replace("\"", "") };
		let y = p_y.get(i).unwrap();
		if let (Some(&xi), Some(&ci)) = (x_to_idx.get(&x.to_bits()), cat_to_idx.get(&cat_str)) {
			category_values[ci][xi] = y;
		}
	}
	let mut max_sum = 0.0f32;
	for x_idx in 0..num_xs {
		let mut current_sum = 0.0f32;
		for cat_idx in 0..num_cats {
			current_sum += category_values[cat_idx][x_idx];
		}
		if current_sum > max_sum {
			max_sum = current_sum;
		}
	}
	let x_range = (unique_xs[0], unique_xs[num_xs - 1]);
	let y_range = (0.0, max_sum.max(0.001) * 1.05);
	StackedAreaPreparedData {
		categories,
		unique_xs,
		category_values,
		x_range,
		y_range,
	}
}

pub fn generate_sample_stacked_area_data() -> DataFrame {
	let num_cats = 12;
	let num_xs = 200;
	let total_n = num_cats * num_xs;
	let mut cats = Vec::with_capacity(total_n);
	let mut xs = Vec::with_capacity(total_n);
	let mut ys = Vec::with_capacity(total_n);
	let mut rng = rand::rng();
	for i in 0..num_cats {
		let cat = format!("Series {}", i);
		for j in 0..num_xs {
			cats.push(cat.clone());
			xs.push(j as f32);
			let trend = (j as f32 / 20.0).sin() + 2.0;
			let noise: f32 = rng.random_range(0.0..1.0f32);
			ys.push(trend + noise);
		}
	}
	DataFrame::new(total_n, vec![Column::new("cat".into(), cats), Column::new("x".into(), xs), Column::new("y".into(), ys)]).unwrap()
}
