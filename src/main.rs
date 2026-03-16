mod bar;
mod box_plot;
mod bubble;
mod colors;
mod fill_between;
mod funnel;
mod heatmap;
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
