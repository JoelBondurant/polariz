use crate::message::Message;
use iced::advanced::mouse::Cursor;
use iced::alignment;
use iced::widget::canvas::{self, Frame, Geometry, Path, Program, Stroke, Style, Text};
use iced::{Color, Event, Point, Rectangle, Renderer, Theme};

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
	Parallel {
		dimensions: Vec<String>,
		ranges: Vec<(f32, f32)>,
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
			let x_delta = (x_range.1 - x_range.0).abs().max(f32::EPSILON);
			let y_delta = (y_range.1 - y_range.0).abs().max(f32::EPSILON);
			let x_scale = self.bounds.width / x_delta;
			let y_scale = self.bounds.height / y_delta;
			let pixel_x = self.bounds.x + ((data_x - x_range.0) * x_scale);
			let pixel_y = self.bounds.y + self.bounds.height - ((data_y - y_range.0) * y_scale);
			Point::new(pixel_x, pixel_y)
		} else {
			Point::ORIGIN
		}
	}

	pub fn categorical(&self, category_index: usize, data_y: f32) -> (Point, f32) {
		match self.layout {
			PlotLayout::CategoricalX {
				categories,
				y_range,
			} => {
				let num_cats = categories.len().max(1) as f32;
				let band_width = self.bounds.width / num_cats;
				let center_x =
					self.bounds.x + (category_index as f32 * band_width) + (band_width / 2.0);
				let y_delta = (y_range.1 - y_range.0).abs().max(f32::EPSILON);
				let y_scale = self.bounds.height / y_delta;
				let pixel_y = self.bounds.y + self.bounds.height - ((data_y - y_range.0) * y_scale);
				(Point::new(center_x, pixel_y), band_width)
			}
			PlotLayout::Parallel { dimensions, ranges } => {
				let num_dims = dimensions.len().max(1) as f32;
				let axis_spacing = self.bounds.width / (num_dims - 1.0).max(1.0);
				let axis_x = self.bounds.x + (category_index as f32 * axis_spacing);
				let range = ranges.get(category_index).copied().unwrap_or((0.0, 1.0));
				let y_delta = (range.1 - range.0).abs().max(f32::EPSILON);
				let y_scale = self.bounds.height / y_delta;
				let pixel_y = self.bounds.y + self.bounds.height - ((data_y - range.0) * y_scale);
				(Point::new(axis_x, pixel_y), 0.0)
			}
			_ => (Point::ORIGIN, 0.0),
		}
	}

	#[allow(dead_code)]
	pub fn pixel_to_cartesian(&self, cursor_pos: Point) -> Option<(f32, f32)> {
		if let PlotLayout::Cartesian { x_range, y_range } = self.layout {
			if !self.bounds.contains(cursor_pos) {
				return None;
			}
			let x_delta = (x_range.1 - x_range.0).abs().max(f32::EPSILON);
			let y_delta = (y_range.1 - y_range.0).abs().max(f32::EPSILON);
			let x_scale = x_delta / self.bounds.width;
			let y_scale = y_delta / self.bounds.height;
			let data_x = x_range.0 + ((cursor_pos.x - self.bounds.x) * x_scale);
			let data_y =
				y_range.0 + ((self.bounds.y + self.bounds.height - cursor_pos.y) * y_scale);
			Some((data_x, data_y))
		} else {
			None
		}
	}
}

#[derive(Debug, Clone, Copy)]
pub struct LegendSettings {
	pub max_rows: u32,
	pub position_x: f32,
	pub position_y: f32,
}

impl Default for LegendSettings {
	fn default() -> Self {
		Self {
			max_rows: 4,
			position_x: 0.95,
			position_y: 0.05,
		}
	}
}

pub trait PlotKernel {
	fn layout(&self) -> PlotLayout;

	fn plot(
		&self,
		frame: &mut Frame,
		bounds: Rectangle,
		transform: &CoordinateTransformer,
		cursor: Cursor,
	);

	fn draw_legend(&self, _frame: &mut Frame, _bounds: Rectangle, _settings: LegendSettings) {}

	fn hover(&self, transform: &CoordinateTransformer, cursor: Cursor) -> Option<String>;
}

