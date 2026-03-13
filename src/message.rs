#[derive(Clone, Debug)]
pub enum Message {
	RasterizationResult(u32, u32, Vec<u8>),
	UpdateHover(Option<String>),
	ChangePlotType(PlotType),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlotType {
	Violin,
	Hexbin,
}

impl PlotType {
	pub const ALL: [PlotType; 2] = [PlotType::Violin, PlotType::Hexbin];
}

impl std::fmt::Display for PlotType {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			PlotType::Violin => write!(f, "Violin Plot"),
			PlotType::Hexbin => write!(f, "Hexbin Plot"),
		}
	}
}
