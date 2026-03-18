use crate::plot::common::{CoordinateTransformer, PlotKernel, PlotLayout, PlotSettings};
use iced::advanced::mouse::Cursor;
use iced::widget::canvas::{Frame, Path};
use iced::{Color, Point, Rectangle};
use polars::prelude::*;
use std::sync::Arc;

pub struct RadialDialPlotKernel {
	pub prepared_data: Arc<RadialDialPreparedData>,
}

impl PlotKernel for RadialDialPlotKernel {
	fn layout(&self, _settings: PlotSettings) -> PlotLayout {
		PlotLayout::Radial
	}

	fn plot(
		&self,
		frame: &mut Frame,
		bounds: Rectangle,
		_transform: &CoordinateTransformer,
		_cursor: Cursor,
		settings: PlotSettings,
	) {
		let num_cats = self.prepared_data.categories.len();
		if num_cats == 0 {
			return;
		}
		let center = Point::new(
			bounds.x + bounds.width / 2.0,
			bounds.y + bounds.height * 0.8,
		);
		let max_radius = (bounds.width / 2.0).min(bounds.height * 0.7) * 1.0;
		let ring_spacing = 5.0;
		let total_spacing = (num_cats as f32 - 1.0) * ring_spacing;
		let ring_thickness = (max_radius - total_spacing) / num_cats as f32;
		let pi = std::f32::consts::PI;
		let start_angle = pi;
		for i in 0..num_cats {
			let val = self.prepared_data.values[i];
			let max_val = self.prepared_data.max_values[i].max(1e-6);
			let ratio = (val / max_val) as f32;
			let total_sweep = pi;
			let sweep = ratio * total_sweep;
			let t = if num_cats > 1 {
				i as f32 / (num_cats - 1) as f32
			} else {
				0.5
			};
			let color = settings.color_theme.get_color(t);
			let outer_r = max_radius - i as f32 * (ring_thickness + ring_spacing);
			let inner_r = outer_r - ring_thickness;
			let track_path = Path::new(|builder| {
				let steps = 40;
				for step in 0..=steps {
					let angle = start_angle + (total_sweep * step as f32 / steps as f32);
					let p = Point::new(
						center.x + angle.cos() * outer_r,
						center.y + angle.sin() * outer_r,
					);
					if step == 0 {
						builder.move_to(p);
					} else {
						builder.line_to(p);
					}
				}
				for step in (0..=steps).rev() {
					let angle = start_angle + (total_sweep * step as f32 / steps as f32);
					let p = Point::new(
						center.x + angle.cos() * inner_r,
						center.y + angle.sin() * inner_r,
					);
					builder.line_to(p);
				}
				builder.close();
			});
			frame.fill(&track_path, Color::from_rgba(0.5, 0.5, 0.5, 0.1));
			if sweep > 0.001 {
				let ring_path = Path::new(|builder| {
					let steps = (sweep.abs() / total_sweep * 40.0).ceil() as usize;
					let steps = steps.max(2);
					for step in 0..=steps {
						let angle = start_angle + (sweep * step as f32 / steps as f32);
						let p = Point::new(
							center.x + angle.cos() * outer_r,
							center.y + angle.sin() * outer_r,
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
							center.x + angle.cos() * inner_r,
							center.y + angle.sin() * inner_r,
						);
						builder.line_to(p);
					}
					builder.close();
				});
				frame.fill(&ring_path, color);
			}
		}
	}

	fn draw_legend(&self, frame: &mut Frame, bounds: Rectangle, settings: PlotSettings) {
		let num_cats = self.prepared_data.categories.len();
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
			Color {
				a: 0.6,
				..settings.background_color
			},
		);
		for (i, name) in self.prepared_data.categories.iter().enumerate() {
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
				iced::Point::new(item_x, item_y + (item_height - rect_size) / 2.0),
				iced::Size::new(rect_size, rect_size),
				color,
			);
			frame.fill_text(iced::widget::canvas::Text {
				content: name.clone(),
				position: iced::Point::new(item_x + rect_size + 10.0, item_y + item_height / 2.0),
				color: settings.decoration_color,
				size: iced::Pixels(14.0),
				align_x: iced::alignment::Horizontal::Left.into(),
				align_y: iced::alignment::Vertical::Center,
				..Default::default()
			});
		}
	}

	fn hover(&self, transform: &CoordinateTransformer, cursor: Cursor) -> Option<String> {
		if let Some(cursor_pos) = cursor.position() {
			let bounds = transform.bounds;
			let center = Point::new(
				bounds.x + bounds.width / 2.0,
				bounds.y + bounds.height * 0.8,
			);
			let dx = cursor_pos.x - center.x;
			let dy = cursor_pos.y - center.y;
			let dist = (dx * dx + dy * dy).sqrt();
			let num_cats = self.prepared_data.categories.len();
			let max_radius = (bounds.width / 2.0).min(bounds.height * 0.7) * 0.9;
			let ring_spacing = 5.0;
			let ring_thickness =
				(max_radius - (num_cats as f32 - 1.0) * ring_spacing) / num_cats as f32;
			if dy > 0.0 {
				return None;
			}
			for i in 0..num_cats {
				let outer_r = max_radius - i as f32 * (ring_thickness + ring_spacing);
				let inner_r = outer_r - ring_thickness;
				if dist >= inner_r && dist <= outer_r {
					let cat = &self.prepared_data.categories[i];
					let val = self.prepared_data.values[i];
					let max_val = self.prepared_data.max_values[i];
					return Some(format!(
						"{}: {:.1} / {:.1} ({:.0}%)",
						cat,
						val,
						max_val,
						(val / max_val.max(1e-6) * 100.0)
					));
				}
			}
		}
		None
	}
}

pub struct RadialDialPreparedData {
	pub categories: Vec<String>,
	pub values: Vec<f64>,
	pub max_values: Vec<f64>,
}

pub fn prepare_radial_dial_data(
	df: &DataFrame,
	cat_col: &str,
	val_col: &str,
	max_val_col: &str,
) -> RadialDialPreparedData {
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
	let mut max_values = Vec::with_capacity(categories.len());
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
			.collect()
			.unwrap();
		let val = filtered
			.column(val_col)
			.unwrap()
			.cast(&DataType::Float64)
			.unwrap()
			.f64()
			.unwrap()
			.get(0)
			.unwrap_or(0.0);
		let max_v = filtered
			.column(max_val_col)
			.unwrap()
			.cast(&DataType::Float64)
			.unwrap()
			.f64()
			.unwrap()
			.get(0)
			.unwrap_or(1.0);
		values.push(val);
		max_values.push(max_v);
	}
	RadialDialPreparedData {
		categories,
		values,
		max_values,
	}
}

pub fn generate_sample_radial_dial_data() -> DataFrame {
	let cats = ["Steps", "Calories", "Exercise", "Stand"];
	let vals = [404, 1400, 30, 72];
	let max_vals = [2000, 1500, 100, 100];
	DataFrame::new(
		4,
		vec![
			Column::new("cat".into(), cats),
			Column::new("val".into(), vals),
			Column::new("max".into(), max_vals),
		],
	)
	.unwrap()
}
