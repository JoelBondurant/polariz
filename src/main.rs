mod bar;
mod hexbin;
mod line;
mod message;
mod plot;
mod plot_core;
mod scatter;
mod state;
mod violin;

fn main() -> state::Result {
	state::run()
}
