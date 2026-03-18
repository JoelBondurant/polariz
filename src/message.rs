use crate::plot::colors::ColorTheme;
use crate::plot::core::PlotType;
use iced::Color;

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub enum Message {
	UpdateHover(Option<String>),
	ChangePlotType(PlotType),
	SetMaxLegendRows(u32),
	SetLegendX(f32),
	SetLegendY(f32),
	SetXRotation(f32),
	SetXOffset(f32),
	ChangeColorTheme(ColorTheme),
	ChangeBackgroundColor(Color),
	ChangeBackgroundHex(String),
	ChangeDecorationColor(Color),
	ChangeDecorationHex(String),
	SetXMin(Option<f64>),
	SetXMax(Option<f64>),
	SetYMin(Option<f64>),
	SetYMax(Option<f64>),
}