pub struct PlotWidget<'a> {
	pub kernel: &'a dyn PlotKernel,
	pub title: String,
	pub padding: f32,
	pub legend_settings: LegendSettings,
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
		let padding_top = self.padding + 50.0;
		let padding_bottom = self.padding + 40.0;
		let padding_left = self.padding + 40.0;
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
		self.kernel
			.plot(&mut frame, plot_area, &transform, relative_cursor);
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
			PlotLayout::Parallel { dimensions, ranges } => {
				self.draw_parallel_axes(&mut frame, plot_area, &transform, dimensions, ranges);
			}
			PlotLayout::Radial => {}
		}
		self.kernel
			.draw_legend(&mut frame, bounds, self.legend_settings);
		frame.fill_text(Text {
			content: self.title.clone(),
			position: Point::new(bounds.width / 2.0, 20.0),
			color: Color::WHITE,
			size: iced::Pixels(28.0),
			align_x: alignment::Horizontal::Center.into(),
			align_y: alignment::Vertical::Top,
			..Default::default()
		});
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
			let padding_top = self.padding + 50.0;
			let padding_bottom = self.padding + 40.0;
			let padding_left = self.padding + 40.0;
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
		let halo_stroke = Stroke {
			style: Style::Solid(Color::BLACK),
			width: 4.0,
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
		frame.stroke(&axes_path, halo_stroke);
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
				size: iced::Pixels(18.0),
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
				size: iced::Pixels(18.0),
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
		let halo_stroke = Stroke {
			style: Style::Solid(Color::BLACK),
			width: 4.0,
			..Default::default()
		};
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
		frame.stroke(&axes_path, halo_stroke);
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
				size: iced::Pixels(18.0),
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
				size: iced::Pixels(18.0),
				align_x: alignment::Horizontal::Center.into(),
				..Default::default()
			});
		}
	}

	fn draw_parallel_axes(
		&self,
		frame: &mut Frame,
		_area: Rectangle,
		transform: &CoordinateTransformer,
		dimensions: &[String],
		ranges: &[(f32, f32)],
	) {
		let halo_stroke = Stroke {
			style: Style::Solid(Color::BLACK),
			width: 10.0,
			..Default::default()
		};
		let axis_stroke = Stroke {
			style: Style::Solid(Color::WHITE),
			width: 4.0,
			..Default::default()
		};
		let tick_stroke = Stroke {
			style: Style::Solid(Color::from_rgba(1.0, 1.0, 1.0, 0.6)),
			width: 1.5,
			..Default::default()
		};
		let num_ticks = 5;
		for (i, dim) in dimensions.iter().enumerate() {
			let range = ranges[i];
			let (top_px, _) = transform.categorical(i, range.1);
			let (bottom_px, _) = transform.categorical(i, range.0);
			let axis_path = Path::new(|builder| {
				builder.move_to(top_px);
				builder.line_to(bottom_px);
			});
			frame.stroke(&axis_path, halo_stroke);
			frame.stroke(&axis_path, axis_stroke);
			for j in 0..=num_ticks {
				let t = j as f32 / num_ticks as f32;
				let data_y = range.0 + (range.1 - range.0) * t;
				let (p, _) = transform.categorical(i, data_y);
				let tick_path = Path::new(|builder| {
					builder.move_to(p);
					builder.line_to(Point::new(p.x - 6.0, p.y));
				});
				frame.stroke(&tick_path, tick_stroke);
				frame.fill_text(Text {
					content: format!("{:.1}", data_y),
					position: Point::new(p.x - 14.0, p.y),
					color: Color::WHITE,
					size: iced::Pixels(18.0),
					align_x: alignment::Horizontal::Right.into(),
					align_y: alignment::Vertical::Center,
					..Default::default()
				});
			}
			frame.fill_text(Text {
				content: dim.clone(),
				position: Point::new(top_px.x, top_px.y - 20.0),
				color: Color::WHITE,
				size: iced::Pixels(22.0),
				align_x: alignment::Horizontal::Center.into(),
				align_y: alignment::Vertical::Bottom,
				..Default::default()
			});
		}
	}
}
