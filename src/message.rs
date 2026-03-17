use crate::plot_core::PlotType;

#[derive(Clone, Debug)]
pub enum Message {
	UpdateHover(Option<String>),
	ChangePlotType(PlotType),
	SetMaxLegendRows(u32),
	SetLegendX(f32),
	SetLegendY(f32),
	SetXRotation(f32),
	SetXOffset(f32),
}
