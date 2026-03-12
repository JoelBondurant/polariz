use crate::message::Message;
use crate::plot::{PlotLayout, PlotWidget};
use crate::violin::{self, ViolinPlotKernel};
use iced::widget::{canvas, container};
use iced::{Element, Length, Task};
use polars::prelude::*;
use std::sync::Arc;

struct AppState {
	kernel: ViolinPlotKernel,
	df: Arc<DataFrame>,
	cat_col: String,
	val_col: String,
	#[allow(dead_code)]
	current_size: (u32, u32),
}

pub type Result = iced::Result;

pub fn run() -> Result {
	iced::application(new, update, view).title("Polariz").run()
}

fn new() -> (AppState, Task<Message>) {
	let df = violin::generate_sample_data();
	let cat_col = "group".to_string();
	let val_col = "y".to_string();
	let vals_series = df
		.column(&val_col)
		.unwrap()
		.as_materialized_series()
		.f32()
		.unwrap();
	let (y_min, y_max) = (vals_series.min().unwrap(), vals_series.max().unwrap());
	let pad = (y_max - y_min) * 0.1;
	let y_range = (y_min - pad, y_max + pad);
	let categories_series = df
		.column(&cat_col)
		.unwrap()
		.unique()
		.unwrap()
		.sort(SortOptions::default())
		.unwrap();
	let categories_series = categories_series.as_materialized_series();
	let categories: Vec<String> = if let Ok(ca) = categories_series.i32() {
		ca.into_no_null_iter().map(|i| i.to_string()).collect()
	} else {
		(0..categories_series.len())
			.map(|i| i.to_string())
			.collect()
	};
	let group_data = df
		.clone()
		.lazy()
		.group_by([col(&cat_col)])
		.agg([col(&val_col).median().alias("median")])
		.sort([&cat_col], Default::default())
		.collect()
		.expect("Failed to aggregate data");
	let medians_series = group_data
		.column("median")
		.unwrap()
		.as_materialized_series()
		.f32()
		.unwrap();
	let medians: Vec<f32> = medians_series.into_no_null_iter().collect();
	let kernel = ViolinPlotKernel {
		layout_cache: PlotLayout::CategoricalX {
			categories,
			y_range,
		},
		image_cache: None,
		medians,
	};
	let width = 3840;
	let height = 2160;
	let state = AppState {
		kernel,
		df: Arc::new(df),
		cat_col: cat_col.clone(),
		val_col: val_col.clone(),
		current_size: (width, height),
	};
	let task = Task::perform(
		generate_plot_task(
			state.df.clone(),
			state.cat_col.clone(),
			state.val_col.clone(),
			width,
			height,
			Some(y_range),
		),
		|(w, h, pixels)| Message::WgpuRasterFinished(w, h, pixels),
	);
	(state, task)
}

fn update(state: &mut AppState, message: Message) -> Task<Message> {
	match message {
		Message::WgpuRasterFinished(w, h, pixels) => {
			let handle = iced::widget::image::Handle::from_rgba(w, h, pixels);
			state.kernel.image_cache = Some(handle);
			Task::none()
		}
		Message::None => Task::none(),
	}
}

fn view(state: &AppState) -> Element<'_, Message> {
	let content = container(
		canvas(PlotWidget {
			kernel: &state.kernel,
			padding: 40.0,
		})
		.width(Length::Fill)
		.height(Length::Fill),
	)
	.width(Length::Fill)
	.height(Length::Fill)
	.style(|_| container::Style {
		background: Some(iced::Background::Color(iced::Color::from_rgba(
			0.001, 0.001, 0.001, 0.8,
		))),
		..Default::default()
	});
	let element: Element<()> = content.into();
	element.map(|_| Message::None)
}

async fn generate_plot_task(
	df: Arc<DataFrame>,
	cat_col: String,
	val_col: String,
	width: u32,
	height: u32,
	range: Option<(f32, f32)>,
) -> (u32, u32, Vec<u8>) {
	violin::generate_violin_plot(&df, &cat_col, &val_col, width, height, range).await
}
