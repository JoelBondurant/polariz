use crate::bar::{self, BarPlotKernel};
use crate::box_plot::{self, BoxPlotKernel};
use crate::bubble::{self, BubblePlotKernel};
use crate::candlestick::{self, CandlestickPlotKernel};
use crate::fill_between::{self, FillBetweenPlotKernel};
use crate::funnel::{self, FunnelPlotKernel};
use crate::heatmap::{self, HeatmapPlotKernel};
use crate::hexbin::{self, HexbinPlotKernel};
use crate::histogram::{self, HistogramPlotKernel};
use crate::line::{self, LinePlotKernel};
use crate::message::Message;
use crate::parallel::{self, ParallelPlotKernel};
use crate::pie::{self, PiePlotKernel};
use crate::plot::{PlotKernel, PlotSettings, PlotWidget};
use crate::plot_core::PlotType;
use crate::radar::{self, RadarPlotKernel};
use crate::radial_dial::{self, RadialDialPlotKernel};
use crate::scatter::{self, ScatterPlotKernel};
use crate::stacked_area::{self, StackedAreaPlotKernel};
use crate::stacked_bar::{self, StackedBarPlotKernel};
use crate::violin::{self, ViolinPlotKernel};
use iced::widget::{canvas, column, container, pick_list, row, text, text_input, tooltip, Tooltip};
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
	plot_settings: PlotSettings,
	max_legend_rows_input: String,
	legend_x_input: String,
	legend_y_input: String,
	x_rotation_input: String,
	x_offset_input: String,
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
	let kernel = create_plot(plot_type, WIDTH, HEIGHT);
	let plot_settings = PlotSettings::default();
	let state = AppState {
		kernel,
		hovered_info: None,
		current_plot_type: plot_type,
		current_size: (WIDTH, HEIGHT),
		plot_settings,
		max_legend_rows_input: plot_settings.max_legend_rows.to_string(),
		legend_x_input: plot_settings.legend_x.to_string(),
		legend_y_input: plot_settings.legend_y.to_string(),
		x_rotation_input: plot_settings.x_label_rotation.to_string(),
		x_offset_input: plot_settings.x_label_offset.to_string(),
	};
	(state, Task::none())
}

