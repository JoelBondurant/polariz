use crate::colors;
use crate::plot::{CoordinateTransformer, PlotKernel, PlotLayout};
use iced::advanced::mouse::Cursor;
use iced::widget::canvas::{Frame, Path, Stroke, Style};
use iced::{Color, Rectangle};
use polars::lazy::prelude::*;
use polars::prelude::*;
use rand_distr::{Distribution, Normal};
use std::sync::Arc;

pub struct HexbinPlotKernel {
	pub prepared_data: Arc<HexbinPreparedData>,
}

impl PlotKernel for HexbinPlotKernel {
	fn layout(&self) -> PlotLayout {
		PlotLayout::Cartesian {
			x_range: self.prepared_data.x_range,
			y_range: self.prepared_data.y_range,
		}
	}

	fn plot(
		&self,
		frame: &mut Frame,
		bounds: Rectangle,
		transform: &CoordinateTransformer,
		_cursor: Cursor,
	) {
		let radius = self.prepared_data.radius;
		let sqrt_3 = 3.0f32.sqrt();
		let max_count = self.prepared_data.max_count as f32;
		frame.with_clip(bounds, |frame| {
			for (&(q, r), &count) in &self.prepared_data.bins {
				if count == 0 { continue; }
				let lx = radius * (sqrt_3 * q as f32 + sqrt_3 / 2.0 * r as f32);
				let ly = radius * (3.0 / 2.0 * r as f32);
				let hex_path = Path::new(|builder| {
					for i in 0..6 {
						let angle_deg = 60.0 * i as f32 - 30.0;
						let angle_rad = std::f32::consts::PI / 180.0 * angle_deg;
						let dx = radius * angle_rad.cos();
						let dy = radius * angle_rad.sin();
						let p = transform.cartesian(lx + dx, ly + dy);
						if i == 0 {
							builder.move_to(p);
						} else {
							builder.line_to(p);
						}
					}
					builder.close();
				});
				let t = count as f32 / max_count;
				let color = colors::viridis(t);
				frame.fill(&hex_path, color);
				frame.stroke(&hex_path, Stroke {
					style: Style::Solid(Color::from_rgba(0.0, 0.0, 0.0, 0.1)),
					width: 0.5,
					..Default::default()
				});
			}
		});
	}

	fn hover(&self, transform: &CoordinateTransformer, cursor: Cursor) -> Option<String> {
		if let Some(cursor_pos) = cursor.position()
			&& let Some((x, y)) = transform.pixel_to_cartesian(cursor_pos) {
			let radius = self.prepared_data.radius;
			let sqrt_3 = 3.0f32.sqrt();
			let q_frac = (sqrt_3 / 3.0 * x - 1.0 / 3.0 * y) / radius;
			let r_frac = (2.0 / 3.0 * y) / radius;
			let mut q = q_frac.round();
			let mut r = r_frac.round();
			let s = (-q_frac - r_frac).round();
			let q_diff = (q - q_frac).abs();
			let r_diff = (r - r_frac).abs();
			let s_diff = (s - (-q_frac - r_frac)).abs();
			if q_diff > r_diff && q_diff > s_diff {
				q = -r - s;
			} else if r_diff > s_diff {
				r = -q - s;
			}
			if let Some(&count) = self.prepared_data.bins.get(&(q as i32, r as i32)) {
				return Some(format!("Bin: ({}, {})\nCount: {}\nPos: ({:.2}, {:.2})", q, r, count, x, y));
			}
		}
		None
	}
}

pub struct HexbinPreparedData {
	pub bins: std::collections::HashMap<(i32, i32), u32>,
	pub max_count: u32,
	pub radius: f32,
	pub x_range: (f32, f32),
	pub y_range: (f32, f32),
}

fn bin_data_to_hex(df: DataFrame, radius: f32) -> PolarsResult<DataFrame> {
	let sqrt_3 = 3.0f32.sqrt();
	let with_frac = df.lazy().with_columns([
		((lit(sqrt_3 / 3.0) * col("x") - lit(1.0f32 / 3.0) * col("y")) / lit(radius))
			.alias("q_frac"),
		((lit(2.0f32 / 3.0) * col("y")) / lit(radius)).alias("r_frac"),
	]);
	let with_rounded = with_frac.with_columns([
		col("q_frac").round(0, RoundMode::HalfToEven).alias("q"),
		col("r_frac").round(0, RoundMode::HalfToEven).alias("r"),
		(-col("q_frac") - col("r_frac"))
			.round(0, RoundMode::HalfToEven)
			.alias("s"),
	]);
	let with_diffs = with_rounded.with_columns([
		(col("q_frac") - col("q")).abs().alias("q_diff"),
		(col("r_frac") - col("r")).abs().alias("r_diff"),
		((-col("q_frac") - col("r_frac")) - col("s"))
			.abs()
			.alias("s_diff"),
	]);
	let with_corrected = with_diffs.with_columns([
		when(
			col("q_diff")
				.gt(col("r_diff"))
				.and(col("q_diff").gt(col("s_diff"))),
		)
		.then(-col("r") - col("s"))
		.otherwise(col("q"))
		.cast(DataType::Int32)
		.alias("q"),
		when(
			col("q_diff")
				.gt(col("r_diff"))
				.and(col("q_diff").gt(col("s_diff")))
				.not()
				.and(col("r_diff").gt(col("s_diff"))),
		)
		.then(-col("q") - col("s"))
		.otherwise(col("r"))
		.cast(DataType::Int32)
		.alias("r"),
	]);
	with_corrected
		.group_by([col("q"), col("r")])
		.agg([len().alias("count")])
		.collect()
}

pub fn prepare_hexbin_data(df: &DataFrame, radius: f32) -> HexbinPreparedData {
	let x_col = df.column("x").unwrap().f32().unwrap();
	let y_col = df.column("y").unwrap().f32().unwrap();
	let x_range = (x_col.min().unwrap_or(0.0), x_col.max().unwrap_or(1.0));
	let y_range = (y_col.min().unwrap_or(0.0), y_col.max().unwrap_or(1.0));
	let binned = bin_data_to_hex(df.clone(), radius).unwrap();
	let q_col = binned.column("q").unwrap().i32().unwrap();
	let r_col = binned.column("r").unwrap().i32().unwrap();
	let count_col = binned.column("count").unwrap().u32().unwrap();
	let mut bins = std::collections::HashMap::new();
	let mut max_count = 0;
	for i in 0..binned.height() {
		let q = q_col.get(i).unwrap();
		let r = r_col.get(i).unwrap();
		let count = count_col.get(i).unwrap();
		bins.insert((q, r), count);
		if count > max_count {
			max_count = count;
		}
	}
	HexbinPreparedData {
		bins,
		max_count,
		radius,
		x_range,
		y_range,
	}
}

pub fn generate_sample_hex_data(width: u32, height: u32) -> DataFrame {
	let aspect_ratio = width as f32 / height as f32;
	let n = 1_000_000usize;
	let mut rng = rand::rng();
	let normal_x = Normal::new(0.5 * aspect_ratio, 0.12).unwrap();
	let normal_y = Normal::new(0.5, 0.12).unwrap();
	let xs: Vec<f32> = (0..n).map(|_| normal_x.sample(&mut rng)).collect();
	let ys: Vec<f32> = (0..n).map(|_| normal_y.sample(&mut rng)).collect();
	DataFrame::new(
		n,
		vec![Column::new("x".into(), xs), Column::new("y".into(), ys)],
	)
	.unwrap()
}
