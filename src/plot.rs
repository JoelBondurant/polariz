use crate::message::Message;
use iced::advanced::mouse::Cursor;
use iced::alignment;
use iced::widget::canvas::{self, Frame, Geometry, Path, Program, Stroke, Style, Text};
use iced::{Color, Event, Point, Rectangle, Renderer, Task, Theme};

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

	fn hover(&self, transform: &CoordinateTransformer, cursor: Cursor) -> Option<String>;

	fn rasterize(&self, width: u32, height: u32) -> Task<Message>;

	fn update_raster(&mut self, width: u32, height: u32, pixels: Vec<u8>);
}

pub struct PlotWidget<'a> {
	pub kernel: &'a dyn PlotKernel,
	pub title: String,
	pub padding: f32,
}

impl<'a> Program<Message> for PlotWidget<'a> {
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
		let padding_top = self.padding + 20.0;
		let padding_bottom = self.padding + 20.0;
		let padding_left = self.padding + 20.0;
		let padding_right = self.padding;
		let plot_area = Rectangle {
			x: padding_left,
			y: padding_top,
			width: bounds.width - padding_left - padding_right,
			height: bounds.height - padding_top - padding_bottom,
		};
		let layout = self.kernel.layout();
		let transform = CoordinateTransformer::new(&layout, plot_area);
		frame.fill_text(Text {
			content: self.title.clone(),
			position: Point::new(bounds.width / 2.0, padding_top / 2.0),
			color: Color::WHITE,
			size: iced::Pixels(20.0),
			align_x: alignment::Horizontal::Center.into(),
			align_y: alignment::Vertical::Center,
			..Default::default()
		});
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
		let relative_cursor = match cursor.position() {
			Some(pos) => Cursor::Available(Point::new(pos.x - bounds.x, pos.y - bounds.y)),
			None => Cursor::Unavailable,
		};
		self.kernel
			.draw_overlay(&mut frame, plot_area, &transform, relative_cursor);
		vec![frame.into_geometry()]
	}

	fn update(
		&self,
		_state: &mut Self::State,
		event: &Event,
		bounds: Rectangle,
		cursor: Cursor,
	) -> Option<canvas::Action<Message>> {
		if let Event::Mouse(iced::mouse::Event::CursorMoved { .. }) = event {
			let padding_top = self.padding + 20.0;
			let padding_bottom = self.padding + 20.0;
			let padding_left = self.padding + 20.0;
			let padding_right = self.padding;
			let plot_area = Rectangle {
				x: padding_left,
				y: padding_top,
				width: bounds.width - padding_left - padding_right,
				height: bounds.height - padding_top - padding_bottom,
			};
			let layout = self.kernel.layout();
			let transform = CoordinateTransformer::new(&layout, plot_area);
			let relative_cursor = match cursor.position() {
				Some(pos) => Cursor::Available(Point::new(pos.x - bounds.x, pos.y - bounds.y)),
				None => Cursor::Unavailable,
			};
			let hover = self.kernel.hover(&transform, relative_cursor);
			return Some(canvas::Action::publish(Message::UpdateHover(hover)));
		}
		None
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
		let grid_path = Path::new(|builder| {
			for i in 0..=num_ticks {
				let t = i as f32 / num_ticks as f32;
				let data_y = y_range.0 + (y_range.1 - y_range.0) * t;
				let p_left = transform.cartesian(x_range.0, data_y);
				let p_right = transform.cartesian(x_range.1, data_y);
				builder.move_to(p_left);
				builder.line_to(p_right);
			}
			for i in 0..=num_ticks {
				let t = i as f32 / num_ticks as f32;
				let data_x = x_range.0 + (x_range.1 - x_range.0) * t;
				let p_bottom = transform.cartesian(data_x, y_range.0);
				let p_top = transform.cartesian(data_x, y_range.1);
				builder.move_to(p_bottom);
				builder.line_to(p_top);
			}
		});
		frame.stroke(&grid_path, grid_stroke);
		let axes_path = Path::new(|builder| {
			let origin = transform.cartesian(x_range.0, y_range.0);
			let x_max = transform.cartesian(x_range.1, y_range.0);
			let y_max = transform.cartesian(x_range.0, y_range.1);
			builder.move_to(y_max);
			builder.line_to(origin);
			builder.line_to(x_max);
		});
		frame.stroke(&axes_path, axis_stroke);
		for i in 0..=num_ticks {
			let t = i as f32 / num_ticks as f32;
			let data_y = y_range.0 + (y_range.1 - y_range.0) * t;
			let p_left = transform.cartesian(x_range.0, data_y);
			let tick_path = Path::new(|builder| {
				builder.move_to(p_left);
				builder.line_to(Point::new(p_left.x - 5.0, p_left.y));
			});
			frame.stroke(&tick_path, axis_stroke);
			frame.fill_text(Text {
				content: format!("{:.1}", data_y),
				position: Point::new(p_left.x - 10.0, p_left.y),
				color: Color::WHITE,
				size: iced::Pixels(14.0),
				align_x: alignment::Horizontal::Right.into(),
				align_y: alignment::Vertical::Center,
				..Default::default()
			});
		}
		for i in 0..=num_ticks {
			let t = i as f32 / num_ticks as f32;
			let data_x = x_range.0 + (x_range.1 - x_range.0) * t;
			let p_bottom = transform.cartesian(data_x, y_range.0);
			let tick_path = Path::new(|builder| {
				builder.move_to(p_bottom);
				builder.line_to(Point::new(p_bottom.x, p_bottom.y + 5.0));
			});
			frame.stroke(&tick_path, axis_stroke);
			frame.fill_text(Text {
				content: format!("{:.1}", data_x),
				position: Point::new(p_bottom.x, p_bottom.y + 10.0),
				color: Color::WHITE,
				size: iced::Pixels(14.0),
				align_x: alignment::Horizontal::Center.into(),
				..Default::default()
			});
		}
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
		let axes_path = Path::new(|builder| {
			let (first_cat_center, band_width) = transform.categorical(0, y_range.1);
			let left_edge = first_cat_center.x - (band_width / 2.0);
			let top_y = first_cat_center.y;
			let (last_cat_center, _) = transform.categorical(categories.len() - 1, y_range.0);
			let right_edge = last_cat_center.x + (band_width / 2.0);
			let bottom_y = last_cat_center.y;
			builder.move_to(Point::new(left_edge, top_y));
			builder.line_to(Point::new(left_edge, bottom_y));
			builder.line_to(Point::new(right_edge, bottom_y));
		});
		frame.stroke(&axes_path, axis_stroke);
		for i in 0..=num_ticks {
			let t = i as f32 / num_ticks as f32;
			let data_y = y_range.0 + (y_range.1 - y_range.0) * t;
			let (center, band_width) = transform.categorical(0, data_y);
			let left_edge = center.x - (band_width / 2.0);
			let p_left = Point::new(left_edge, center.y);
			let tick_path = Path::new(|builder| {
				builder.move_to(p_left);
				builder.line_to(Point::new(p_left.x - 5.0, p_left.y));
			});
			frame.stroke(&tick_path, axis_stroke);
			frame.fill_text(Text {
				content: format!("{:.1}", data_y),
				position: Point::new(p_left.x - 10.0, p_left.y),
				color: Color::WHITE,
				size: iced::Pixels(14.0),
				align_x: alignment::Horizontal::Right.into(),
				align_y: alignment::Vertical::Center,
				..Default::default()
			});
		}
		for (i, cat) in categories.iter().enumerate() {
			let (center_px, _band_width) = transform.categorical(i, y_range.0);
			let tick_path = Path::new(|builder| {
				builder.move_to(center_px);
				builder.line_to(Point::new(center_px.x, center_px.y + 5.0));
			});
			frame.stroke(&tick_path, axis_stroke);
			frame.fill_text(Text {
				content: cat.clone(),
				position: Point::new(center_px.x, center_px.y + 10.0),
				color: Color::WHITE,
				size: iced::Pixels(14.0),
				align_x: alignment::Horizontal::Center.into(),
				..Default::default()
			});
		}
	}
}