fn create_plot(plot_type: PlotType, width: u32, height: u32) -> Box<dyn PlotKernel> {
	match plot_type {
		PlotType::Violin => {
			let df = violin::generate_sample_data();
			let prepared = violin::prepare_violin_data(&df, "group", "y", None);
			Box::new(ViolinPlotKernel {
				prepared_data: Arc::new(prepared),
			})
		}
		PlotType::Hexbin => {
			let df = hexbin::generate_sample_hex_data(width, height);
			let prepared = hexbin::prepare_hexbin_data(&df, 0.02);
			Box::new(HexbinPlotKernel {
				prepared_data: Arc::new(prepared),
			})
		}
		PlotType::Line => {
			let df = line::generate_sample_line_data();
			let prepared = line::prepare_line_data(&df, "cat", "x", "y");
			Box::new(LinePlotKernel {
				prepared_data: Arc::new(prepared),
			})
		}
		PlotType::Bar => {
			let df = bar::generate_sample_bar_data();
			let prepared = bar::prepare_bar_data(&df, "cat", "group", "val");
			Box::new(BarPlotKernel {
				prepared_data: Arc::new(prepared),
				orientation: bar::Orientation::Vertical,
			})
		}
		PlotType::HorizontalBar => {
			let df = bar::generate_sample_bar_data();
			let prepared = bar::prepare_bar_data(&df, "cat", "group", "val");
			Box::new(BarPlotKernel {
				prepared_data: Arc::new(prepared),
				orientation: bar::Orientation::Horizontal,
			})
		}
		PlotType::Scatter => {
			let df = scatter::generate_sample_scatter_data();
			let prepared = scatter::prepare_scatter_data(&df, "cat", "x", "y", 3.0);
			Box::new(ScatterPlotKernel {
				prepared_data: Arc::new(prepared),
			})
		}
		PlotType::StackedBar => {
			let df = stacked_bar::generate_sample_stacked_bar_data();
			let prepared = stacked_bar::prepare_stacked_bar_data(&df, "cat", "group", "val");
			Box::new(StackedBarPlotKernel {
				prepared_data: Arc::new(prepared),
				orientation: bar::Orientation::Vertical,
			})
		}
		PlotType::HorizontalStackedBar => {
			let df = stacked_bar::generate_sample_stacked_bar_data();
			let prepared = stacked_bar::prepare_stacked_bar_data(&df, "cat", "group", "val");
			Box::new(StackedBarPlotKernel {
				prepared_data: Arc::new(prepared),
				orientation: bar::Orientation::Horizontal,
			})
		}
		PlotType::Pie => {
			let df = pie::generate_sample_pie_data();
			let prepared = pie::prepare_pie_data(&df, "cat", "val");
			Box::new(PiePlotKernel {
				prepared_data: Arc::new(prepared),
			})
		}
		PlotType::BoxPlot => {
			let df = violin::generate_sample_data();
			let prepared = box_plot::prepare_box_plot_data(&df, "group", "y");
			Box::new(BoxPlotKernel {
				prepared_data: Arc::new(prepared),
			})
		}
		PlotType::Bubble => {
			let df = bubble::generate_sample_bubble_data();
			let prepared =
				bubble::prepare_bubble_data(&df, "x", "y", "size", "color", Some("label"));
			Box::new(BubblePlotKernel {
				prepared_data: Arc::new(prepared),
			})
		}
		PlotType::Candlestick => {
			let df = candlestick::generate_sample_candlestick_data();
			let prepared =
				candlestick::prepare_candlestick_data(&df, "x", "open", "high", "low", "close");
			Box::new(CandlestickPlotKernel {
				prepared_data: Arc::new(prepared),
			})
		}
		PlotType::FillBetween => {
			let df = fill_between::generate_sample_fill_between_data();
			let prepared =
				fill_between::prepare_fill_between_data(&df, "x", "y_mid", "y_lower", "y_upper");
			Box::new(FillBetweenPlotKernel {
				prepared_data: Arc::new(prepared),
			})
		}
		PlotType::Funnel => {
			let df = funnel::generate_sample_funnel_data();
			let prepared = funnel::prepare_funnel_data(&df, "stage", "value");
			Box::new(FunnelPlotKernel {
				prepared_data: Arc::new(prepared),
			})
		}
		PlotType::Heatmap => {
			let df = heatmap::generate_sample_heatmap_data();
			let prepared = heatmap::prepare_heatmap_data(&df, "x", "y", "val");
			Box::new(HeatmapPlotKernel {
				prepared_data: Arc::new(prepared),
			})
		}
		PlotType::Histogram => {
			let df = histogram::generate_sample_histogram_data();
			let prepared = histogram::prepare_histogram_data(&df, "val", 50);
			Box::new(HistogramPlotKernel {
				prepared_data: Arc::new(prepared),
			})
		}
		PlotType::StackedArea => {
			let df = stacked_area::generate_sample_stacked_area_data();
			let prepared = stacked_area::prepare_stacked_area_data(&df, "cat", "x", "y");
			Box::new(StackedAreaPlotKernel {
				prepared_data: Arc::new(prepared),
			})
		}
		PlotType::Parallel => {
			let df = parallel::generate_sample_parallel_data();
			let dims = vec![
				"Dim A".to_string(),
				"Dim B".to_string(),
				"Dim C".to_string(),
				"Dim D".to_string(),
			];
			let prepared = parallel::prepare_parallel_data(&df, &dims, "cat");
			Box::new(ParallelPlotKernel {
				prepared_data: Arc::new(prepared),
			})
		}
		PlotType::Radar => {
			let df = radar::generate_sample_radar_data();
			let dims = vec![
				"Speed".to_string(),
				"Power".to_string(),
				"Agility".to_string(),
				"Stamina".to_string(),
				"Skill".to_string(),
				"Luck".to_string(),
			];
			let prepared = radar::prepare_radar_data(&df, &dims, "cat");
			Box::new(RadarPlotKernel {
				prepared_data: Arc::new(prepared),
			})
		}
		PlotType::RadialDial => {
			let df = radial_dial::generate_sample_radial_dial_data();
			let prepared = radial_dial::prepare_radial_dial_data(&df, "cat", "val", "max");
			Box::new(RadialDialPlotKernel {
				prepared_data: Arc::new(prepared),
			})
		}
	}
}

