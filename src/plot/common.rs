use crate::message::Message;
use crate::plot::colors::ColorTheme;
use iced::advanced::mouse::Cursor;
use iced::alignment;
use iced::widget::canvas::{self, Frame, Geometry, Path, Program, Stroke, Style, Text};
use iced::{Color, Event, Point, Rectangle, Renderer, Theme};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Orientation {
	Vertical,
	Horizontal,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TimeUnit {
	Nanoseconds,
	Microseconds,
	Milliseconds,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum AxisType {
	#[default]
	Linear,
	Date,
	Datetime(TimeUnit),
	Time,
}

#[allow(dead_code)]
#[derive(Clone)]
pub enum PlotLayout {
	Cartesian {
		x_range: (f64, f64),
		y_range: (f64, f64),
		x_axis_type: AxisType,
		y_axis_type: AxisType,
	},
	CategoricalX {
		categories: Vec<String>,
		y_range: (f64, f64),
	},
	CategoricalY {
		categories: Vec<String>,
		x_range: (f64, f64),
	},
	CategoricalXY {
		x_categories: Vec<String>,
		y_categories: Vec<String>,
	},
	Parallel {
		dimensions: Vec<String>,
		ranges: Vec<(f64, f64)>,
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

	pub fn cartesian(&self, data_x: f64, data_y: f64) -> Point {
		if let PlotLayout::Cartesian {
			x_range, y_range, ..
		} = self.layout
		{
			let x_delta = (x_range.1 - x_range.0).abs().max(f64::EPSILON);
			let y_delta = (y_range.1 - y_range.0).abs().max(f64::EPSILON);
			let x_scale = self.bounds.width as f64 / x_delta;
			let y_scale = self.bounds.height as f64 / y_delta;
			let pixel_x = self.bounds.x + ((data_x - x_range.0) * x_scale) as f32;
			let pixel_y =
				self.bounds.y + self.bounds.height - ((data_y - y_range.0) * y_scale) as f32;
			Point::new(pixel_x, pixel_y)
		} else {
			Point::ORIGIN
		}
	}

	pub fn categorical(&self, category_index: usize, data_val: f64) -> (Point, f32) {
		match self.layout {
			PlotLayout::CategoricalX {
				categories,
				y_range,
			} => {
				let num_cats = categories.len().max(1) as f32;
				let band_width = self.bounds.width / num_cats;
				let center_x =
					self.bounds.x + (category_index as f32 * band_width) + (band_width / 2.0);
				let y_delta = (y_range.1 - y_range.0).abs().max(f64::EPSILON);
				let y_scale = self.bounds.height as f64 / y_delta;
				let pixel_y =
					self.bounds.y + self.bounds.height - ((data_val - y_range.0) * y_scale) as f32;
				(Point::new(center_x, pixel_y), band_width)
			}
			PlotLayout::CategoricalY {
				categories,
				x_range,
			} => {
				let num_cats = categories.len().max(1) as f32;
				let band_height = self.bounds.height / num_cats;
				let center_y = self.bounds.y + self.bounds.height
					- (category_index as f32 * band_height)
					- (band_height / 2.0);
				let x_delta = (x_range.1 - x_range.0).abs().max(f64::EPSILON);
				let x_scale = self.bounds.width as f64 / x_delta;
				let pixel_x = self.bounds.x + ((data_val - x_range.0) * x_scale) as f32;
				(Point::new(pixel_x, center_y), band_height)
			}
			PlotLayout::Parallel { dimensions, ranges } => {
				let num_dims = dimensions.len().max(1) as f32;
				let axis_spacing = self.bounds.width / (num_dims - 1.0).max(1.0);
				let axis_x = self.bounds.x + (category_index as f32 * axis_spacing);
				let range = ranges.get(category_index).copied().unwrap_or((0.0, 1.0));
				let y_delta = (range.1 - range.0).abs().max(f64::EPSILON);
				let y_scale = self.bounds.height as f64 / y_delta;
				let pixel_y =
					self.bounds.y + self.bounds.height - ((data_val - range.0) * y_scale) as f32;
				(Point::new(axis_x, pixel_y), 0.0)
			}
			_ => (Point::ORIGIN, 0.0),
		}
	}

	pub fn categorical_2d(&self, x_index: usize, y_index: usize) -> (Point, f32, f32) {
		match self.layout {
			PlotLayout::CategoricalXY {
				x_categories,
				y_categories,
			} => {
				let num_x = x_categories.len().max(1) as f32;
				let num_y = y_categories.len().max(1) as f32;
				let band_width = self.bounds.width / num_x;
				let band_height = self.bounds.height / num_y;
				let center_x = self.bounds.x + (x_index as f32 * band_width) + (band_width / 2.0);
				let center_y = self.bounds.y + self.bounds.height
					- (y_index as f32 * band_height)
					- (band_height / 2.0);
				(Point::new(center_x, center_y), band_width, band_height)
			}
			_ => (Point::ORIGIN, 0.0, 0.0),
		}
	}

	#[allow(dead_code)]
	pub fn pixel_to_cartesian(&self, cursor_pos: Point) -> Option<(f64, f64)> {
		if let PlotLayout::Cartesian {
			x_range, y_range, ..
		} = self.layout
		{
			if !self.bounds.contains(cursor_pos) {
				return None;
			}
			let x_delta = (x_range.1 - x_range.0).abs().max(f64::EPSILON);
			let y_delta = (y_range.1 - y_range.0).abs().max(f64::EPSILON);
			let x_scale = x_delta / self.bounds.width as f64;
			let y_scale = y_delta / self.bounds.height as f64;
			let data_x = x_range.0 + ((cursor_pos.x - self.bounds.x) as f64 * x_scale);
			let data_y =
				y_range.0 + ((self.bounds.y + self.bounds.height - cursor_pos.y) as f64 * y_scale);
			Some((data_x, data_y))
		} else {
			None
		}
	}
}

#[derive(Debug, Clone, Copy)]
pub struct PlotSettings {
	pub max_legend_rows: u32,
	pub legend_x: f32,
	pub legend_y: f32,
	pub x_label_rotation: f32,
	pub x_label_offset: f32,
	pub color_theme: ColorTheme,
	pub background_color: Color,
	pub decoration_color: Color,
}

impl Default for PlotSettings {
	fn default() -> Self {
		Self {
			max_legend_rows: 4,
			legend_x: 0.95,
			legend_y: 0.05,
			x_label_rotation: 0.0,
			x_label_offset: 10.0,
			color_theme: ColorTheme::default(),
			background_color: Color::from_rgb(0.001, 0.001, 0.001),
			decoration_color: Color::WHITE,
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
		settings: PlotSettings,
	);

	fn draw_legend(&self, _frame: &mut Frame, _bounds: Rectangle, _settings: PlotSettings) {}

	fn hover(&self, transform: &CoordinateTransformer, cursor: Cursor) -> Option<String>;

	fn x_label(&self) -> String {
		String::new()
	}

	fn y_label(&self) -> String {
		String::new()
	}
}

pub struct PlotWidget<'a> {
	pub kernel: &'a dyn PlotKernel,
	pub title: String,
	pub padding: f32,
	pub settings: PlotSettings,
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
		let padding_bottom = self.padding + 80.0;
		let padding_left = self.padding + 100.0;
		let padding_right = self.padding + 20.0;
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
		self.kernel.plot(
			&mut frame,
			plot_area,
			&transform,
			relative_cursor,
			self.settings,
		);
		match &layout {
			PlotLayout::Cartesian {
				x_range, y_range, ..
			} => {
				self.draw_cartesian_grid(&mut frame, plot_area, &transform, *x_range, *y_range);
			}
			PlotLayout::CategoricalX {
				categories,
				y_range,
			} => {
				self.draw_categorical_axes(&mut frame, plot_area, &transform, categories, *y_range);
			}
			PlotLayout::CategoricalY {
				categories,
				x_range,
			} => {
				self.draw_categorical_y_axes(
					&mut frame, plot_area, &transform, categories, *x_range,
				);
			}
			PlotLayout::CategoricalXY {
				x_categories,
				y_categories,
			} => {
				self.draw_categorical_xy_axes(
					&mut frame,
					plot_area,
					&transform,
					x_categories,
					y_categories,
				);
			}
			PlotLayout::Parallel { dimensions, ranges } => {
				self.draw_parallel_axes(&mut frame, plot_area, &transform, dimensions, ranges);
			}
			PlotLayout::Radial => {}
		}
		self.kernel.draw_legend(&mut frame, bounds, self.settings);
		frame.fill_text(Text {
			content: self.title.clone(),
			position: Point::new(bounds.width / 2.0, 20.0),
			color: self.settings.decoration_color,
			size: iced::Pixels(28.0),
			align_x: alignment::Horizontal::Center.into(),
			align_y: alignment::Vertical::Top,
			..Default::default()
		});
		let x_label = self.kernel.x_label();
		let y_label = self.kernel.y_label();
		if !x_label.is_empty() {
			frame.fill_text(Text {
				content: x_label,
				position: Point::new(
					plot_area.x + plot_area.width / 2.0,
					plot_area.y + plot_area.height + 45.0,
				),
				color: self.settings.decoration_color,
				size: iced::Pixels(20.0),
				align_x: alignment::Horizontal::Center.into(),
				align_y: alignment::Vertical::Top,
				..Default::default()
			});
		}
		if !y_label.is_empty() {
			frame.with_save(|frame| {
				frame.translate(iced::Vector::new(
					plot_area.x - 85.0,
					plot_area.y + plot_area.height / 2.0,
				));
				frame.rotate(-std::f32::consts::FRAC_PI_2);
				frame.fill_text(Text {
					content: y_label,
					position: Point::ORIGIN,
					color: self.settings.decoration_color,
					size: iced::Pixels(20.0),
					align_x: alignment::Horizontal::Center.into(),
					align_y: alignment::Vertical::Bottom,
					..Default::default()
				});
			});
		}
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
			let padding_bottom = self.padding + 60.0;
			let padding_left = self.padding + 100.0;
			let padding_right = self.padding + 20.0;
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
		x_range: (f64, f64),
		y_range: (f64, f64),
	) {
		let x_axis_type = if let PlotLayout::Cartesian { x_axis_type, .. } = transform.layout {
			*x_axis_type
		} else {
			AxisType::Linear
		};
		let y_axis_type = if let PlotLayout::Cartesian { y_axis_type, .. } = transform.layout {
			*y_axis_type
		} else {
			AxisType::Linear
		};

		let grid_stroke = Stroke {
			style: Style::Solid(Color {
				a: 0.2,
				..self.settings.decoration_color
			}),
			width: 1.0,
			..Default::default()
		};
		let halo_stroke = Stroke {
			style: Style::Solid(self.settings.background_color),
			width: 4.0,
			..Default::default()
		};
		let axis_stroke = Stroke {
			style: Style::Solid(self.settings.decoration_color),
			width: 2.0,
			..Default::default()
		};
		let num_ticks = 8;
		let grid_path = Path::new(|builder| {
			for i in 0..=num_ticks {
				let t = i as f64 / num_ticks as f64;
				let data_y = y_range.0 + (y_range.1 - y_range.0) * t;
				let p_left = transform.cartesian(x_range.0, data_y);
				let p_right = transform.cartesian(x_range.1, data_y);
				builder.move_to(p_left);
				builder.line_to(p_right);
			}
			for i in 0..=num_ticks {
				let t = i as f64 / num_ticks as f64;
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
			let t = i as f64 / num_ticks as f64;
			let data_y = y_range.0 + (y_range.1 - y_range.0) * t;
			let p_left = transform.cartesian(x_range.0, data_y);
			let tick_path = Path::new(|builder| {
				builder.move_to(p_left);
				builder.line_to(Point::new(p_left.x - 5.0, p_left.y));
			});
			frame.stroke(&tick_path, axis_stroke);
			frame.fill_text(Text {
				content: format_label(data_y, y_axis_type),
				position: Point::new(p_left.x - 10.0, p_left.y),
				color: self.settings.decoration_color,
				size: iced::Pixels(18.0),
				align_x: alignment::Horizontal::Right.into(),
				align_y: alignment::Vertical::Center,
				..Default::default()
			});
		}
		for i in 0..=num_ticks {
			let t = i as f64 / num_ticks as f64;
			let data_x = x_range.0 + (x_range.1 - x_range.0) * t;
			let p_bottom = transform.cartesian(data_x, y_range.0);
			let tick_path = Path::new(|builder| {
				builder.move_to(p_bottom);
				builder.line_to(Point::new(p_bottom.x, p_bottom.y + 5.0));
			});
			frame.stroke(&tick_path, axis_stroke);
			frame.with_save(|frame| {
				frame.translate(iced::Vector::new(
					p_bottom.x,
					p_bottom.y + self.settings.x_label_offset,
				));
				frame.rotate(self.settings.x_label_rotation.to_radians());
				frame.fill_text(Text {
					content: format_label(data_x, x_axis_type),
					position: Point::ORIGIN,
					color: self.settings.decoration_color,
					size: iced::Pixels(18.0),
					align_x: alignment::Horizontal::Center.into(),
					..Default::default()
				});
			});
		}
	}

	fn draw_categorical_axes(
		&self,
		frame: &mut Frame,
		_area: Rectangle,
		transform: &CoordinateTransformer,
		categories: &[String],
		y_range: (f64, f64),
	) {
		let halo_stroke = Stroke {
			style: Style::Solid(self.settings.background_color),
			width: 4.0,
			..Default::default()
		};
		let axis_stroke = Stroke {
			style: Style::Solid(self.settings.decoration_color),
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
			let t = i as f64 / num_ticks as f64;
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
				content: format_label(data_y, AxisType::Linear),
				position: Point::new(p_left.x - 10.0, p_left.y),
				color: self.settings.decoration_color,
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
			frame.with_save(|frame| {
				frame.translate(iced::Vector::new(
					center_px.x,
					center_px.y + self.settings.x_label_offset,
				));
				frame.rotate(self.settings.x_label_rotation.to_radians());
				frame.fill_text(Text {
					content: cat.clone(),
					position: Point::ORIGIN,
					color: self.settings.decoration_color,
					size: iced::Pixels(18.0),
					align_x: alignment::Horizontal::Center.into(),
					..Default::default()
				});
			});
		}
	}

	fn draw_categorical_y_axes(
		&self,
		frame: &mut Frame,
		_area: Rectangle,
		transform: &CoordinateTransformer,
		categories: &[String],
		x_range: (f64, f64),
	) {
		let halo_stroke = Stroke {
			style: Style::Solid(self.settings.background_color),
			width: 4.0,
			..Default::default()
		};
		let axis_stroke = Stroke {
			style: Style::Solid(self.settings.decoration_color),
			width: 2.0,
			..Default::default()
		};
		let num_ticks = 8;
		let axes_path = Path::new(|builder| {
			let (first_cat_center, band_height) = transform.categorical(0, x_range.0);
			let bottom_edge = first_cat_center.y + (band_height / 2.0);
			let left_x = first_cat_center.x;
			let (last_cat_center, _) = transform.categorical(categories.len() - 1, x_range.1);
			let top_edge = last_cat_center.y - (band_height / 2.0);
			let right_x = last_cat_center.x;
			builder.move_to(Point::new(left_x, top_edge));
			builder.line_to(Point::new(left_x, bottom_edge));
			builder.line_to(Point::new(right_x, bottom_edge));
		});
		frame.stroke(&axes_path, halo_stroke);
		frame.stroke(&axes_path, axis_stroke);
		for i in 0..=num_ticks {
			let t = i as f64 / num_ticks as f64;
			let data_x = x_range.0 + (x_range.1 - x_range.0) * t;
			let (center, band_height) = transform.categorical(0, data_x);
			let bottom_edge = center.y + (band_height / 2.0);
			let p_bottom = Point::new(center.x, bottom_edge);
			let tick_path = Path::new(|builder| {
				builder.move_to(p_bottom);
				builder.line_to(Point::new(p_bottom.x, p_bottom.y + 5.0));
			});
			frame.stroke(&tick_path, axis_stroke);
			frame.fill_text(Text {
				content: format_label(data_x, AxisType::Linear),
				position: Point::new(p_bottom.x, p_bottom.y + 10.0),
				color: self.settings.decoration_color,
				size: iced::Pixels(18.0),
				align_x: alignment::Horizontal::Center.into(),
				..Default::default()
			});
		}
		for (i, cat) in categories.iter().enumerate() {
			let (center_px, _band_height) = transform.categorical(i, x_range.0);
			let tick_path = Path::new(|builder| {
				builder.move_to(center_px);
				builder.line_to(Point::new(center_px.x - 5.0, center_px.y));
			});
			frame.stroke(&tick_path, axis_stroke);
			frame.fill_text(Text {
				content: cat.clone(),
				position: Point::new(center_px.x - 10.0, center_px.y),
				color: self.settings.decoration_color,
				size: iced::Pixels(18.0),
				align_x: alignment::Horizontal::Right.into(),
				align_y: alignment::Vertical::Center,
				..Default::default()
			});
		}
	}

	fn draw_categorical_xy_axes(
		&self,
		frame: &mut Frame,
		_area: Rectangle,
		transform: &CoordinateTransformer,
		x_categories: &[String],
		y_categories: &[String],
	) {
		let halo_stroke = Stroke {
			style: Style::Solid(self.settings.background_color),
			width: 4.0,
			..Default::default()
		};
		let axis_stroke = Stroke {
			style: Style::Solid(self.settings.decoration_color),
			width: 2.0,
			..Default::default()
		};
		let axes_path = Path::new(|builder| {
			let (first_p, _bw, bh) = transform.categorical_2d(0, 0);
			let (last_p, bw, _bh) =
				transform.categorical_2d(x_categories.len() - 1, y_categories.len() - 1);
			let left_x = first_p.x - bw / 2.0;
			let bottom_y = first_p.y + bh / 2.0;
			let right_x = last_p.x + bw / 2.0;
			let top_y = last_p.y - bh / 2.0;
			builder.move_to(Point::new(left_x, top_y));
			builder.line_to(Point::new(left_x, bottom_y));
			builder.line_to(Point::new(right_x, bottom_y));
		});
		frame.stroke(&axes_path, halo_stroke);
		frame.stroke(&axes_path, axis_stroke);
		for (i, cat) in x_categories.iter().enumerate() {
			let (p, _bw, bh) = transform.categorical_2d(i, 0);
			let tick_x = p.x;
			let tick_y = p.y + bh / 2.0;
			let tick_path = Path::new(|builder| {
				builder.move_to(Point::new(tick_x, tick_y));
				builder.line_to(Point::new(tick_x, tick_y + 5.0));
			});
			frame.stroke(&tick_path, axis_stroke);
			frame.with_save(|frame| {
				frame.translate(iced::Vector::new(
					tick_x,
					tick_y + self.settings.x_label_offset,
				));
				frame.rotate(self.settings.x_label_rotation.to_radians());
				frame.fill_text(Text {
					content: cat.clone(),
					position: Point::ORIGIN,
					color: self.settings.decoration_color,
					size: iced::Pixels(18.0),
					align_x: alignment::Horizontal::Center.into(),
					..Default::default()
				});
			});
		}
		for (i, cat) in y_categories.iter().enumerate() {
			let (p, bw, _bh) = transform.categorical_2d(0, i);
			let tick_x = p.x - bw / 2.0;
			let tick_y = p.y;
			let tick_path = Path::new(|builder| {
				builder.move_to(Point::new(tick_x, tick_y));
				builder.line_to(Point::new(tick_x - 5.0, tick_y));
			});
			frame.stroke(&tick_path, axis_stroke);
			frame.fill_text(Text {
				content: cat.clone(),
				position: Point::new(tick_x - 10.0, tick_y),
				color: self.settings.decoration_color,
				size: iced::Pixels(18.0),
				align_x: alignment::Horizontal::Right.into(),
				align_y: alignment::Vertical::Center,
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
		ranges: &[(f64, f64)],
	) {
		let halo_stroke = Stroke {
			style: Style::Solid(self.settings.background_color),
			width: 10.0,
			..Default::default()
		};
		let axis_stroke = Stroke {
			style: Style::Solid(self.settings.decoration_color),
			width: 4.0,
			..Default::default()
		};
		let tick_stroke = Stroke {
			style: Style::Solid(Color {
				a: 0.6,
				..self.settings.decoration_color
			}),
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
				let t = j as f64 / num_ticks as f64;
				let data_y = range.0 + (range.1 - range.0) * t;
				let (p, _) = transform.categorical(i, data_y);
				let tick_path = Path::new(|builder| {
					builder.move_to(p);
					builder.line_to(Point::new(p.x - 6.0, p.y));
				});
				frame.stroke(&tick_path, tick_stroke);
				frame.fill_text(Text {
					content: format_label(data_y, AxisType::Linear),
					position: Point::new(p.x - 14.0, p.y),
					color: self.settings.decoration_color,
					size: iced::Pixels(18.0),
					align_x: alignment::Horizontal::Right.into(),
					align_y: alignment::Vertical::Center,
					..Default::default()
				});
			}
			frame.fill_text(Text {
				content: dim.clone(),
				position: Point::new(top_px.x, top_px.y - 20.0),
				color: self.settings.decoration_color,
				size: iced::Pixels(22.0),
				align_x: alignment::Horizontal::Center.into(),
				align_y: alignment::Vertical::Bottom,
				..Default::default()
			});
		}
	}
}

pub fn polars_type_to_axis_type(dt: &polars::prelude::DataType) -> AxisType {
	match dt {
		polars::prelude::DataType::Date => AxisType::Date,
		polars::prelude::DataType::Datetime(unit, _) => {
			let tu = match unit {
				polars::prelude::TimeUnit::Nanoseconds => TimeUnit::Nanoseconds,
				polars::prelude::TimeUnit::Microseconds => TimeUnit::Microseconds,
				polars::prelude::TimeUnit::Milliseconds => TimeUnit::Milliseconds,
			};
			AxisType::Datetime(tu)
		}
		polars::prelude::DataType::Time => AxisType::Time,
		_ => AxisType::Linear,
	}
}

pub fn format_label(value: f64, axis_type: AxisType) -> String {
	match axis_type {
		AxisType::Linear => {
			if value.abs() >= 1e6 || (value.abs() < 1e-3 && value != 0.0) {
				format!("{:.1e}", value)
			} else if value.fract() == 0.0 {
				format!("{:.0}", value)
			} else {
				format!("{:.1}", value)
			}
		}
		AxisType::Date => {
			let days = value as i64;
			if let Some(date) = chrono::NaiveDate::from_num_days_from_ce_opt(days as i32 + 719163) {
				date.format("%Y-%m-%d").to_string()
			} else {
				format!("{:.1}", value)
			}
		}
		AxisType::Datetime(unit) => {
			let timestamp = value as i64;
			let naive = match unit {
				TimeUnit::Milliseconds => chrono::DateTime::from_timestamp(
					timestamp / 1_000,
					(timestamp % 1_000) as u32 * 1_000_000,
				),
				TimeUnit::Microseconds => chrono::DateTime::from_timestamp(
					timestamp / 1_000_000,
					(timestamp % 1_000_000) as u32 * 1_000,
				),
				TimeUnit::Nanoseconds => chrono::DateTime::from_timestamp(
					timestamp / 1_000_000_000,
					(timestamp % 1_000_000_000) as u32,
				),
			};
			naive
				.map(|dt: chrono::DateTime<chrono::Utc>| dt.format("%Y-%m-%d %H:%M:%S").to_string())
				.unwrap_or_else(|| format!("{:.1}", value))
		}
		AxisType::Time => {
			let total_nanos = value as i64;
			let seconds = total_nanos / 1_000_000_000;
			let nanos = total_nanos % 1_000_000_000;
			let hours = seconds / 3600;
			let minutes = (seconds % 3600) / 60;
			let seconds = seconds % 60;
			format!(
				"{:02}:{:02}:{:02}.{:03}",
				hours,
				minutes,
				seconds,
				nanos / 1_000_000
			)
		}
	}
}
