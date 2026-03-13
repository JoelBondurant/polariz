use crate::message::Message;
use iced::advanced::mouse::Cursor;
use iced::widget::canvas::{Frame, Geometry, Path, Program, Stroke, Style, Text};
use iced::{Color, Point, Rectangle, Renderer, Task, Theme};

#[allow(dead_code)]
#[derive(Clone)]
pub enum PlotLayout {
	Cartesian {
		x_range: (f32, f32),
		y_range: (f32, f32),
	},
	CategoricalX {
		categories: Vec<String>,
		y_range: (f32, f32),
	},
	Radial,
}

pub struct CoordinateTransformer<'a> {
	pub layout: &'a PlotLayout,
	pub bounds: Rectangle,
}

impl<'a> CoordinateTransformer<'a> {
	pub fn new(layout: &'a PlotLayout, bounds: Rectangle) -> Self {
		Self { layout, bounds }
	}

	pub fn cartesian(&self, data_x: f32, data_y: f32) -> Point {
		if let PlotLayout::Cartesian { x_range, y_range } = self.layout {
			let x_scale = self.bounds.width / (x_range.1 - x_range.0);
			let y_scale = self.bounds.height / (y_range.1 - y_range.0);
			let pixel_x = self.bounds.x + ((data_x - x_range.0) * x_scale);
			let pixel_y = self.bounds.y + self.bounds.height - ((data_y - y_range.0) * y_scale);
			Point::new(pixel_x, pixel_y)
		} else {
			Point::ORIGIN
		}
	}

	pub fn categorical(&self, category_index: usize, data_y: f32) -> (Point, f32) {
		if let PlotLayout::CategoricalX {
			categories,
			y_range,
		} = self.layout
		{
			let num_cats = categories.len() as f32;
			let band_width = self.bounds.width / num_cats;
			let center_x =
				self.bounds.x + (category_index as f32 * band_width) + (band_width / 2.0);
			let y_scale = self.bounds.height / (y_range.1 - y_range.0);
			let pixel_y = self.bounds.y + self.bounds.height - ((data_y - y_range.0) * y_scale);
			(Point::new(center_x, pixel_y), band_width)
		} else {
			(Point::ORIGIN, 0.0)
		}
	}

	#[allow(dead_code)]
	pub fn pixel_to_cartesian(&self, cursor_pos: Point) -> Option<(f32, f32)> {
		if let PlotLayout::Cartesian { x_range, y_range } = self.layout {
			if !self.bounds.contains(cursor_pos) {
				return None;
			}
			let x_scale = (x_range.1 - x_range.0) / self.bounds.width;
			let y_scale = (y_range.1 - y_range.0) / self.bounds.height;
			let data_x = x_range.0 + ((cursor_pos.x - self.bounds.x) * x_scale);
			let data_y =
				y_range.0 + ((self.bounds.y + self.bounds.height - cursor_pos.y) * y_scale);
			Some((data_x, data_y))
		} else {
			None
		}
	}
}

pub trait PlotKernel {
	fn layout(&self) -> PlotLayout;

	fn draw_raster(&self, frame: &mut Frame, bounds: Rectangle, transform: &CoordinateTransformer);

	fn draw_overlay(
		&self,
		frame: &mut Frame,
		bounds: Rectangle,
		transform: &CoordinateTransformer,
		cursor: Cursor,
	);

	fn rasterize(&self, width: u32, height: u32) -> Task<Message>;

	fn update_raster(&mut self, width: u32, height: u32, pixels: Vec<u8>);
}

pub struct PlotWidget<'a> {
	pub kernel: &'a dyn PlotKernel,
	pub padding: f32,
}

impl<'a> Program<()> for PlotWidget<'a> {
	type State = ();

	fn draw(
		&self,
		_state: &(),
		renderer: &Renderer,
		_theme: &Theme,
		bounds: Rectangle,
		cursor: Cursor,
	) -> Vec<Geometry> {
		let mut frame = Frame::new(renderer, bounds.size());
		let plot_area = Rectangle {
			x: bounds.x + self.padding,
			y: bounds.y + self.padding,
			width: bounds.width - (self.padding * 2.0),
			height: bounds.height - (self.padding * 2.0),
		};
		let layout = self.kernel.layout();
		let transform = CoordinateTransformer::new(&layout, plot_area);
		match &layout {
			PlotLayout::Cartesian { x_range, y_range } => {
				self.draw_cartesian_grid(&mut frame, plot_area, &transform, *x_range, *y_range);
			}
			PlotLayout::CategoricalX {
				categories,
				y_range,
			} => {
				self.draw_categorical_axes(&mut frame, plot_area, &transform, categories, *y_range);
			}
			PlotLayout::Radial => {}
		}
		self.kernel.draw_raster(&mut frame, plot_area, &transform);
		self.kernel
			.draw_overlay(&mut frame, plot_area, &transform, cursor);
		vec![frame.into_geometry()]
	}
}

