mod bar;
mod box_plot;
mod colors;
mod hexbin;
mod histogram;
mod line;
mod message;
mod parallel;
mod pie;
mod plot;
mod plot_core;
mod scatter;
mod stacked_area;
mod stacked_bar;
mod state;
mod violin;

fn main() -> state::Result {
	state::run()
}
