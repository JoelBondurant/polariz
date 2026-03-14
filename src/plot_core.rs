#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PlotType {
	Bar,
	BoxPlot,
	Hexbin,
	Line,
	Pie,
	Scatter,
	StackedBar,
	Violin,
}

impl PlotType {
	pub const ALL: [PlotType; 8] = [
		PlotType::Bar,
		PlotType::BoxPlot,
		PlotType::Hexbin,
		PlotType::Line,
		PlotType::Pie,
		PlotType::Scatter,
		PlotType::StackedBar,
		PlotType::Violin,
	];
}

impl std::fmt::Display for PlotType {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			PlotType::Bar => write!(f, "Bar Plot"),
			PlotType::BoxPlot => write!(f, "Box Plot"),
			PlotType::Hexbin => write!(f, "Hexbin Plot"),
			PlotType::Line => write!(f, "Line Plot"),
			PlotType::Pie => write!(f, "Pie Chart"),
			PlotType::Scatter => write!(f, "Scatter Plot"),
			PlotType::StackedBar => write!(f, "Stacked Bar Plot"),
			PlotType::Violin => write!(f, "Violin Plot"),
		}
	}
}
