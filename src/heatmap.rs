use crate::plot::{CoordinateTransformer, PlotKernel, PlotLayout};
use iced::advanced::mouse::Cursor;
use iced::widget::canvas::{Frame, Stroke, Style};
use iced::{Color, Point, Rectangle, Size};
use polars::prelude::*;
use std::sync::Arc;

pub struct HeatmapPlotKernel {
	pub prepared_data: Arc<HeatmapPreparedData>,
}

impl PlotKernel for HeatmapPlotKernel {
	fn layout(&self) -> PlotLayout {
		PlotLayout::CategoricalXY {
			x_categories: self.prepared_data.x_categories.clone(),
			y_categories: self.prepared_data.y_categories.clone(),
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
		let num_x = self.prepared_data.x_categories.len();
		let num_y = self.prepared_data.y_categories.len();
		let max_val = self.prepared_data.max_val;
		for i in 0..num_x {
			for j in 0..num_y {
				let val = self.prepared_data.values[i][j];
				let t = if max_val > 0.0 { (val / max_val) as f32 } else { 0.0 };
				let color = settings.color_theme.get_color(t);
				let (center, bw, bh) = transform.categorical_2d(i, j);
				let rect_x = center.x - bw / 2.0;
				let rect_y = center.y - bh / 2.0;
				frame.fill_rectangle(Point::new(rect_x, rect_y), Size::new(bw, bh), color);
				frame.stroke(
					&iced::widget::canvas::Path::rectangle(
						Point::new(rect_x, rect_y),
						Size::new(bw, bh),
					),
					Stroke {
						style: Style::Solid(Color::from_rgba(0.0, 0.0, 0.0, 0.1)),
						width: 1.0,
						..Default::default()
					},
				);
			}
		}
	}

	fn hover(&self, transform: &CoordinateTransformer, cursor: Cursor) -> Option<String> {
		if let Some(cursor_pos) = cursor.position() {
			let num_x = self.prepared_data.x_categories.len();
			let num_y = self.prepared_data.y_categories.len();
			for i in 0..num_x {
				for j in 0..num_y {
					let (center, bw, bh) = transform.categorical_2d(i, j);
					let rect = Rectangle {
						x: center.x - bw / 2.0,
						y: center.y - bh / 2.0,
						width: bw,
						height: bh,
					};
					if rect.contains(cursor_pos) {
						let x_cat = &self.prepared_data.x_categories[i];
						let y_cat = &self.prepared_data.y_categories[j];
						let val = self.prepared_data.values[i][j];
						return Some(format!("X: {}\nY: {}\nValue: {:.2}", x_cat, y_cat, val));
					}
				}
			}
		}
		None
	}

	fn draw_legend(
		&self,
		frame: &mut Frame,
		bounds: Rectangle,
		settings: crate::plot::PlotSettings,
	) {
		let max_val = self.prepared_data.max_val;
		let legend_width = 60.0;
		let legend_height = 200.0;
		let legend_padding = 10.0;
		let x = bounds.x + (bounds.width - legend_width) * settings.legend_x;
		let y = bounds.y + (bounds.height - legend_height) * settings.legend_y;
		frame.fill_rectangle(
			Point::new(x, y),
			Size::new(legend_width, legend_height),
			Color { a: 0.6, ..settings.background_color },
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
				Point::new(bar_x, step_y),
				iced::Size::new(bar_width, step_height + 0.5),
				color,
			);
		}
		frame.stroke(
			&iced::widget::canvas::Path::rectangle(
				Point::new(bar_x, bar_y),
				Size::new(bar_width, bar_height),
			),
			Stroke {
				style: Style::Solid(settings.decoration_color),
				width: 1.0,
				..Default::default()
			},
		);
		let label_x = bar_x + bar_width + 5.0;
		frame.fill_text(iced::widget::canvas::Text {
			content: format!("{:.1}", max_val),
			position: Point::new(label_x, bar_y),
			color: settings.decoration_color,
			size: iced::Pixels(12.0),
			align_y: iced::alignment::Vertical::Top,
			..Default::default()
		});
		frame.fill_text(iced::widget::canvas::Text {
			content: "0.0".to_string(),
			position: Point::new(label_x, bar_y + bar_height),
			color: settings.decoration_color,
			size: iced::Pixels(12.0),
			align_y: iced::alignment::Vertical::Bottom,
			..Default::default()
		});
	}

