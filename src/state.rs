use crate::message::Message;
use crate::plot::{PlotKernel, PlotWidget};
use crate::violin::{self, ViolinPlotKernel};
use iced::widget::{canvas, container};
use iced::{window, Element, Length, Size, Task};
use std::sync::Arc;

struct AppState {
	kernel: Box<dyn PlotKernel>,
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
	let prepared = violin::prepare_violin_data(&df, "group", "y", None);
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
	};
	(state, task)
}

fn update(state: &mut AppState, message: Message) -> Task<Message> {
	match message {
		Message::RasterizationResult(w, h, pixels) => {
			state.kernel.update_raster(w, h, pixels);
			Task::none()
		}
		Message::None => Task::none(),
	}
}

fn view(state: &AppState) -> Element<'_, Message> {
	let content = container(
		canvas(PlotWidget {
			kernel: state.kernel.as_ref(),
			padding: 50.0,
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
	let element: Element<'_, ()> = content.into();
	element.map(|_| Message::None)
}
