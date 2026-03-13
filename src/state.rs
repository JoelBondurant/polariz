use crate::hexbin::{self, HexbinPlotKernel};
use crate::message::{Message, PlotType};
use crate::plot::{PlotKernel, PlotWidget};
use crate::violin::{self, ViolinPlotKernel};
use iced::widget::{canvas, column, container, pick_list, row, text, tooltip, Tooltip};
use iced::{window, Element, Length, Size, Task};
use std::sync::Arc;

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
			size: Size::new(1200.0, 1200.0),
			..Default::default()
		})
		.run()
}

fn new() -> (AppState, Task<Message>) {
	let width = 1200;
	let height = 1200;
	let plot_type = PlotType::Violin;
	let (kernel, task) = create_plot(plot_type, width, height);
	let state = AppState {
		kernel,
		hovered_info: None,
		current_plot_type: plot_type,
		current_size: (width, height),
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
				let (new_kernel, task) = create_plot(new_type, 1200, 1200);
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
