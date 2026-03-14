mod bar;
mod hexbin;
mod line;
mod message;
mod pie;
mod plot;
mod plot_core;
mod scatter;
mod stacked_bar;
mod state;
mod violin;

fn main() -> state::Result {
	state::run()
}
