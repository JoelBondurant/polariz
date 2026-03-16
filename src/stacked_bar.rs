use crate::colors;
use crate::plot::{CoordinateTransformer, PlotKernel, PlotLayout};
use iced::advanced::mouse::Cursor;
use iced::widget::canvas::Frame;
use iced::Rectangle;
use polars::prelude::*;
use rand::RngExt;
use std::sync::Arc;
use std::collections::HashMap;

pub struct StackedBarPlotKernel {
	pub prepared_data: Arc<StackedBarPreparedData>,
}

impl PlotKernel for StackedBarPlotKernel {
	fn layout(&self) -> PlotLayout {
		PlotLayout::CategoricalX {
			categories: self.prepared_data.categories.clone(),
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
		let num_groups = self.prepared_data.group_names.len();
		let total_band_width = transform.bounds.width / num_cats as f32;
		let bar_width = total_band_width * 0.8;
		let bar_offset = (total_band_width - bar_width) / 2.0;
		for i in 0..num_cats {
			let cat_left = transform.bounds.x + (i as f32 * total_band_width) + bar_offset;
			let mut current_y = 0.0f32;
			for j in 0..num_groups {
				let val = self.prepared_data.category_values[i][j];
				if val <= 0.0 { continue; }
				let (p_top, _) = transform.categorical(i, current_y + val);
				let (p_bottom, _) = transform.categorical(i, current_y);
				let bar_rect = Rectangle {
					x: cat_left,
					y: p_top.y,
					width: bar_width,
					height: (p_bottom.y - p_top.y).max(1.0),
				};
				let t = if num_groups > 1 { j as f32 / (num_groups - 1) as f32 } else { 0.5 };
				let color = colors::viridis(t);
				frame.fill_rectangle(bar_rect.position(), bar_rect.size(), color);
				
				current_y += val;
			}
		}
	}

	fn hover(&self, transform: &CoordinateTransformer, cursor: Cursor) -> Option<String> {
		if let Some(cursor_pos) = cursor.position()
			&& let PlotLayout::CategoricalX { categories, y_range } = self.layout() {
			for (i, cat_name) in categories.iter().enumerate() {
				let (center, band_width) = transform.categorical(i, 0.0);
				let left = center.x - band_width / 2.0;
				let right = center.x + band_width / 2.0;
				if cursor_pos.x >= left && cursor_pos.x <= right {
					let bar_width = band_width * 0.8;
					let bar_offset = (band_width - bar_width) / 2.0;
					let bar_left = left + bar_offset;
					let bar_right = left + bar_offset + bar_width;
					if cursor_pos.x >= bar_left && cursor_pos.x <= bar_right {
						let y_scale = transform.bounds.height / (y_range.1 - y_range.0);
						let data_y = y_range.0 + (transform.bounds.y + transform.bounds.height - cursor_pos.y) / y_scale;
						let mut current_sum = 0.0;
						for (j, &val) in self.prepared_data.category_values[i].iter().enumerate() {
							if data_y >= current_sum && data_y <= current_sum + val {
								let group_name = &self.prepared_data.group_names[j];
								return Some(format!("{}: {} (Value: {:.2}, Total: {:.2})", cat_name, group_name, val, current_sum + val));
							}
							current_sum += val;
						}
						return Some(format!("{}: Total {:.2}", cat_name, current_sum));
					}
				}
			}
		}
		None
	}
}

pub struct StackedBarPreparedData {
	pub categories: Vec<String>,
	pub group_names: Vec<String>,
	pub category_values: Vec<Vec<f32>>, // For hover detection
	pub y_range: (f32, f32),
}

pub fn prepare_stacked_bar_data(df: &DataFrame, cat_col: &str, group_col: &str, val_col: &str) -> StackedBarPreparedData {
	let categories_series = df.column(cat_col).unwrap().unique().unwrap().sort(Default::default()).unwrap();
	let categories: Vec<String> = categories_series.as_materialized_series().iter().map(|v| {
		if let AnyValue::String(s) = v { s.to_string() } else { v.to_string() }
	}).collect();
	let groups_series = df.column(group_col).unwrap().unique().unwrap().sort(Default::default()).unwrap();
	let groups_series_mat = groups_series.as_materialized_series();
	let group_names: Vec<String> = groups_series_mat.iter().map(|v| {
		if let AnyValue::String(s) = v { s.to_string() } else { v.to_string() }
	}).collect();
	let group_idx_map: HashMap<AnyValue, usize> = groups_series_mat.iter().enumerate().map(|(i, v)| (v.into_static(), i)).collect();
	let num_cats = categories.len();
	let num_groups = group_names.len();
	let mut category_values = vec![vec![0.0f32; num_groups]; num_cats];
	let mut max_sum = 0.0f32;
	let partitions = df.partition_by([cat_col], true).unwrap();
	for (i, group_df) in partitions.into_iter().enumerate() {
		let group_partitions = group_df.partition_by([group_col], true).unwrap();
		let mut current_cat_sum = 0.0f32;
		for sub_group_df in group_partitions {
			let group_val = sub_group_df.column(group_col).unwrap().get(0).unwrap();
			if let Some(&group_idx) = group_idx_map.get(&group_val) {
				let val = sub_group_df.column(val_col).unwrap().cast(&DataType::Float32).unwrap().f32().unwrap().get(0).unwrap_or(0.0);
				category_values[i][group_idx] = val;
				current_cat_sum += val;
			}
		}
		if current_cat_sum > max_sum {
			max_sum = current_cat_sum;
		}
	}
	let y_min = 0.0f32;
	let y_range = (y_min, max_sum * 1.1);
	StackedBarPreparedData {
		categories,
		group_names,
		category_values,
		y_range,
	}
}

pub fn generate_sample_stacked_bar_data() -> DataFrame {
	let num_cats = 8;
	let num_groups = 5;
	let total_n = num_cats * num_groups;
	let mut cats = Vec::with_capacity(total_n);
	let mut groups = Vec::with_capacity(total_n);
	let mut vals = Vec::with_capacity(total_n);
	let mut rng = rand::rng();
	for i in 0..num_cats {
		let cat = format!("Cat {}", i);
		for j in 0..num_groups {
			let group = format!("Group {}", j);
			cats.push(cat.clone());
			groups.push(group);
			vals.push(rng.random_range(5.0..25.0f32));
		}
	}
	DataFrame::new(
		total_n,
		vec![
			Column::new("cat".into(), cats),
			Column::new("group".into(), groups),
			Column::new("val".into(), vals),
		],
	)
	.unwrap()
}
