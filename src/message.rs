use crate::plot_core::PlotType;

#[derive(Clone, Debug)]
pub enum Message {
	UpdateHover(Option<String>),
	ChangePlotType(PlotType),
}
