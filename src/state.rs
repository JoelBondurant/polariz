use crate::message::Message;
use crate::plot::colors::{self, ColorTheme};
use crate::plot::common::{Orientation, PlotKernel, PlotSettings, PlotWidget};
use crate::plot::core::PlotType;
use crate::plot::kernels::bar::{self, BarPlotKernel};
use crate::plot::kernels::boxplot::{self, BoxPlotKernel};
use crate::plot::kernels::bubble::{self, BubblePlotKernel};
use crate::plot::kernels::candlestick::{self, CandlestickPlotKernel};
use crate::plot::kernels::fill_between::{self, FillBetweenPlotKernel};
use crate::plot::kernels::funnel::{self, FunnelPlotKernel};
use crate::plot::kernels::heatmap::{self, HeatmapPlotKernel};
use crate::plot::kernels::hexbin::{self, HexbinPlotKernel};
use crate::plot::kernels::histogram::{self, HistogramPlotKernel};
use crate::plot::kernels::line::{self, LinePlotKernel};
use crate::plot::kernels::parallel::{self, ParallelPlotKernel};
use crate::plot::kernels::pie::{self, PiePlotKernel};
use crate::plot::kernels::radar::{self, RadarPlotKernel};
use crate::plot::kernels::radial_dial::{self, RadialDialPlotKernel};
use crate::plot::kernels::scatter::{self, ScatterPlotKernel};
use crate::plot::kernels::stacked_area::{self, StackedAreaPlotKernel};
use crate::plot::kernels::stacked_bar::{self, StackedBarPlotKernel};
use crate::plot::kernels::violin::{self, ViolinPlotKernel};
use iced::widget::{
	button, canvas, column, container, opaque, pick_list, row, scrollable, space, stack, text,
	text_input, tooltip, Tooltip,
};
use iced::{window, Alignment, Element, Length, Size, Task};
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
	bg_color_input: String,
	decoration_color_input: String,
	x_min_input: String,
	x_max_input: String,
	y_min_input: String,
	y_max_input: String,
	title_input: String,
	subtitle_input: String,
	x_label_input: String,
	y_label_input: String,
	title_offset_input: String,
	subtitle_offset_input: String,
	x_label_padding_input: String,
	y_label_padding_input: String,
	plot_padding_top_input: String,
	plot_padding_bottom_input: String,
	plot_padding_left_input: String,
	plot_padding_right_input: String,
	title_size_input: String,
	subtitle_size_input: String,
	x_label_size_input: String,
	y_label_size_input: String,
	x_tick_size_input: String,
	y_tick_size_input: String,
	legend_size_input: String,
	x_ticks_input: String,
	y_ticks_input: String,
	settings_open: bool,
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
		bg_color_input: colors::color_to_hex(plot_settings.background_color),
		decoration_color_input: colors::color_to_hex(plot_settings.decoration_color),
		x_min_input: String::new(),
		x_max_input: String::new(),
		y_min_input: String::new(),
		y_max_input: String::new(),
		title_input: String::new(),
		subtitle_input: String::new(),
		x_label_input: String::new(),
		y_label_input: String::new(),
		title_offset_input: plot_settings.title_offset.to_string(),
		subtitle_offset_input: plot_settings.subtitle_offset.to_string(),
		x_label_padding_input: plot_settings.x_label_padding.to_string(),
		y_label_padding_input: plot_settings.y_label_padding.to_string(),
		plot_padding_top_input: plot_settings.plot_padding_top.to_string(),
		plot_padding_bottom_input: plot_settings.plot_padding_bottom.to_string(),
		plot_padding_left_input: plot_settings.plot_padding_left.to_string(),
		plot_padding_right_input: plot_settings.plot_padding_right.to_string(),
		title_size_input: plot_settings.title_size.to_string(),
		subtitle_size_input: plot_settings.subtitle_size.to_string(),
		x_label_size_input: plot_settings.x_label_size.to_string(),
		y_label_size_input: plot_settings.y_label_size.to_string(),
		x_tick_size_input: plot_settings.x_tick_size.to_string(),
		y_tick_size_input: plot_settings.y_tick_size.to_string(),
		legend_size_input: plot_settings.legend_size.to_string(),
		x_ticks_input: plot_settings.x_ticks.to_string(),
		y_ticks_input: plot_settings.y_ticks.to_string(),
		plot_settings: plot_settings.clone(),
		max_legend_rows_input: plot_settings.max_legend_rows.to_string(),
		legend_x_input: plot_settings.legend_x.to_string(),
		legend_y_input: plot_settings.legend_y.to_string(),
		x_rotation_input: plot_settings.x_label_rotation.to_string(),
		x_offset_input: plot_settings.x_label_offset.to_string(),
		settings_open: false,
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
				orientation: Orientation::Vertical,
			})
		}
		PlotType::HorizontalBar => {
			let df = bar::generate_sample_bar_data();
			let prepared = bar::prepare_bar_data(&df, "cat", "group", "val");
			Box::new(BarPlotKernel {
				prepared_data: Arc::new(prepared),
				orientation: Orientation::Horizontal,
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
				orientation: Orientation::Vertical,
			})
		}
		PlotType::HorizontalStackedBar => {
			let df = stacked_bar::generate_sample_stacked_bar_data();
			let prepared = stacked_bar::prepare_stacked_bar_data(&df, "cat", "group", "val");
			Box::new(StackedBarPlotKernel {
				prepared_data: Arc::new(prepared),
				orientation: Orientation::Horizontal,
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
			let prepared = boxplot::prepare_box_plot_data(&df, "group", "y");
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
		Message::ChangeColorTheme(theme) => {
			state.plot_settings.color_theme = theme;
			Task::none()
		}
		Message::ChangeBackgroundColor(color) => {
			state.plot_settings.background_color = color;
			state.bg_color_input = colors::color_to_hex(color);
			state.plot_settings.decoration_color = colors::contrast_color(color);
			state.decoration_color_input =
				colors::color_to_hex(state.plot_settings.decoration_color);
			Task::none()
		}
		Message::ChangeBackgroundHex(hex) => {
			state.bg_color_input = hex.clone();
			if let Some(color) = colors::hex_to_color(&hex) {
				state.plot_settings.background_color = color;
				state.plot_settings.decoration_color = colors::contrast_color(color);
				state.decoration_color_input =
					colors::color_to_hex(state.plot_settings.decoration_color);
			}
			Task::none()
		}
		Message::ChangeDecorationColor(color) => {
			state.plot_settings.decoration_color = color;
			state.decoration_color_input = colors::color_to_hex(color);
			Task::none()
		}
		Message::ChangeDecorationHex(hex) => {
			state.decoration_color_input = hex.clone();
			if let Some(color) = colors::hex_to_color(&hex) {
				state.plot_settings.decoration_color = color;
			}
			Task::none()
		}
		Message::SetXMin(val) => {
			state.plot_settings.x_min = val;
			state.x_min_input = val.map(|v| v.to_string()).unwrap_or_default();
			Task::none()
		}
		Message::SetXMax(val) => {
			state.plot_settings.x_max = val;
			state.x_max_input = val.map(|v| v.to_string()).unwrap_or_default();
			Task::none()
		}
		Message::SetYMin(val) => {
			state.plot_settings.y_min = val;
			state.y_min_input = val.map(|v| v.to_string()).unwrap_or_default();
			Task::none()
		}
		Message::SetYMax(val) => {
			state.plot_settings.y_max = val;
			state.y_max_input = val.map(|v| v.to_string()).unwrap_or_default();
			Task::none()
		}
		Message::SetTitle(val) => {
			state.plot_settings.title = val.as_ref().map(|s| Arc::new(s.clone()));
			state.title_input = val.unwrap_or_default();
			Task::none()
		}
		Message::SetSubtitle(val) => {
			state.plot_settings.subtitle = val.as_ref().map(|s| Arc::new(s.clone()));
			state.subtitle_input = val.unwrap_or_default();
			Task::none()
		}
		Message::SetXLabel(val) => {
			state.plot_settings.x_label = val.as_ref().map(|s| Arc::new(s.clone()));
			state.x_label_input = val.unwrap_or_default();
			Task::none()
		}
		Message::SetYLabel(val) => {
			state.plot_settings.y_label = val.as_ref().map(|s| Arc::new(s.clone()));
			state.y_label_input = val.unwrap_or_default();
			Task::none()
		}
		Message::SetTitleOffset(val) => {
			state.plot_settings.title_offset = val;
			state.title_offset_input = val.to_string();
			Task::none()
		}
		Message::SetSubtitleOffset(val) => {
			state.plot_settings.subtitle_offset = val;
			state.subtitle_offset_input = val.to_string();
			Task::none()
		}
		Message::SetXLabelPadding(val) => {
			state.plot_settings.x_label_padding = val;
			state.x_label_padding_input = val.to_string();
			Task::none()
		}
		Message::SetYLabelPadding(val) => {
			state.plot_settings.y_label_padding = val;
			state.y_label_padding_input = val.to_string();
			Task::none()
		}
		Message::SetPlotPaddingTop(val) => {
			state.plot_settings.plot_padding_top = val;
			state.plot_padding_top_input = val.to_string();
			Task::none()
		}
		Message::SetPlotPaddingBottom(val) => {
			state.plot_settings.plot_padding_bottom = val;
			state.plot_padding_bottom_input = val.to_string();
			Task::none()
		}
		Message::SetPlotPaddingLeft(val) => {
			state.plot_settings.plot_padding_left = val;
			state.plot_padding_left_input = val.to_string();
			Task::none()
		}
		Message::SetPlotPaddingRight(val) => {
			state.plot_settings.plot_padding_right = val;
			state.plot_padding_right_input = val.to_string();
			Task::none()
		}
		Message::SetTitleSize(val) => {
			state.plot_settings.title_size = val;
			state.title_size_input = val.to_string();
			Task::none()
		}
		Message::SetSubtitleSize(val) => {
			state.plot_settings.subtitle_size = val;
			state.subtitle_size_input = val.to_string();
			Task::none()
		}
		Message::SetXLabelSize(val) => {
			state.plot_settings.x_label_size = val;
			state.x_label_size_input = val.to_string();
			Task::none()
		}
		Message::SetYLabelSize(val) => {
			state.plot_settings.y_label_size = val;
			state.y_label_size_input = val.to_string();
			Task::none()
		}
		Message::SetXTickSize(val) => {
			state.plot_settings.x_tick_size = val;
			state.x_tick_size_input = val.to_string();
			Task::none()
		}
		Message::SetYTickSize(val) => {
			state.plot_settings.y_tick_size = val;
			state.y_tick_size_input = val.to_string();
			Task::none()
		}
		Message::SetLegendSize(val) => {
			state.plot_settings.legend_size = val;
			state.legend_size_input = val.to_string();
			Task::none()
		}
		Message::SetXTicks(val) => {
			state.plot_settings.x_ticks = val;
			state.x_ticks_input = val.to_string();
			Task::none()
		}
		Message::SetYTicks(val) => {
			state.plot_settings.y_ticks = val;
			state.y_ticks_input = val.to_string();
			Task::none()
		}
		Message::ToggleSettings => {
			state.settings_open = !state.settings_open;
			Task::none()
		}
		Message::CloseSettings => {
			state.settings_open = false;
			Task::none()
		}
	}
}

fn view(state: &AppState) -> Element<'_, Message> {
	let canvas_widget = canvas(PlotWidget {
		kernel: state.kernel.as_ref(),
		title: state.current_plot_type.to_string(),
		padding: 50.0,
		settings: state.plot_settings.clone(),
	})
	.width(Length::Fill)
	.height(Length::Fill);
	let plot_content: Element<_> = if let Some(info) = &state.hovered_info {
		Tooltip::new(
			canvas_widget,
			container(text(info))
				.padding(5)
				.style(|_| container::Style {
					background: Some(iced::Background::Color(iced::Color {
						a: 0.85,
						..state.plot_settings.background_color
					})),
					border: iced::Border {
						color: iced::Color {
							a: 0.2,
							..state.plot_settings.decoration_color
						},
						width: 1.0,
						radius: 2.0.into(),
					},
					text_color: Some(state.plot_settings.decoration_color),
					..Default::default()
				}),
			tooltip::Position::FollowCursor,
		)
		.into()
	} else {
		canvas_widget.into()
	};
	let mut main_stack = stack![plot_content];
	if state.settings_open {
		let settings_panel = container(
			column![
				row![
					text("Plot Settings").size(24),
					space::horizontal(),
					button("Close").on_press(Message::CloseSettings)
				]
				.align_y(Alignment::Center),
				scrollable(
					column![
						section(
							"General",
							column![
								field(
									"Plot Type",
									pick_list(
										&PlotType::ALL[..],
										Some(state.current_plot_type),
										Message::ChangePlotType
									)
								),
								field(
									"Theme",
									pick_list(
										&ColorTheme::ALL[..],
										Some(state.plot_settings.color_theme),
										Message::ChangeColorTheme
									)
								),
							]
						),
						section(
							"Titles",
							column![
								field(
									"Title",
									text_input("auto", &state.title_input).on_input(|s| {
										if s.is_empty() {
											Message::SetTitle(None)
										} else {
											Message::SetTitle(Some(s))
										}
									})
								),
								field(
									"Title Size",
									text_input("", &state.title_size_input).on_input(|s| {
										if let Ok(val) = s.parse::<f32>() {
											Message::SetTitleSize(val)
										} else {
											Message::UpdateHover(state.hovered_info.clone())
										}
									})
								),
								field(
									"Title Offset",
									text_input("", &state.title_offset_input).on_input(|s| {
										if let Ok(val) = s.parse::<f32>() {
											Message::SetTitleOffset(val)
										} else {
											Message::UpdateHover(state.hovered_info.clone())
										}
									})
								),
								field(
									"Subtitle",
									text_input("none", &state.subtitle_input).on_input(|s| {
										if s.is_empty() {
											Message::SetSubtitle(None)
										} else {
											Message::SetSubtitle(Some(s))
										}
									})
								),
								field(
									"Subtitle Size",
									text_input("", &state.subtitle_size_input).on_input(|s| {
										if let Ok(val) = s.parse::<f32>() {
											Message::SetSubtitleSize(val)
										} else {
											Message::UpdateHover(state.hovered_info.clone())
										}
									})
								),
								field(
									"Subtitle Offset",
									text_input("", &state.subtitle_offset_input).on_input(|s| {
										if let Ok(val) = s.parse::<f32>() {
											Message::SetSubtitleOffset(val)
										} else {
											Message::UpdateHover(state.hovered_info.clone())
										}
									})
								),
							]
						),
						section(
							"Axis Labels",
							column![
								field(
									"X Label",
									text_input("auto", &state.x_label_input).on_input(|s| {
										if s.is_empty() {
											Message::SetXLabel(None)
										} else {
											Message::SetXLabel(Some(s))
										}
									})
								),
								field(
									"X Label Size",
									text_input("", &state.x_label_size_input).on_input(|s| {
										if let Ok(val) = s.parse::<f32>() {
											Message::SetXLabelSize(val)
										} else {
											Message::UpdateHover(state.hovered_info.clone())
										}
									})
								),
								field("X Padding", text_input("", &state.x_label_padding_input)
									.on_input(|s| {
										if let Ok(val) = s.parse::<f32>() { Message::SetXLabelPadding(val) }
										else { Message::UpdateHover(state.hovered_info.clone()) }
									})),
								field("X Ticks", text_input("", &state.x_ticks_input)
									.on_input(|s| {
										if let Ok(val) = s.parse::<u32>() { Message::SetXTicks(val) }
										else { Message::UpdateHover(state.hovered_info.clone()) }
									})),

								field(
									"X Tick Size",
									text_input("", &state.x_tick_size_input).on_input(|s| {
										if let Ok(val) = s.parse::<f32>() {
											Message::SetXTickSize(val)
										} else {
											Message::UpdateHover(state.hovered_info.clone())
										}
									})
								),
								horizontal_rule(),
								field(
									"Y Label",
									text_input("auto", &state.y_label_input).on_input(|s| {
										if s.is_empty() {
											Message::SetYLabel(None)
										} else {
											Message::SetYLabel(Some(s))
										}
									})
								),
								field(
									"Y Label Size",
									text_input("", &state.y_label_size_input).on_input(|s| {
										if let Ok(val) = s.parse::<f32>() {
											Message::SetYLabelSize(val)
										} else {
											Message::UpdateHover(state.hovered_info.clone())
										}
									})
								),
								field("Y Padding", text_input("", &state.y_label_padding_input)
									.on_input(|s| {
										if let Ok(val) = s.parse::<f32>() { Message::SetYLabelPadding(val) }
										else { Message::UpdateHover(state.hovered_info.clone()) }
									})),
								field("Y Ticks", text_input("", &state.y_ticks_input)
									.on_input(|s| {
										if let Ok(val) = s.parse::<u32>() { Message::SetYTicks(val) }
										else { Message::UpdateHover(state.hovered_info.clone()) }
									})),

								field(
									"Y Tick Size",
									text_input("", &state.y_tick_size_input).on_input(|s| {
										if let Ok(val) = s.parse::<f32>() {
											Message::SetYTickSize(val)
										} else {
											Message::UpdateHover(state.hovered_info.clone())
										}
									})
								),
							]
						),
						section(
							"Plot Padding",
							column![
								field(
									"Top",
									text_input("", &state.plot_padding_top_input).on_input(|s| {
										if let Ok(val) = s.parse::<f32>() {
											Message::SetPlotPaddingTop(val)
										} else {
											Message::UpdateHover(state.hovered_info.clone())
										}
									})
								),
								field(
									"Bottom",
									text_input("", &state.plot_padding_bottom_input).on_input(
										|s| {
											if let Ok(val) = s.parse::<f32>() {
												Message::SetPlotPaddingBottom(val)
											} else {
												Message::UpdateHover(state.hovered_info.clone())
											}
										}
									)
								),
								field(
									"Left",
									text_input("", &state.plot_padding_left_input).on_input(|s| {
										if let Ok(val) = s.parse::<f32>() {
											Message::SetPlotPaddingLeft(val)
										} else {
											Message::UpdateHover(state.hovered_info.clone())
										}
									})
								),
								field(
									"Right",
									text_input("", &state.plot_padding_right_input).on_input(|s| {
										if let Ok(val) = s.parse::<f32>() {
											Message::SetPlotPaddingRight(val)
										} else {
											Message::UpdateHover(state.hovered_info.clone())
										}
									})
								),
							]
						),
						section(
							"Colors",
							column![
								field(
									"Background",
									text_input("", &state.bg_color_input)
										.on_input(Message::ChangeBackgroundHex)
								),
								field(
									"Decoration",
									text_input("", &state.decoration_color_input)
										.on_input(Message::ChangeDecorationHex)
								),
							]
						),
						section(
							"Legend",
							column![
								field(
									"Max Rows",
									text_input("", &state.max_legend_rows_input).on_input(|s| {
										if let Ok(rows) = s.parse::<u32>() {
											Message::SetMaxLegendRows(rows)
										} else if s.is_empty() {
											Message::SetMaxLegendRows(0)
										} else {
											Message::UpdateHover(state.hovered_info.clone())
										}
									})
								),
								field(
									"Legend Size",
									text_input("", &state.legend_size_input).on_input(|s| {
										if let Ok(val) = s.parse::<f32>() {
											Message::SetLegendSize(val)
										} else {
											Message::UpdateHover(state.hovered_info.clone())
										}
									})
								),
								field(
									"X (0-1)",
									text_input("", &state.legend_x_input).on_input(|s| {
										if let Ok(x) = s.parse::<f32>() {
											Message::SetLegendX(x)
										} else {
											Message::UpdateHover(state.hovered_info.clone())
										}
									})
								),
								field(
									"Y (0-1)",
									text_input("", &state.legend_y_input).on_input(|s| {
										if let Ok(y) = s.parse::<f32>() {
											Message::SetLegendY(y)
										} else {
											Message::UpdateHover(state.hovered_info.clone())
										}
									})
								),
							]
						),
						section(
							"X Axis Labels",
							column![
								field(
									"Rotation",
									text_input("", &state.x_rotation_input).on_input(|s| {
										if let Ok(deg) = s.parse::<f32>() {
											Message::SetXRotation(deg)
										} else {
											Message::UpdateHover(state.hovered_info.clone())
										}
									})
								),
								field(
									"Offset",
									text_input("", &state.x_offset_input).on_input(|s| {
										if let Ok(offset) = s.parse::<f32>() {
											Message::SetXOffset(offset)
										} else {
											Message::UpdateHover(state.hovered_info.clone())
										}
									})
								),
							]
						),
						section(
							"X Axis Range",
							column![
								field(
									"Min",
									text_input("auto", &state.x_min_input).on_input(|s| {
										if let Ok(val) = s.parse::<f64>() {
											Message::SetXMin(Some(val))
										} else if s.is_empty() {
											Message::SetXMin(None)
										} else {
											Message::UpdateHover(state.hovered_info.clone())
										}
									})
								),
								field(
									"Max",
									text_input("auto", &state.x_max_input).on_input(|s| {
										if let Ok(val) = s.parse::<f64>() {
											Message::SetXMax(Some(val))
										} else if s.is_empty() {
											Message::SetXMax(None)
										} else {
											Message::UpdateHover(state.hovered_info.clone())
										}
									})
								),
							]
						),
						section(
							"Y Axis Range",
							column![
								field(
									"Min",
									text_input("auto", &state.y_min_input).on_input(|s| {
										if let Ok(val) = s.parse::<f64>() {
											Message::SetYMin(Some(val))
										} else if s.is_empty() {
											Message::SetYMin(None)
										} else {
											Message::UpdateHover(state.hovered_info.clone())
										}
									})
								),
								field(
									"Max",
									text_input("auto", &state.y_max_input).on_input(|s| {
										if let Ok(val) = s.parse::<f64>() {
											Message::SetYMax(Some(val))
										} else if s.is_empty() {
											Message::SetYMax(None)
										} else {
											Message::UpdateHover(state.hovered_info.clone())
										}
									})
								),
							]
						),
					]
					.spacing(20)
				)
			]
			.spacing(20)
			.padding(20),
		)
		.width(400)
		.height(Length::Fill)
		.style(|_| container::Style {
			background: Some(iced::Background::Color(iced::Color {
				a: 0.95,
				..state.plot_settings.background_color
			})),
			border: iced::Border {
				color: state.plot_settings.decoration_color,
				width: 1.0,
				radius: 0.0.into(),
			},
			text_color: Some(state.plot_settings.decoration_color),
			..Default::default()
		});

		let modal_overlay = container(opaque(
			row![space::horizontal(), settings_panel].width(Length::Fill),
		))
		.width(Length::Fill)
		.height(Length::Fill)
		.style(|_| container::Style {
			background: Some(iced::Background::Color(iced::Color {
				a: 0.2,
				..iced::Color::BLACK
			})),
			..Default::default()
		});
		main_stack = main_stack.push(modal_overlay);
	}
	let container_style = {
		let bg = state.plot_settings.background_color;
		let decor = state.plot_settings.decoration_color;
		move |_theme: &iced::Theme| container::Style {
			background: Some(iced::Background::Color(bg)),
			text_color: Some(decor),
			..Default::default()
		}
	};
	container(main_stack.width(Length::Fill).height(Length::Fill))
		.style(container_style)
		.into()
}

fn section<'a>(title: &'a str, content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
	column![
		text(title).size(18),
		container(content).padding(10).style(|_| container::Style {
			border: iced::Border {
				color: iced::Color {
					a: 0.2,
					r: 0.5,
					g: 0.5,
					b: 0.5
				},
				width: 1.0,
				radius: 4.0.into(),
			},
			..Default::default()
		})
	]
	.spacing(8)
	.into()
}

fn field<'a>(label: &'a str, widget: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
	row![text(label).width(Length::Fixed(120.0)), widget.into()]
		.spacing(10)
		.align_y(Alignment::Center)
		.into()
}

fn horizontal_rule<'a>() -> Element<'a, Message> {
	container(row![].width(Length::Fill).height(1))
		.style(|_| container::Style {
			background: Some(iced::Background::Color(iced::Color {
				a: 0.1,
				..iced::Color::WHITE
			})),
			..Default::default()
		})
		.into()
}
