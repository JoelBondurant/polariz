#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PlotType {
	Bar,
	HorizontalBar,
	BoxPlot,
	Bubble,
	FillBetween,
	Funnel,
	Heatmap,
	Hexbin,
	Histogram,
	Line,
	Parallel,
	Pie,
	Scatter,
	StackedArea,
	StackedBar,
	HorizontalStackedBar,
	Violin,
}

impl PlotType {
	pub const ALL: [PlotType; 17] = [
		PlotType::Bar,
		PlotType::HorizontalBar,
		PlotType::BoxPlot,
		PlotType::Bubble,
		PlotType::FillBetween,
		PlotType::Funnel,
		PlotType::Heatmap,
		PlotType::Hexbin,
		PlotType::Histogram,
		PlotType::Line,
		PlotType::Parallel,
		PlotType::Pie,
		PlotType::Scatter,
		PlotType::StackedArea,
		PlotType::StackedBar,
		PlotType::HorizontalStackedBar,
		PlotType::Violin,
	];
}

impl std::fmt::Display for PlotType {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			PlotType::Bar => write!(f, "Bar Plot"),
			PlotType::HorizontalBar => write!(f, "Horizontal Bar Plot"),
			PlotType::BoxPlot => write!(f, "Box Plot"),
			PlotType::Bubble => write!(f, "Bubble Plot"),
			PlotType::FillBetween => write!(f, "Fill Between Plot"),
			PlotType::Funnel => write!(f, "Funnel Plot"),
			PlotType::Heatmap => write!(f, "Heatmap"),
			PlotType::Hexbin => write!(f, "Hexbin Plot"),
			PlotType::Histogram => write!(f, "Histogram Plot"),
			PlotType::Line => write!(f, "Line Plot"),
			PlotType::Parallel => write!(f, "Parallel Coordinates Plot"),
			PlotType::Pie => write!(f, "Pie Plot"),
			PlotType::Scatter => write!(f, "Scatter Plot"),
			PlotType::StackedArea => write!(f, "Stacked Area Plot"),
			PlotType::StackedBar => write!(f, "Stacked Bar Plot"),
			PlotType::HorizontalStackedBar => write!(f, "Horizontal Stacked Bar Plot"),
			PlotType::Violin => write!(f, "Violin Plot"),
		}
	}
}
