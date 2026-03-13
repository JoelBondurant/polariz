mod hexbin;
mod line;
mod message;
mod plot;
mod plot_core;
mod state;
mod violin;

fn main() -> state::Result {
	state::run()
}
