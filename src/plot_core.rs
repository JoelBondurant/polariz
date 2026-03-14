#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PlotType {
	Bar,
	BoxPlot,
	Hexbin,
	Histogram,
	Line,
	Pie,
	Scatter,
	StackedArea,
	StackedBar,
	Violin,
}

impl PlotType {
	pub const ALL: [PlotType; 10] = [
		PlotType::Bar,
		PlotType::BoxPlot,
		PlotType::Hexbin,
		PlotType::Histogram,
		PlotType::Line,
		PlotType::Pie,
		PlotType::Scatter,
		PlotType::StackedArea,
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
			PlotType::Histogram => write!(f, "Histogram Plot"),
			PlotType::Line => write!(f, "Line Plot"),
			PlotType::Pie => write!(f, "Pie Plot"),
			PlotType::Scatter => write!(f, "Scatter Plot"),
			PlotType::StackedArea => write!(f, "Stacked Area Plot"),
			PlotType::StackedBar => write!(f, "Stacked Bar Plot"),
			PlotType::Violin => write!(f, "Violin Plot"),
		}
	}
}
