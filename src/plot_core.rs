#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlotType {
	Violin,
	Hexbin,
	Line,
}

impl PlotType {
	pub const ALL: [PlotType; 3] = [PlotType::Violin, PlotType::Hexbin, PlotType::Line];
}

impl std::fmt::Display for PlotType {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			PlotType::Violin => write!(f, "Violin Plot"),
			PlotType::Hexbin => write!(f, "Hexbin Plot"),
			PlotType::Line => write!(f, "Line Plot"),
		}
	}
}
