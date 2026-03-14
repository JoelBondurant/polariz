use crate::bar::{self, BarPlotKernel};
use crate::box_plot::{self, BoxPlotKernel};
use crate::hexbin::{self, HexbinPlotKernel};
use crate::histogram::{self, HistogramPlotKernel};
use crate::line::{self, LinePlotKernel};
use crate::message::Message;
use crate::pie::{self, PiePlotKernel};
use crate::plot::{PlotKernel, PlotWidget};
use crate::plot_core::PlotType;
use crate::scatter::{self, ScatterPlotKernel};
use crate::stacked_bar::{self, StackedBarPlotKernel};
use crate::violin::{self, ViolinPlotKernel};
use iced::widget::{canvas, column, container, pick_list, row, text, tooltip, Tooltip};
use iced::{window, Element, Length, Size, Task};
use std::sync::Arc;

const WIDTH: u32 = 1200;
const HEIGHT: u32 = 1200;

struct AppState {
	kernel: Box<dyn PlotKernel>,
	hovered_info: Option<String>,
	current_plot_type: PlotType,
	#[allow(dead_code)]
	current_size: (u32, u32),
}

pub type Result = iced::Result;

pub fn run() -> Result {
	iced::application(new, update, view)
		.title("Polariz")
		.window(window::Settings {
			size: Size::new(WIDTH as f32, HEIGHT as f32),
			..Default::default()
		})
		.run()
}

fn new() -> (AppState, Task<Message>) {
	let plot_type = PlotType::Bar;
	let (kernel, task) = create_plot(plot_type, WIDTH, HEIGHT);
	let state = AppState {
		kernel,
		hovered_info: None,
		current_plot_type: plot_type,
		current_size: (WIDTH, HEIGHT),
	};
	(state, task)
}

fn create_plot(
	plot_type: PlotType,
	width: u32,
	height: u32,
) -> (Box<dyn PlotKernel>, Task<Message>) {
	match plot_type {
		PlotType::Violin => {
			let df = violin::generate_sample_data();
			let prepared = violin::prepare_violin_data(&df, "group", "y", None);
			let kernel = ViolinPlotKernel {
				prepared_data: Arc::new(prepared),
				image_cache: None,
			};
			let task = kernel.rasterize(width, height);
			(Box::new(kernel), task)
		}
		PlotType::Hexbin => {
			let df = hexbin::generate_sample_hex_data(width, height);
			let prepared = hexbin::prepare_hexbin_data(&df, 0.02);
			let kernel = HexbinPlotKernel {
				prepared_data: Arc::new(prepared),
				image_cache: None,
			};
			let task = kernel.rasterize(width, height);
			(Box::new(kernel), task)
		}
		PlotType::Line => {
			let df = line::generate_sample_line_data();
			let prepared = line::prepare_line_data(&df, "cat", "x", "y");
			let kernel = LinePlotKernel {
				prepared_data: Arc::new(prepared),
				image_cache: None,
			};
			let task = kernel.rasterize(width, height);
			(Box::new(kernel), task)
		}
		PlotType::Bar => {
			let df = bar::generate_sample_bar_data();
			let prepared = bar::prepare_bar_data(&df, "cat", "group", "val");
			let kernel = BarPlotKernel {
				prepared_data: Arc::new(prepared),
				image_cache: None,
			};
			let task = kernel.rasterize(width, height);
			(Box::new(kernel), task)
		}
		PlotType::Scatter => {
			let df = scatter::generate_sample_scatter_data();
			let prepared = scatter::prepare_scatter_data(&df, "cat", "x", "y", 0.005);
			let kernel = ScatterPlotKernel {
				prepared_data: Arc::new(prepared),
				image_cache: None,
			};
			let task = kernel.rasterize(width, height);
			(Box::new(kernel), task)
		}
		PlotType::StackedBar => {
			let df = stacked_bar::generate_sample_stacked_bar_data();
			let prepared = stacked_bar::prepare_stacked_bar_data(&df, "cat", "group", "val");
			let kernel = StackedBarPlotKernel {
				prepared_data: Arc::new(prepared),
				image_cache: None,
			};
			let task = kernel.rasterize(width, height);
			(Box::new(kernel), task)
		}
		PlotType::Pie => {
			let df = pie::generate_sample_pie_data();
			let prepared = pie::prepare_pie_data(&df, "cat", "val");
			let kernel = PiePlotKernel {
				prepared_data: Arc::new(prepared),
				image_cache: None,
			};
			let task = kernel.rasterize(width, height);
			(Box::new(kernel), task)
		}
		PlotType::BoxPlot => {
			let df = violin::generate_sample_data();
			let prepared = box_plot::prepare_box_plot_data(&df, "group", "y", height);
			let kernel = BoxPlotKernel {
				prepared_data: Arc::new(prepared),
				image_cache: None,
			};
			let task = kernel.rasterize(width, height);
			(Box::new(kernel), task)
		}
		PlotType::Histogram => {
			let df = histogram::generate_sample_histogram_data();
			let prepared = histogram::prepare_histogram_data(&df, "val", 50);
			let kernel = HistogramPlotKernel {
				prepared_data: Arc::new(prepared),
				image_cache: None,
			};
			let task = kernel.rasterize(width, height);
			(Box::new(kernel), task)
		}
	}
}

fn update(state: &mut AppState, message: Message) -> Task<Message> {
	match message {
		Message::RasterizationResult(w, h, pixels) => {
			state.kernel.update_raster(w, h, pixels);
			Task::none()
		}
		Message::UpdateHover(hover) => {
			state.hovered_info = hover;
			Task::none()
		}
		Message::ChangePlotType(new_type) => {
			if new_type != state.current_plot_type {
				state.current_plot_type = new_type;
				let (new_kernel, task) = create_plot(new_type, WIDTH, HEIGHT);
				state.kernel = new_kernel;
				state.hovered_info = None;
				return task;
			}
			Task::none()
		}
	}
}

fn view(state: &AppState) -> Element<'_, Message> {
	let canvas_widget = canvas(PlotWidget {
		kernel: state.kernel.as_ref(),
		title: state.current_plot_type.to_string(),
		padding: 50.0,
	})
	.width(Length::Fill)
	.height(Length::Fill);
	let plot_content: Element<_> = if let Some(info) = &state.hovered_info {
		Tooltip::new(canvas_widget, text(info), tooltip::Position::FollowCursor).into()
	} else {
		canvas_widget.into()
	};
	let controls = row![
		text("Plot Type:"),
		pick_list(
			&PlotType::ALL[..],
			Some(state.current_plot_type),
			Message::ChangePlotType
		),
	]
	.spacing(10)
	.padding(5)
	.align_y(iced::Alignment::Center);
	container(
		column![controls, plot_content]
			.width(Length::Fill)
			.height(Length::Fill),
	)
	.style(|_| container::Style {
		background: Some(iced::Background::Color(iced::Color::from_rgba(
			0.001, 0.001, 0.001, 0.8,
		))),
		..Default::default()
	})
	.into()
}
