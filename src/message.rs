#[derive(Clone, Debug)]
pub enum Message {
	RasterizationResult(u32, u32, Vec<u8>),
	UpdateHover(Option<String>),
}
