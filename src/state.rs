use crate::message::Message;
use crate::plot::{PlotKernel, PlotWidget};
use crate::violin::{self, ViolinPlotKernel};
use iced::widget::{canvas, container, text, tooltip, Tooltip};
use iced::{window, Element, Length, Size, Task};
use std::sync::Arc;

struct AppState {
	kernel: Box<dyn PlotKernel>,
	hovered_info: Option<String>,
	#[allow(dead_code)]
	current_size: (u32, u32),
}

pub type Result = iced::Result;

pub fn run() -> Result {
	iced::application(new, update, view)
		.title("Polariz")
		.window(window::Settings {
			size: Size::new(1800.0, 1000.0),
			..Default::default()
		})
		.run()
}

fn new() -> (AppState, Task<Message>) {
	let df = violin::generate_sample_data();
	let cat_col = "group";
	let val_col = "y";
	let prepared = violin::prepare_violin_data(&df, cat_col, val_col, None);
	let kernel = ViolinPlotKernel {
		prepared_data: Arc::new(prepared),
		image_cache: None,
	};
	let width = 7680;
	let height = 4320;
	let boxed_kernel: Box<dyn PlotKernel> = Box::new(kernel);
	let task = boxed_kernel.rasterize(width, height);
	let state = AppState {
		kernel: boxed_kernel,
		hovered_info: None,
		current_size: (width, height),
	};
	(state, task)
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
	}
}

fn view(state: &AppState) -> Element<'_, Message> {
	let canvas_widget = canvas(PlotWidget {
		kernel: state.kernel.as_ref(),
		padding: 50.0,
	})
	.width(Length::Fill)
	.height(Length::Fill);
	let content: Element<_> = if let Some(info) = &state.hovered_info {
		Tooltip::new(canvas_widget, text(info), tooltip::Position::FollowCursor).into()
	} else {
		canvas_widget.into()
	};
	container(content)
		.width(Length::Fill)
		.height(Length::Fill)
		.style(|_| container::Style {
			background: Some(iced::Background::Color(iced::Color::from_rgba(
				0.001, 0.001, 0.001, 0.8,
			))),
			..Default::default()
		})
		.into()
}