	fn x_label(&self) -> String {
		self.prepared_data.x_label.clone()
	}

	fn y_label(&self) -> String {
		self.prepared_data.y_label.clone()
	}
}

pub struct HeatmapPreparedData {
	pub x_categories: Vec<String>,
	pub y_categories: Vec<String>,
	pub values: Vec<Vec<f64>>,
	pub max_val: f64,
	pub x_label: String,
	pub y_label: String,
}

pub fn prepare_heatmap_data(
	df: &DataFrame,
	x_col: &str,
	y_col: &str,
	val_col: &str,
) -> HeatmapPreparedData {
	let x_cats_series = df
		.column(x_col)
		.unwrap()
		.unique()
		.unwrap()
		.sort(Default::default())
		.unwrap();
	let x_categories: Vec<String> = x_cats_series
		.as_materialized_series()
		.iter()
		.map(|v| v.to_string().replace("\"", ""))
		.collect();
	let y_cats_series = df
		.column(y_col)
		.unwrap()
		.unique()
		.unwrap()
		.sort(Default::default())
		.unwrap();
	let y_categories: Vec<String> = y_cats_series
		.as_materialized_series()
		.iter()
		.map(|v| v.to_string().replace("\"", ""))
		.collect();
	let num_x = x_categories.len();
	let num_y = y_categories.len();
	let mut values = vec![vec![0.0f64; num_y]; num_x];
	let mut max_val = 0.0f64;
	let x_to_idx: std::collections::HashMap<String, usize> = x_categories
		.iter()
		.enumerate()
		.map(|(i, s)| (s.clone(), i))
		.collect();
	let y_to_idx: std::collections::HashMap<String, usize> = y_categories
		.iter()
		.enumerate()
		.map(|(i, s)| (s.clone(), i))
		.collect();
	let binding_val = df
		.column(val_col)
		.unwrap()
		.cast(&DataType::Float64)
		.unwrap();
	let p_val = binding_val.f64().unwrap();
	let p_x = df.column(x_col).unwrap();
	let p_y = df.column(y_col).unwrap();
	for i in 0..df.height() {
		let x_v = p_x.get(i).unwrap().to_string().replace("\"", "");
		let y_v = p_y.get(i).unwrap().to_string().replace("\"", "");
		let val = p_val.get(i).unwrap_or(0.0);
		if let (Some(&xi), Some(&yi)) = (x_to_idx.get(&x_v), y_to_idx.get(&y_v)) {
			values[xi][yi] = val;
			if val > max_val {
				max_val = val;
			}
		}
	}
	HeatmapPreparedData {
		x_categories,
		y_categories,
		values,
		max_val,
		x_label: x_col.to_string(),
		y_label: y_col.to_string(),
	}
}

pub fn generate_sample_heatmap_data() -> DataFrame {
	let num_x = 20;
	let num_y = 20;
	let total_n = num_x * num_y;
	let mut xs = Vec::with_capacity(total_n);
	let mut ys = Vec::with_capacity(total_n);
	let mut vals = Vec::with_capacity(total_n);
	for i in 0..num_x {
		let x_label = format!("X{:02}", i + 1);
		for j in 0..num_y {
			let y_label = format!("Y{:02}", j + 1);
			xs.push(x_label.clone());
			ys.push(y_label);
			let val = ((i as f32 / 5.0).cos() + (j as f32 / 3.0).sin()).abs() * 10.0;
			vals.push(val as f64);
		}
	}
	DataFrame::new(
		total_n,
		vec![
			Column::new("x".into(), xs),
			Column::new("y".into(), ys),
			Column::new("val".into(), vals),
		],
	)
	.unwrap()
}
