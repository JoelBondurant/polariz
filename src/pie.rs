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
		let radius = transform.bounds.width.min(transform.bounds.height) * 0.35;
		let inner_radius = radius * 0.05;
		let num_sectors = self.prepared_data.categories.len();
		let pi = std::f32::consts::PI;
		let mut start_angle = -pi / 2.0; // Start at 12 o'clock
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
			let radius = transform.bounds.width.min(transform.bounds.height) * 0.35;
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
				v.to_string()
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