impl<'a> PlotWidget<'a> {
	fn draw_cartesian_grid(
		&self,
		frame: &mut Frame,
		_area: Rectangle,
		transform: &CoordinateTransformer,
		x_range: (f32, f32),
		y_range: (f32, f32),
	) {
		let grid_stroke = Stroke {
			style: Style::Solid(Color::from_rgba(0.5, 0.5, 0.5, 0.2)),
			width: 1.0,
			..Default::default()
		};
		let axis_stroke = Stroke {
			style: Style::Solid(Color::WHITE),
			width: 2.0,
			..Default::default()
		};
		let num_ticks = 8;
		let path = Path::new(|builder| {
			for i in 0..=num_ticks {
				let t = i as f32 / num_ticks as f32;
				let data_y = y_range.0 + (y_range.1 - y_range.0) * t;
				let p_left = transform.cartesian(x_range.0, data_y);
				let p_right = transform.cartesian(x_range.1, data_y);
				builder.move_to(p_left);
				builder.line_to(p_right);
				frame.fill_text(Text {
					content: format!("{:.1}", data_y),
					position: Point::new(p_left.x - 30.0, p_left.y - 8.0),
					color: Color::WHITE,
					size: iced::Pixels(14.0),
					..Default::default()
				});
			}
			for i in 0..=num_ticks {
				let t = i as f32 / num_ticks as f32;
				let data_x = x_range.0 + (x_range.1 - x_range.0) * t;
				let p_bottom = transform.cartesian(data_x, y_range.0);
				let p_top = transform.cartesian(data_x, y_range.1);
				builder.move_to(p_bottom);
				builder.line_to(p_top);
				frame.fill_text(Text {
					content: format!("{:.1}", data_x),
					position: Point::new(p_bottom.x - 10.0, p_bottom.y + 10.0),
					color: Color::WHITE,
					size: iced::Pixels(14.0),
					..Default::default()
				});
			}
		});
		frame.stroke(&path, grid_stroke);
		let axes_path = Path::new(|builder| {
			let origin = transform.cartesian(x_range.0, y_range.0);
			let x_max = transform.cartesian(x_range.1, y_range.0);
			let y_max = transform.cartesian(x_range.0, y_range.1);
			builder.move_to(y_max);
			builder.line_to(origin);
			builder.line_to(x_max);
		});
		frame.stroke(&axes_path, axis_stroke);
	}

	fn draw_categorical_axes(
		&self,
		frame: &mut Frame,
		_area: Rectangle,
		transform: &CoordinateTransformer,
		categories: &[String],
		y_range: (f32, f32),
	) {
		let axis_stroke = Stroke {
			style: Style::Solid(Color::WHITE),
			width: 2.0,
			..Default::default()
		};
		let num_ticks = 8;
		let y_path = Path::new(|builder| {
			for i in 0..=num_ticks {
				let t = i as f32 / num_ticks as f32;
				let data_y = y_range.0 + (y_range.1 - y_range.0) * t;
				let (first_cat_center, band_width) = transform.categorical(0, data_y);
				let left_edge = first_cat_center.x - (band_width / 2.0);
				let p_left = Point::new(left_edge, first_cat_center.y);
				builder.move_to(p_left);
				builder.line_to(Point::new(p_left.x + 5.0, p_left.y)); // Just a tick mark
				frame.fill_text(Text {
					content: format!("{:.1}", data_y),
					position: Point::new(p_left.x - 30.0, p_left.y - 8.0),
					color: Color::WHITE,
					size: iced::Pixels(14.0),
					..Default::default()
				});
			}
		});
		frame.stroke(&y_path, axis_stroke);
		for (i, cat) in categories.iter().enumerate() {
			let (center_px, _) = transform.categorical(i, y_range.0);
			frame.fill_text(Text {
				content: cat.clone(),
				position: Point::new(center_px.x - 15.0, center_px.y + 10.0),
				color: Color::WHITE,
				size: iced::Pixels(14.0),
				..Default::default()
			});
		}
	}
}