fn update(state: &mut AppState, message: Message) -> Task<Message> {
	match message {
		Message::UpdateHover(hover) => {
			state.hovered_info = hover;
			Task::none()
		}
		Message::ChangePlotType(new_type) => {
			if new_type != state.current_plot_type {
				state.current_plot_type = new_type;
				let new_kernel = create_plot(new_type, WIDTH, HEIGHT);
				state.kernel = new_kernel;
				state.hovered_info = None;
			}
			Task::none()
		}
		Message::SetMaxLegendRows(rows) => {
			state.plot_settings.max_legend_rows = rows;
			state.max_legend_rows_input = rows.to_string();
			Task::none()
		}
		Message::SetLegendX(x) => {
			state.plot_settings.legend_x = x.clamp(0.0, 1.0);
			state.legend_x_input = x.to_string();
			Task::none()
		}
		Message::SetLegendY(y) => {
			state.plot_settings.legend_y = y.clamp(0.0, 1.0);
			state.legend_y_input = y.to_string();
			Task::none()
		}
		Message::SetXRotation(deg) => {
			state.plot_settings.x_label_rotation = deg;
			state.x_rotation_input = deg.to_string();
			Task::none()
		}
		Message::SetXOffset(offset) => {
			state.plot_settings.x_label_offset = offset;
			state.x_offset_input = offset.to_string();
			Task::none()
		}
	}
}

fn view(state: &AppState) -> Element<'_, Message> {
	let canvas_widget = canvas(PlotWidget {
		kernel: state.kernel.as_ref(),
		title: state.current_plot_type.to_string(),
		padding: 50.0,
		settings: state.plot_settings,
	})
	.width(Length::Fill)
	.height(Length::Fill);
	let plot_content: Element<_> = if let Some(info) = &state.hovered_info {
		Tooltip::new(
			canvas_widget,
			container(text(info))
				.padding(5)
				.style(|_| container::Style {
					background: Some(iced::Background::Color(iced::Color::from_rgba(
						0.001, 0.001, 0.001, 0.85,
					))),
					border: iced::Border {
						color: iced::Color::from_rgba(1.0, 1.0, 1.0, 0.2),
						width: 1.0,
						radius: 2.0.into(),
					},
					..Default::default()
				}),
			tooltip::Position::FollowCursor,
		)
		.into()
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
		text("Legend Rows:"),
		text_input("", &state.max_legend_rows_input)
			.on_input(|s| {
				if let Ok(rows) = s.parse::<u32>() {
					Message::SetMaxLegendRows(rows)
				} else if s.is_empty() {
					Message::SetMaxLegendRows(0)
				} else {
					Message::UpdateHover(state.hovered_info.clone())
				}
			})
			.width(50),
		text("X:"),
		text_input("", &state.legend_x_input)
			.on_input(|s| {
				if let Ok(x) = s.parse::<f32>() {
					Message::SetLegendX(x)
				} else {
					Message::UpdateHover(state.hovered_info.clone())
				}
			})
			.width(60),
		text("Y:"),
		text_input("", &state.legend_y_input)
			.on_input(|s| {
				if let Ok(y) = s.parse::<f32>() {
					Message::SetLegendY(y)
				} else {
					Message::UpdateHover(state.hovered_info.clone())
				}
			})
			.width(60),
		text("Angle:"),
		text_input("", &state.x_rotation_input)
			.on_input(|s| {
				if let Ok(deg) = s.parse::<f32>() {
					Message::SetXRotation(deg)
				} else {
					Message::UpdateHover(state.hovered_info.clone())
				}
			})
			.width(60),
		text("Offset:"),
		text_input("", &state.x_offset_input)
			.on_input(|s| {
				if let Ok(offset) = s.parse::<f32>() {
					Message::SetXOffset(offset)
				} else {
					Message::UpdateHover(state.hovered_info.clone())
				}
			})
			.width(60),
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
