mod bar;
mod box_plot;
mod bubble;
mod candlestick;
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
mod radial_dial;
mod radar;
mod scatter;
mod stacked_area;
mod stacked_bar;
mod state;
mod violin;

fn main() -> state::Result {
	state::run()
}
