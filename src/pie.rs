use crate::colors;
use crate::plot::{CoordinateTransformer, PlotKernel, PlotLayout};
use iced::advanced::mouse::Cursor;
use iced::widget::canvas::{Frame, Path, Stroke, Style};
use iced::{Color, Point, Rectangle};
use polars::prelude::*;
use rand::RngExt;
use std::sync::Arc;

pub struct PiePlotKernel {
	pub prepared_data: Arc<PiePreparedData>,
}

impl PlotKernel for PiePlotKernel {
	fn layout(&self) -> PlotLayout {
		PlotLayout::Radial
	}

	fn plot(
		&self,
		frame: &mut Frame,
		_bounds: Rectangle,
		transform: &CoordinateTransformer,
		_cursor: Cursor,
	) {
		let center = transform.bounds.center();
		let radius = transform.bounds.width.min(transform.bounds.height) * 0.50;
		let inner_radius = radius * 0.02;
		let num_sectors = self.prepared_data.categories.len();
		let pi = std::f32::consts::PI;
		let mut start_angle = -pi / 2.0;
		for i in 0..num_sectors {
			let val = self.prepared_data.values[i];
			let sweep = (val / self.prepared_data.total_sum) * 2.0 * pi;
			let end_angle = start_angle + sweep;
			let t = if num_sectors > 1 {
				i as f32 / (num_sectors - 1) as f32
			} else {
				0.5
			};
			let color = colors::viridis(t);
			let wedge_path = Path::new(|builder| {
				let steps = 40;
				for step in 0..=steps {
					let angle = start_angle + (sweep * step as f32 / steps as f32);
					let p = Point::new(
						center.x + angle.cos() * radius,
						center.y + angle.sin() * radius,
					);
					if step == 0 {
						builder.move_to(p);
					} else {
						builder.line_to(p);
					}
				}
				for step in (0..=steps).rev() {
					let angle = start_angle + (sweep * step as f32 / steps as f32);
					let p = Point::new(
						center.x + angle.cos() * inner_radius,
						center.y + angle.sin() * inner_radius,
					);
					builder.line_to(p);
				}
				builder.close();
			});
			frame.fill(&wedge_path, color);
			let border_stroke = Stroke {
				style: Style::Solid(Color::from_rgba(1.0, 1.0, 1.0, 0.3)),
				width: 1.0,
				..Default::default()
			};
			frame.stroke(&wedge_path, border_stroke);
			start_angle = end_angle;
		}
	}

	fn hover(&self, transform: &CoordinateTransformer, cursor: Cursor) -> Option<String> {
		if let Some(cursor_pos) = cursor.position() {
			let center = transform.bounds.center();
			let dx = cursor_pos.x - center.x;
			let dy = cursor_pos.y - center.y;
			let dist = (dx * dx + dy * dy).sqrt();
			let radius = transform.bounds.width.min(transform.bounds.height) * 0.45;
			let inner_radius = radius * 0.05;
			if dist >= inner_radius && dist <= radius {
				let pi = std::f32::consts::PI;
				let angle = dy.atan2(dx); // atan2(y, x)
				let mut normalized_angle = angle - (-pi / 2.0);
				while normalized_angle < 0.0 {
					normalized_angle += 2.0 * pi;
				}
				while normalized_angle >= 2.0 * pi {
					normalized_angle -= 2.0 * pi;
				}
				let angle_ratio = normalized_angle / (2.0 * pi);
				for (i, &limit) in self.prepared_data.cumulative_angles.iter().enumerate() {
					if angle_ratio < limit {
						let cat = &self.prepared_data.categories[i];
						let val = self.prepared_data.values[i];
						return Some(format!(
							"{}: {:.2} ({:.1}%)",
							cat,
							val,
							val / self.prepared_data.total_sum * 100.0
						));
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
		settings: crate::plot::LegendSettings,
	) {
		let num_cats = self.prepared_data.categories.len();
		if num_cats == 0 {
			return;
		}
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
			Color::from_rgba(0.0, 0.0, 0.0, 0.6),
		);
		for (i, name) in self.prepared_data.categories.iter().enumerate() {
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
}

pub struct PiePreparedData {
	pub categories: Vec<String>,
	pub values: Vec<f32>,
	pub cumulative_angles: Vec<f32>,
	pub total_sum: f32,
}

pub fn prepare_pie_data(df: &DataFrame, cat_col: &str, val_col: &str) -> PiePreparedData {
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
				v.to_string().replace("\"", "")
			}
		})
		.collect();
	let mut values = Vec::with_capacity(categories.len());
	let mut total_sum = 0.0f32;
	for cat_val in categories_series.as_materialized_series().iter() {
		let lit_val = match cat_val {
			AnyValue::String(s) => lit(s),
			AnyValue::Int32(i) => lit(i),
			AnyValue::Int64(i) => lit(i),
			_ => lit(cat_val.to_string()),
		};
		let filtered = df
			.clone()
			.lazy()
			.filter(col(cat_col).eq(lit_val))
			.select([col(val_col).sum()])
			.collect()
			.unwrap();
		let val = filtered
			.column(val_col)
			.unwrap()
			.cast(&DataType::Float32)
			.unwrap()
			.f32()
			.unwrap()
			.get(0)
			.unwrap_or(0.0);
		values.push(val);
		total_sum += val;
	}
	let mut cumulative_angles = Vec::with_capacity(values.len());
	let mut current_sum = 0.0f32;
	for &val in &values {
		current_sum += val;
		cumulative_angles.push(current_sum / total_sum);
	}
	PiePreparedData {
		categories,
		values,
		cumulative_angles,
		total_sum,
	}
}

pub fn generate_sample_pie_data() -> DataFrame {
	let num_cats = 6;
	let mut cats = Vec::with_capacity(num_cats);
	let mut vals = Vec::with_capacity(num_cats);
	let mut rng = rand::rng();
	for i in 0..num_cats {
		cats.push(format!("Category {}", i + 1));
		vals.push(rng.random_range(10.0..100.0f32));
	}
	DataFrame::new(
		num_cats,
		vec![
			Column::new("cat".into(), cats),
			Column::new("val".into(), vals),
		],
	)
	.unwrap()
}
