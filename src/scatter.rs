use crate::colors;
use crate::plot::{CoordinateTransformer, PlotKernel, PlotLayout};
use iced::advanced::mouse::Cursor;
use iced::widget::canvas::{Frame, Path};
use iced::{Color, Rectangle};
use polars::prelude::*;
use rand::RngExt;
use std::sync::Arc;

pub struct ScatterPlotKernel {
	pub prepared_data: Arc<ScatterPreparedData>,
}

impl PlotKernel for ScatterPlotKernel {
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
		for series in &self.prepared_data.series {
			let color = colors::viridis(series.color_t);
			let path = Path::new(|builder| {
				for p in &series.points {
					let pixel_p = transform.cartesian(p[0], p[1]);
					builder.circle(pixel_p, self.prepared_data.point_size_px);
				}
			});
			frame.fill(&path, color);
		}
	}

	fn draw_legend(&self, frame: &mut Frame, bounds: Rectangle, settings: crate::plot::LegendSettings) {
		let num_series = self.prepared_data.series.len();
		if num_series == 0 { return; }
		let max_rows = settings.max_rows.max(1) as usize;
		let num_cols = num_series.div_ceil(max_rows);
		let actual_rows = num_series.min(max_rows);
		let item_height = 25.0;
		let legend_padding = 10.0;
		let swatch_size = 12.0;
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
		for (i, series) in self.prepared_data.series.iter().enumerate() {
			let color = colors::viridis(series.color_t);
			let col = i / max_rows;
			let row = i % max_rows;
			let item_x = x + legend_padding + col as f32 * col_width;
			let item_y = y + legend_padding + row as f32 * item_height;
			let dot_path = Path::circle(
				iced::Point::new(item_x + swatch_size / 2.0, item_y + item_height / 2.0),
				swatch_size / 2.0
			);
			frame.fill(&dot_path, color);
			frame.fill_text(iced::widget::canvas::Text {
				content: series.name.clone(),
				position: iced::Point::new(item_x + swatch_size + 10.0, item_y + item_height / 2.0),
				color: Color::WHITE,
				size: iced::Pixels(14.0),
				align_x: iced::alignment::Horizontal::Left.into(),
				align_y: iced::alignment::Vertical::Center,
				..Default::default()
			});
		}
	}

	fn hover(&self, transform: &CoordinateTransformer, cursor: Cursor) -> Option<String> {
		if let Some(cursor_pos) = cursor.position()
			&& let Some((x, y)) = transform.pixel_to_cartesian(cursor_pos) {
			return Some(format!("X: {:.2}, Y: {:.2}", x, y));
		}
		None
	}
}

pub struct ScatterSeries {
	pub name: String,
	pub points: Vec<[f32; 2]>,
	pub color_t: f32,
}

pub struct ScatterPreparedData {
	pub series: Vec<ScatterSeries>,
	pub x_range: (f32, f32),
	pub y_range: (f32, f32),
	pub point_size_px: f32,
}

pub fn prepare_scatter_data(df: &DataFrame, cat_col: &str, x_col: &str, y_col: &str, point_size_px: f32) -> ScatterPreparedData {
	let x_col_series = df.column(x_col).unwrap().cast(&DataType::Float32).unwrap();
	let y_col_series = df.column(y_col).unwrap().cast(&DataType::Float32).unwrap();
	let x_series = x_col_series.f32().unwrap();
	let y_series = y_col_series.f32().unwrap();
	let x_range = (x_series.min().unwrap_or(0.0), x_series.max().unwrap_or(1.0));
	let y_range = (y_series.min().unwrap_or(0.0), y_series.max().unwrap_or(1.0));
	let x_pad = (x_range.1 - x_range.0).max(0.001) * 0.1;
	let y_pad = (y_range.1 - y_range.0).max(0.001) * 0.1;
	let x_range = (x_range.0 - x_pad, x_range.1 + x_pad);
	let y_range = (y_range.0 - y_pad, y_range.1 + y_pad);
	let partitions = df.partition_by([cat_col], true).unwrap();
	let num_partitions = partitions.len();
	let mut series_list = Vec::with_capacity(num_partitions);
	for (i, group_df) in partitions.into_iter().enumerate() {
		let cat_val = group_df.column(cat_col).unwrap().get(0).unwrap();
		let cat_name = if let AnyValue::String(s) = cat_val {
			s.to_string()
		} else {
			cat_val.to_string().replace("\"", "")
		};
		let xs_col = group_df.column(x_col).unwrap().cast(&DataType::Float32).unwrap();
		let ys_col = group_df.column(y_col).unwrap().cast(&DataType::Float32).unwrap();
		let xs = xs_col.f32().unwrap();
		let ys = ys_col.f32().unwrap();
		let t = if num_partitions > 1 {
			i as f32 / (num_partitions - 1) as f32
		} else {
			0.5
		};
		let mut points = Vec::with_capacity(group_df.height());
		for j in 0..group_df.height() {
			points.push([xs.get(j).unwrap(), ys.get(j).unwrap()]);
		}
		series_list.push(ScatterSeries { name: cat_name, points, color_t: t });
	}

	ScatterPreparedData {
		series: series_list,
		x_range,
		y_range,
		point_size_px,
	}
}

pub fn generate_sample_scatter_data() -> DataFrame {
	let num_series = 8;
	let n_per_series = 1000;
	let total_n = num_series * n_per_series;
	let mut rng = rand::rng();
	let mut cats = Vec::with_capacity(total_n);
	let mut xs = Vec::with_capacity(total_n);
	let mut ys = Vec::with_capacity(total_n);
	use rand_distr::{Distribution, Normal};
	for i in 0..num_series {
		let cat = format!("Cluster {}", i);
		let center_x = rng.random_range(0.0..10.0f32);
		let center_y = rng.random_range(0.0..10.0f32);
		let dist_x = Normal::new(center_x, 0.5 + rng.random_range(0.0..0.10f32)).unwrap();
		let dist_y = Normal::new(center_y, 0.5 + rng.random_range(0.0..0.10f32)).unwrap();
		for _ in 0..n_per_series {
			cats.push(cat.clone());
			xs.push(dist_x.sample(&mut rng));
			ys.push(dist_y.sample(&mut rng));
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
