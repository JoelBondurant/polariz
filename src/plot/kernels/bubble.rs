use crate::plot::common::{
	format_label, polars_type_to_axis_type, AxisType, CoordinateTransformer, PlotKernel,
	PlotLayout, PlotSettings,
};
use iced::advanced::mouse::Cursor;
use iced::widget::canvas::{Frame, Path, Stroke, Style, Text};
use iced::{Color, Pixels, Point, Rectangle};
use polars::prelude::*;
use rand::RngExt;
use std::sync::Arc;

pub struct BubblePlotKernel {
	pub prepared_data: Arc<BubblePreparedData>,
}

pub struct BubblePreparedData {
	pub points: Vec<BubblePoint>,
	pub x_range: (f64, f64),
	pub y_range: (f64, f64),
	pub x_axis_type: AxisType,
	pub y_axis_type: AxisType,
	pub size_range: (f64, f64),
	pub color_range: (f64, f64),
	pub x_label: String,
	pub y_label: String,
	pub size_label: String,
	pub color_label: String,
}

pub struct BubblePoint {
	pub x: f64,
	pub y: f64,
	pub size_val: f64,
	pub color_val: f64,
	pub label: String,
}

impl PlotKernel for BubblePlotKernel {
	fn layout(&self, settings: PlotSettings) -> PlotLayout {
		PlotLayout::Cartesian {
			x_range: (
				settings.x_min.unwrap_or(self.prepared_data.x_range.0),
				settings.x_max.unwrap_or(self.prepared_data.x_range.1),
			),
			y_range: (
				settings.y_min.unwrap_or(self.prepared_data.y_range.0),
				settings.y_max.unwrap_or(self.prepared_data.y_range.1),
			),
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
		settings: PlotSettings,
	) {
		let color_min = self.prepared_data.color_range.0;
		let color_max = self.prepared_data.color_range.1;
		let color_delta = (color_max - color_min).max(f64::EPSILON);
		let size_min = self.prepared_data.size_range.0;
		let size_max = self.prepared_data.size_range.1;
		let size_delta = (size_max - size_min).max(f64::EPSILON);
		for point in &self.prepared_data.points {
			let p = transform.cartesian(point.x, point.y);
			let t_color = ((point.color_val - color_min) / color_delta) as f32;
			let color = settings.color_theme.get_color(t_color);
			let t_size = ((point.size_val - size_min) / size_delta) as f32;
			let radius = 2.0 + t_size * 28.0;
			let circle = Path::circle(p, radius);
			frame.fill(&circle, Color { a: 0.7, ..color });
			frame.stroke(
				&circle,
				Stroke {
					style: Style::Solid(Color { a: 0.9, ..color }),
					width: 1.0,
					..Default::default()
				},
			);
		}
	}

	fn draw_legend(&self, frame: &mut Frame, bounds: Rectangle, settings: PlotSettings) {
		let color_min = self.prepared_data.color_range.0;
		let color_max = self.prepared_data.color_range.1;
		let size_min = self.prepared_data.size_range.0;
		let size_max = self.prepared_data.size_range.1;
		let legend_width = 250.0;
		let legend_height = 240.0;
		let legend_padding = 20.0;
		let x = bounds.x + (bounds.width - legend_width) * settings.legend_x;
		let y = bounds.y + (bounds.height - legend_height) * settings.legend_y;
		frame.fill_rectangle(
			Point::new(x, y),
			iced::Size::new(legend_width, legend_height),
			Color {
				a: 0.7,
				..settings.background_color
			},
		);
		let bar_width = 15.0;
		let bar_height = legend_height - 80.0;
		let color_bar_x = x + legend_width - 70.0;
		let bar_y = y + 50.0;
		let steps = 50;
		for i in 0..steps {
			let t = i as f32 / (steps - 1) as f32;
			let color = settings.color_theme.get_color(t);
			let step_height = bar_height / steps as f32;
			let step_y = bar_y + bar_height - (i as f32 + 1.0) * step_height;
			frame.fill_rectangle(
				Point::new(color_bar_x, step_y),
				iced::Size::new(bar_width, step_height + 0.5),
				color,
			);
		}
		let color_label_x = color_bar_x + bar_width + 8.0;
		frame.fill_text(Text {
			content: format!("{:.1}", color_max),
			position: Point::new(color_label_x, bar_y),
			color: settings.decoration_color,
			size: Pixels(settings.legend_size),
			align_y: iced::alignment::Vertical::Top,
			..Default::default()
		});
		frame.fill_text(Text {
			content: format!("{:.1}", color_min),
			position: Point::new(color_label_x, bar_y + bar_height),
			color: settings.decoration_color,
			size: Pixels(settings.legend_size),
			align_y: iced::alignment::Vertical::Bottom,
			..Default::default()
		});
		frame.fill_text(Text {
			content: self.prepared_data.color_label.clone(),
			position: Point::new(color_bar_x + bar_width / 2.0, y + 15.0),
			color: settings.decoration_color,
			size: Pixels(16.0),
			align_x: iced::alignment::Horizontal::Center.into(),
			align_y: iced::alignment::Vertical::Top,
			..Default::default()
		});
		let size_legend_x = x + legend_padding;
		let size_title_x = size_legend_x + 40.0;
		frame.fill_text(Text {
			content: self.prepared_data.size_label.clone(),
			position: Point::new(size_title_x, y + 15.0),
			color: settings.decoration_color,
			size: Pixels(16.0),
			align_x: iced::alignment::Horizontal::Center.into(),
			align_y: iced::alignment::Vertical::Top,
			..Default::default()
		});
		let num_samples = 3;
		let max_radius = 30.0;
		let min_radius = 2.0;
		for i in 0..num_samples {
			let t = i as f32 / (num_samples - 1) as f32;
			let val = size_min + (size_max - size_min) * t as f64;
			let radius = min_radius + t * (max_radius - min_radius);
			let sample_y =
				(bar_y + bar_height - min_radius) - t * (bar_height - max_radius - min_radius);
			let circle_center = Point::new(size_legend_x + 35.0, sample_y);
			let circle = Path::circle(circle_center, radius);
			frame.fill(
				&circle,
				Color {
					a: 0.5,
					..settings.decoration_color
				},
			);
			frame.stroke(
				&circle,
				Stroke {
					style: Style::Solid(settings.decoration_color),
					width: 1.0,
					..Default::default()
				},
			);
			frame.fill_text(Text {
				content: format!("{:.1}", val),
				position: Point::new(size_legend_x + 75.0, sample_y),
				color: settings.decoration_color,
				size: Pixels(settings.legend_size),
				align_y: iced::alignment::Vertical::Center,
				..Default::default()
			});
		}
	}

	fn hover(&self, transform: &CoordinateTransformer, cursor: Cursor) -> Option<String> {
		if let Some(cursor_pos) = cursor.position() {
			let size_min = self.prepared_data.size_range.0;
			let size_max = self.prepared_data.size_range.1;
			let size_delta = (size_max - size_min).max(f64::EPSILON);
			for point in &self.prepared_data.points {
				let p = transform.cartesian(point.x, point.y);
				let dx = cursor_pos.x - p.x;
				let dy = cursor_pos.y - p.y;
				let dist_sq = dx * dx + dy * dy;
				let t_size = ((point.size_val - size_min) / size_delta) as f32;
				let radius = 2.0 + t_size * 28.0;
				if dist_sq <= radius * radius {
					return Some(format!(
						"{}\n{}: {}\n{}: {:.2}\n{}: {:.2}\n{}: {:.2}",
						point.label,
						self.prepared_data.x_label,
						format_label(point.x, self.prepared_data.x_axis_type),
						self.prepared_data.y_label,
						point.y,
						self.prepared_data.size_label,
						point.size_val,
						self.prepared_data.color_label,
						point.color_val
					));
				}
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

pub fn prepare_bubble_data(
	df: &DataFrame,
	x_col: &str,
	y_col: &str,
	size_col: &str,
	color_col: &str,
	label_col: Option<&str>,
) -> BubblePreparedData {
	let x_dtype = df.column(x_col).unwrap().dtype();
	let y_dtype = df.column(y_col).unwrap().dtype();
	let x_axis_type = polars_type_to_axis_type(x_dtype);
	let y_axis_type = polars_type_to_axis_type(y_dtype);

	let xs = df.column(x_col).unwrap().cast(&DataType::Float64).unwrap();
	let ys = df.column(y_col).unwrap().cast(&DataType::Float64).unwrap();
	let sizes = df
		.column(size_col)
		.unwrap()
		.cast(&DataType::Float64)
		.unwrap();
	let colors_col = df
		.column(color_col)
		.unwrap()
		.cast(&DataType::Float64)
		.unwrap();
	let x_f64 = xs.f64().unwrap();
	let y_f64 = ys.f64().unwrap();
	let size_f64 = sizes.f64().unwrap();
	let color_f64 = colors_col.f64().unwrap();
	let x_range = (x_f64.min().unwrap_or(0.0), x_f64.max().unwrap_or(1.0));
	let y_range = (y_f64.min().unwrap_or(0.0), y_f64.max().unwrap_or(1.0));
	let size_range = (size_f64.min().unwrap_or(0.0), size_f64.max().unwrap_or(1.0));
	let color_range = (
		color_f64.min().unwrap_or(0.0),
		color_f64.max().unwrap_or(1.0),
	);
	let x_pad = (x_range.1 - x_range.0).max(0.1) * 0.1;
	let y_pad = (y_range.1 - y_range.0).max(0.1) * 0.1;
	let labels: Vec<String> = if let Some(l_col) = label_col {
		df.column(l_col)
			.unwrap()
			.as_materialized_series()
			.iter()
			.map(|v| {
				if let AnyValue::String(s) = v {
					s.to_string()
				} else {
					v.to_string().replace("\"", "")
				}
			})
			.collect()
	} else {
		(0..df.height()).map(|i| format!("Point {}", i)).collect()
	};
	let mut points = Vec::with_capacity(df.height());
	for i in 0..df.height() {
		points.push(BubblePoint {
			x: x_f64.get(i).unwrap(),
			y: y_f64.get(i).unwrap(),
			size_val: size_f64.get(i).unwrap(),
			color_val: color_f64.get(i).unwrap(),
			label: labels[i].clone(),
		});
	}
	BubblePreparedData {
		points,
		x_range: (x_range.0 - x_pad, x_range.1 + x_pad),
		y_range: (y_range.0 - y_pad, y_range.1 + y_pad),
		x_axis_type,
		y_axis_type,
		size_range,
		color_range,
		x_label: x_col.to_string(),
		y_label: y_col.to_string(),
		size_label: size_col.to_string(),
		color_label: color_col.to_string(),
	}
}

pub fn generate_sample_bubble_data() -> DataFrame {
	let n = 100;
	let mut rng = rand::rng();
	let clusters: [(f32, f32, f32, f32, f32); 3] = [
		(20.0, 20.0, 0.6, 48.0, 0.15),
		(55.0, 70.0, -0.4, 26.0, 0.55),
		(80.0, 30.0, 1.2, 35.0, 0.85),
	];
	let points_per_cluster = n / clusters.len();
	let mut xs = Vec::with_capacity(n);
	let mut ys = Vec::with_capacity(n);
	let mut sizes = Vec::with_capacity(n);
	let mut color_vals = Vec::with_capacity(n);
	let mut labels = Vec::with_capacity(n);
	for (ci, &(cx, cy, slope, spread, hue)) in clusters.iter().enumerate() {
		for i in 0..points_per_cluster {
			let x_offset: f32 = rng.random_range(-spread..spread);
			let x = (cx + x_offset).clamp(0.0, 100.0);
			let noise: f32 = rng.random_range(-spread * 0.5..spread * 0.5);
			let y = (cy + slope * x_offset + noise).clamp(0.0, 100.0);
			let dist = x_offset.abs();
			let size = (40.0 - dist * 0.6 + rng.random_range(-3.0..3.0)).clamp(5.0, 50.0);
			let color = (hue + rng.random_range(-0.06..0.06)).clamp(0.0, 1.0);
			xs.push(x);
			ys.push(y);
			sizes.push(size);
			color_vals.push(color);
			labels.push(format!("Cluster {} – Item {}", ci + 1, i));
		}
	}
	let remainder = n - xs.len();
	for i in 0..remainder {
		let (cx, cy, slope, spread, hue) = clusters[0];
		let x_offset: f32 = rng.random_range(-spread..spread);
		let x = (cx + x_offset).clamp(0.0, 100.0);
		let noise: f32 = rng.random_range(-spread * 0.5..spread * 0.5);
		let y = (cy + slope * x_offset + noise).clamp(0.0, 100.0);
		xs.push(x);
		ys.push(y);
		sizes.push(rng.random_range(5.0..50.0));
		color_vals.push((hue + rng.random_range(-0.06..0.06)).clamp(0.0, 1.0));
		labels.push(format!("Cluster 1 – Item {}", points_per_cluster + i));
	}
	DataFrame::new(
		n,
		vec![
			Column::new("x".into(), xs),
			Column::new("y".into(), ys),
			Column::new("size".into(), sizes),
			Column::new("color".into(), color_vals),
			Column::new("label".into(), labels),
		],
	)
	.unwrap()
}
