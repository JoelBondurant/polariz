use crate::plot::{CoordinateTransformer, PlotKernel, PlotLayout, PlotSettings};
use iced::advanced::mouse::Cursor;
use iced::widget::canvas::{Frame, Path, Stroke, Style, Text};
use iced::{Color, Pixels, Point, Rectangle};
use polars::prelude::*;
use std::sync::Arc;

pub struct FunnelPlotKernel {
	pub prepared_data: Arc<FunnelPreparedData>,
}

pub struct FunnelPreparedData {
	pub stages: Vec<String>,
	pub values: Vec<f32>,
	pub total_max: f32,
	pub x_label: String,
	pub y_label: String,
}

impl PlotKernel for FunnelPlotKernel {
	fn layout(&self) -> PlotLayout {
		PlotLayout::Radial
	}

	fn plot(
		&self,
		frame: &mut Frame,
		bounds: Rectangle,
		_transform: &CoordinateTransformer,
		_cursor: Cursor,
		settings: crate::plot::PlotSettings,
	) {
		let num_stages = self.prepared_data.stages.len();
		if num_stages == 0 {
			return;
		}
		let stage_height = bounds.height / num_stages as f32;
		let center_x = bounds.x + bounds.width / 2.0;
		for i in 0..num_stages {
			let val = self.prepared_data.values[i];
			let next_val = if i + 1 < num_stages {
				self.prepared_data.values[i + 1]
			} else {
				val
			};
			let width = (val / self.prepared_data.total_max) * bounds.width * 0.8;
			let next_width = (next_val / self.prepared_data.total_max) * bounds.width * 0.8;
			let y_top = bounds.y + i as f32 * stage_height;
			let y_bottom = y_top + stage_height;
			let t = if num_stages > 1 {
				i as f32 / (num_stages - 1) as f32
			} else {
				0.5
			};
			let color = settings.color_theme.get_color(t);
			let trapezoid = Path::new(|builder| {
				builder.move_to(Point::new(center_x - width / 2.0, y_top));
				builder.line_to(Point::new(center_x + width / 2.0, y_top));
				builder.line_to(Point::new(center_x + next_width / 2.0, y_bottom));
				builder.line_to(Point::new(center_x - next_width / 2.0, y_bottom));
				builder.close();
			});
			frame.fill(&trapezoid, color);
			frame.stroke(
				&trapezoid,
				Stroke {
					style: Style::Solid(Color { a: 0.2, ..settings.decoration_color }),
					width: 1.0,
					..Default::default()
				},
			);
			frame.fill_text(Text {
				content: format!("{:.0}", val),
				position: Point::new(center_x, y_top + stage_height / 2.0),
				color: settings.decoration_color,
				size: Pixels(16.0),
				align_x: iced::alignment::Horizontal::Center.into(),
				align_y: iced::alignment::Vertical::Center,
				..Default::default()
			});
		}
	}

	fn draw_legend(&self, frame: &mut Frame, bounds: Rectangle, settings: PlotSettings) {
		let num_cats = self.prepared_data.stages.len();
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
			Point::new(x, y),
			iced::Size::new(legend_width, legend_height),
			Color { a: 0.6, ..settings.background_color },
		);
		for (i, name) in self.prepared_data.stages.iter().enumerate() {
			let t = if num_cats > 1 {
				i as f32 / (num_cats - 1) as f32
			} else {
				0.5
			};
			let color = settings.color_theme.get_color(t);
			let col = i / max_rows;
			let row = i % max_rows;
			let item_x = x + legend_padding + col as f32 * col_width;
			let item_y = y + legend_padding + row as f32 * item_height;
			frame.fill_rectangle(
				Point::new(item_x, item_y + (item_height - rect_size) / 2.0),
				iced::Size::new(rect_size, rect_size),
				color,
			);
			frame.fill_text(Text {
				content: name.clone(),
				position: Point::new(item_x + rect_size + 10.0, item_y + item_height / 2.0),
				color: settings.decoration_color,
				size: Pixels(14.0),
				align_x: iced::alignment::Horizontal::Left.into(),
				align_y: iced::alignment::Vertical::Center,
				..Default::default()
			});
		}
	}

	fn hover(&self, transform: &CoordinateTransformer, cursor: Cursor) -> Option<String> {
		if let Some(cursor_pos) = cursor.position() {
			let bounds = transform.bounds;
			if !bounds.contains(cursor_pos) {
				return None;
			}
			let num_stages = self.prepared_data.stages.len();
			if num_stages == 0 {
				return None;
			}
			let stage_height = bounds.height / num_stages as f32;
			let center_x = bounds.x + bounds.width / 2.0;
			let stage_idx = ((cursor_pos.y - bounds.y) / stage_height).floor() as usize;
			let stage_idx = stage_idx.min(num_stages - 1);
			let val = self.prepared_data.values[stage_idx];
			let next_val = if stage_idx + 1 < num_stages {
				self.prepared_data.values[stage_idx + 1]
			} else {
				val
			};
			let y_within_stage =
				(cursor_pos.y - (bounds.y + stage_idx as f32 * stage_height)) / stage_height;
			let current_val_at_y = val - (val - next_val) * y_within_stage;
			let current_width_at_y =
				(current_val_at_y / self.prepared_data.total_max) * bounds.width * 0.8;
			let left_x = center_x - current_width_at_y / 2.0;
			let right_x = center_x + current_width_at_y / 2.0;
			if cursor_pos.x >= left_x && cursor_pos.x <= right_x {
				let stage_name = &self.prepared_data.stages[stage_idx];
				let mut hover_text = format!("{}: {:.0}", stage_name, val);
				if stage_idx > 0 {
					let prev_val = self.prepared_data.values[stage_idx - 1];
					let conversion = (val / prev_val) * 100.0;
					hover_text.push_str(&format!("\nConversion: {:.1}%", conversion));
				}
				let total_conversion = (val / self.prepared_data.values[0]) * 100.0;
				hover_text.push_str(&format!("\nTotal Rate: {:.1}%", total_conversion));
				return Some(hover_text);
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

pub fn prepare_funnel_data(df: &DataFrame, stage_col: &str, val_col: &str) -> FunnelPreparedData {
	let stages_series = df.column(stage_col).unwrap();
	let values_series = df
		.column(val_col)
		.unwrap()
		.cast(&DataType::Float32)
		.unwrap();
	let stages: Vec<String> = stages_series
		.as_materialized_series()
		.iter()
		.map(|v| {
			if let AnyValue::String(s) = v {
				s.to_string()
			} else {
				v.to_string().replace("\"", "")
			}
		})
		.collect();
	let values: Vec<f32> = values_series.f32().unwrap().into_no_null_iter().collect();
	let total_max = values.iter().cloned().fold(0.0, f32::max);
	FunnelPreparedData {
		stages,
		values,
		total_max,
		x_label: stage_col.to_string(),
		y_label: val_col.to_string(),
	}
}

pub fn generate_sample_funnel_data() -> DataFrame {
	let stages = vec![
		"Website Visits",
		"Downloads",
		"Inquiries",
		"Quotes",
		"Sales",
	];
	let values = vec![1000.0, 600.0, 400.0, 200.0, 100.0];
	DataFrame::new(
		stages.len(),
		vec![
			Column::new("stage".into(), stages),
			Column::new("value".into(), values),
		],
	)
	.unwrap()
}
