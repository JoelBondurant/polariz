#[derive(Clone, Debug)]
pub enum Message {
	WgpuRasterFinished(u32, u32, Vec<u8>),
	None,
}
