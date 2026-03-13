#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlotType {
	Bar,
	Hexbin,
	Line,
	Scatter,
	Violin,
}

impl PlotType {
	pub const ALL: [PlotType; 5] = [
		PlotType::Bar,
		PlotType::Hexbin,
		PlotType::Line,
		PlotType::Scatter,
		PlotType::Violin,
	];
}

impl std::fmt::Display for PlotType {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			PlotType::Bar => write!(f, "Bar Plot"),
			PlotType::Hexbin => write!(f, "Hexbin Plot"),
			PlotType::Line => write!(f, "Line Plot"),
			PlotType::Scatter => write!(f, "Scatter Plot"),
			PlotType::Violin => write!(f, "Violin Plot"),
		}
	}
}
