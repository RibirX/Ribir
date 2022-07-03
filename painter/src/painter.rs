use crate::{
  path::*, Angle, Brush, Color, DeviceSize, PathStyle, Point, Rect, Size, TextStyle, Transform,
  Vector,
};
use algo::{CowRc, Resource};
use euclid::Size2D;
pub use lyon_tessellation::{LineCap, LineJoin};
use std::ops::{Deref, DerefMut};
use text::typography::{Overflow, PlaceLineDirection, TypographyCfg};
use text::{Em, FontFace, Glyph, Pixel, TypographyStore, VisualGlyphs};
use text::{FontSize, Substr};

/// The painter is a two-dimensional grid. The coordinate (0, 0) is at the
/// upper-left corner of the canvas. Along the X-axis, values increase towards
/// the right edge of the canvas. Along the Y-axis, values increase towards the
/// bottom edge of the canvas.
// #[derive(Default, Debug, Clone)]
pub struct Painter {
  state_stack: Vec<PainterState>,
  commands: Vec<PaintCommand>,
  path_builder: Builder,
  device_scale: f32,
  typography_store: TypographyStore,
}

pub type CaptureCallback<'a> =
  Box<dyn for<'r> FnOnce(DeviceSize, Box<dyn Iterator<Item = &[u8]> + 'r>) + 'a>;
/// `PainterBackend` use to draw the picture what the `commands` described  to
/// the target device. Usually is implemented by graphic library.
pub trait PainterBackend {
  /// Submit the paint commands to draw, and call the `capture` callback to
  /// pass the frame image data with rgba(u8 x 4) format if it is Some-Value
  fn submit<'a>(
    &mut self,
    commands: Vec<PaintCommand>,
    capture: Option<CaptureCallback<'a>>,
  ) -> Result<(), &str>;

  fn resize(&mut self, size: DeviceSize);
}

#[derive(Clone)]
pub enum PaintPath {
  Path(Resource<Path>),
  Text {
    font_size: FontSize,
    glyphs: Vec<Glyph<Pixel>>,
    style: PathStyle,
  },
}
#[derive(Clone)]
pub struct PaintCommand {
  pub path: PaintPath,
  pub transform: Transform,
  pub brush: Brush,
}

#[derive(Clone)]
struct PainterState {
  /// The line width use to stroke path.
  stroke_options: StrokeOptions,
  font_size: FontSize,
  letter_space: Option<Pixel>,
  brush: Brush,
  font_face: FontFace,
  text_line_height: Option<Em>,
  transform: Transform,
}

impl Painter {
  pub fn new(device_scale: f32, typography_store: TypographyStore) -> Self {
    let mut p = Self {
      device_scale,
      state_stack: vec![PainterState::default()],
      commands: vec![],
      path_builder: Path::builder(),
      typography_store,
    };
    p.scale(device_scale, device_scale);
    p
  }

  #[inline]
  pub fn finish(&mut self) -> Vec<PaintCommand> {
    self.reset(None);
    self.commands.drain(..).collect()
  }

  /// Saves the entire state and return a guard to auto restore the state when
  /// if drop.
  #[must_use]
  pub fn save_guard(&mut self) -> PainterGuard {
    self.save();
    PainterGuard(self)
  }

  /// Saves the entire state of the canvas by pushing the current drawing state
  /// onto a stack.
  pub fn save(&mut self) -> &mut Self {
    let new_state = self.current_state().clone();
    self.state_stack.push(new_state);
    self
  }

  /// Restores the most recently saved canvas state by popping the top entry in
  /// the drawing state stack. If there is no saved state, this method does
  /// nothing.
  #[inline]
  pub fn restore(&mut self) { self.state_stack.pop(); }

  pub fn reset(&mut self, device_scale: Option<f32>) {
    if let Some(scale) = device_scale {
      self.device_scale = scale;
    }
    self.state_stack.clear();
    self.state_stack.push(PainterState::default());
    self.scale(self.device_scale, self.device_scale);
  }

  pub fn device_scale(&self) -> f32 { self.device_scale }

  /// Returns the color, gradient, or pattern used for draw. Only `Color`
  /// support now.
  #[inline]
  pub fn get_brush(&self) -> &Brush { &self.current_state().brush }

  /// Change the style of pen that used to draw path.
  #[inline]
  pub fn set_brush<S: Into<Brush>>(&mut self, brush: S) -> &mut Self {
    self.current_state_mut().brush = brush.into();
    self
  }

  /// Return the line width of the stroke pen.
  #[inline]
  pub fn get_line_width(&self) -> f32 { self.stroke_options().line_width }

  /// Set the line width of the stroke pen with `line_width`
  #[inline]
  pub fn set_line_width(&mut self, line_width: f32) -> &mut Self {
    self.current_state_mut().stroke_options.line_width = line_width;
    self
  }

  #[inline]
  pub fn get_line_join(&self) -> LineJoin { self.stroke_options().line_join }

  #[inline]
  pub fn set_line_join(&mut self, line_join: LineJoin) -> &mut Self {
    self.current_state_mut().stroke_options.line_join = line_join;
    self
  }

  #[inline]
  pub fn get_start_line_cap(&self) -> LineCap { self.stroke_options().start_cap }

  #[inline]
  pub fn set_start_line_cap(&mut self, start_cap: LineCap) -> &mut Self {
    self.current_state_mut().stroke_options.start_cap = start_cap;
    self
  }

  #[inline]
  pub fn get_end_line_cap(&self) -> LineCap { self.stroke_options().end_cap }

  #[inline]
  pub fn set_end_line_cap(&mut self, end_cap: LineCap) -> &mut Self {
    self.current_state_mut().stroke_options.end_cap = end_cap;
    self
  }

  #[inline]
  pub fn set_line_cap(&mut self, line_cap: LineCap) -> &mut Self {
    self.set_start_line_cap(line_cap).set_end_line_cap(line_cap)
  }

  #[inline]
  pub fn get_miter_limit(&self) -> f32 { self.stroke_options().miter_limit }

  #[inline]
  pub fn set_miter_limit(&mut self, miter_limit: f32) -> &mut Self {
    self.current_state_mut().stroke_options.miter_limit = miter_limit;
    self
  }

  /// Set the text line height which is a factor use to multiplied by the font
  /// size
  #[inline]
  pub fn set_text_line_height(&mut self, line_height: Em) -> &mut Self {
    self.current_state_mut().text_line_height = Some(line_height);
    self
  }

  #[inline]
  pub fn get_font(&self) -> &FontFace { &self.current_state().font_face }

  #[inline]
  pub fn set_font<F: Into<CowRc<FontFace>>>(&mut self, font: FontFace) -> &mut Self {
    self.current_state_mut().font_face = font;
    self
  }

  /// Return the current transformation matrix being applied to the layer.
  #[inline]
  pub fn get_transform(&self) -> &Transform { &self.current_state().transform }

  /// Resets (overrides) the current transformation to the identity matrix, and
  /// then invokes a transformation described by the arguments of this method.
  /// This lets you scale, rotate, translate (move), and skew the context.
  #[inline]
  pub fn set_transform(&mut self, transform: Transform) -> &mut Self {
    self.current_state_mut().transform = transform;
    self
  }

  /// Multiplies the current transformation with the matrix. This lets you
  /// scale, rotate, translate (move), and skew the context.
  pub fn apply_transform(&mut self, transform: &Transform) -> &mut Self {
    let t = &mut self.current_state_mut().transform;
    *t = t.then(transform);
    self
  }

  /// Paint a path with its style.
  pub fn paint_path<P: Into<Resource<Path>>>(&mut self, path: P) -> &mut Self {
    let transform = self.current_state().transform;
    let brush = self.current_state().brush.clone();
    self.commands.push(PaintCommand {
      path: PaintPath::Path(path.into()),
      transform,
      brush,
    });
    self
  }

  /// Strokes (outlines) the current path with the current brush and line width.
  pub fn stroke(&mut self) -> &mut Self {
    let builder = std::mem::take(&mut self.path_builder);
    let path = builder.stroke(self.stroke_options());
    self.paint_path(path);
    self
  }

  /// Fill the current path with current brush.
  pub fn fill(&mut self) -> &mut Self {
    let builder = std::mem::take(&mut self.path_builder);
    let path = builder.fill();
    self.paint_path(path);
    self
  }

  /// Paint text with its style
  pub fn paint_text_with_style<T: Into<Substr>>(
    &mut self,
    text: T,
    style: &TextStyle,
    bounds: Option<Size>,
  ) -> &mut Self {
    let transform = self.current_state().transform;
    let visual_glyphs = typography_with_text_style(&self.typography_store, text, style, bounds);
    self.commands.push(PaintCommand {
      path: PaintPath::Text {
        font_size: style.font_size,
        glyphs: visual_glyphs.pixel_glyphs().collect(),
        style: style.path_style,
      },
      brush: style.foreground.clone(),
      transform,
    });

    self
  }

  /// Paint text without specify text style. The text style will come from the
  /// current state of this painter. Draw from left to right, start at let top
  /// position, use [`translate`](Painter::translate) move to the
  /// position what you want.
  pub fn paint_text_without_style<T: Into<Substr>>(
    &mut self,
    text: T,
    path_style: PathStyle,
    bounds: Option<Size>,
  ) -> &mut Self {
    let &PainterState {
      font_size,
      letter_space,
      ref brush,
      ref font_face,
      text_line_height,
      transform,
      ..
    } = self.current_state();
    let visual_glyphs = typography_with_text_style(
      &self.typography_store,
      text,
      &TextStyle {
        font_size,
        foreground: brush.clone(),
        font_face: font_face.clone(),
        letter_space,
        path_style,
        line_height: text_line_height,
      },
      bounds,
    );

    let cmd = PaintCommand {
      path: PaintPath::Text {
        font_size,
        glyphs: visual_glyphs.pixel_glyphs().collect(),
        style: path_style,
      },
      transform,
      brush: brush.clone(),
    };
    self.commands.push(cmd);
    self
  }

  /// Stroke `text` from left to right, start at let top position, use
  /// [`translate`](Painter::translate) move to the position what you want.
  /// Partially hitting the `max_width` will end the draw. Use `font` and
  /// `font_size` to specify the font and font size. Use
  /// [`fill_complex_texts`](Rendering2DLayer::fill_complex_texts) method to
  /// fill complex text.
  pub fn stroke_text<T: Into<Substr>>(&mut self, text: T) -> &mut Self {
    self.paint_text_without_style(text, PathStyle::Stroke(self.stroke_options()), None)
  }

  /// Fill `text` from left to right, start at let top position, use
  /// [`translate`](Painter::translate) move to the position what you want.
  /// Partially hitting the `max_width` will end the draw. Use `font` and
  /// `font_size` to specify the font and font size. Use
  /// [`fill_complex_texts`](Rendering2DLayer::fill_complex_texts) method to
  /// fill complex text.
  pub fn fill_text<T: Into<Substr>>(&mut self, text: T, bounds: Option<Size>) -> &mut Self {
    self.paint_text_without_style(text, PathStyle::Fill, bounds)
  }

  /// Adds a translation transformation to the current matrix by moving the
  /// canvas and its origin x units horizontally and y units vertically on the
  /// grid.
  ///
  /// * `x` - Distance to move in the horizontal direction. Positive values are
  ///   to the right, and negative to the left.
  /// * `y` - Distance to move in the vertical direction. Positive values are
  ///   down, and negative are up.
  pub fn translate(&mut self, x: f32, y: f32) -> &mut Self {
    let t = self.get_transform().pre_translate(Vector::new(x, y));
    self.set_transform(t);
    self
  }

  pub fn scale(&mut self, x: f32, y: f32) -> &mut Self {
    let t = self.get_transform().pre_scale(x, y);
    self.set_transform(t);
    self
  }

  /// Starts a new path by emptying the list of sub-paths.
  /// Call this method when you want to create a new path.
  #[inline]
  pub fn begin_path(&mut self, at: Point) -> &mut Self {
    self.path_builder.begin_path(at);
    self
  }

  /// Tell the painter the sub-path is finished.
  /// if `close` is true,  causes the point of the pen to move back to the start
  /// of the current sub-path. It tries to draw a straight line from the
  /// current point to the start. If the shape has already been closed or has
  /// only one point, nothing to do.
  #[inline]
  pub fn close_path(&mut self, close: bool) -> &mut Self {
    self.path_builder.end_path(close);
    self
  }

  /// Connects the last point in the current sub-path to the specified (x, y)
  /// coordinates with a straight line.
  #[inline]
  pub fn line_to(&mut self, to: Point) -> &mut Self {
    self.path_builder.line_to(to);
    self
  }

  /// Adds a cubic Bezier curve to the current path.
  #[inline]
  pub fn bezier_curve_to(&mut self, ctrl1: Point, ctrl2: Point, to: Point) -> &mut Self {
    self.path_builder.bezier_curve_to(ctrl1, ctrl2, to);
    self
  }

  /// Adds a quadratic Bézier curve to the current path.
  #[inline]
  pub fn quadratic_curve_to(&mut self, ctrl: Point, to: Point) -> &mut Self {
    self.path_builder.quadratic_curve_to(ctrl, to);
    self
  }

  /// adds a circular arc to the current sub-path, using the given control
  /// points and radius. The arc is automatically connected to the path's latest
  /// point with a straight line, if necessary for the specified
  #[inline]
  pub fn arc_to(
    &mut self,
    center: Point,
    radius: f32,
    start_angle: Angle,
    end_angle: Angle,
  ) -> &mut Self {
    self
      .path_builder
      .arc_to(center, radius, start_angle, end_angle);
    self
  }

  /// The ellipse_to() method creates an elliptical arc centered at `center`
  /// with the `radius`. The path starts at startAngle and ends at endAngle, and
  /// travels in the direction given by anticlockwise (defaulting to
  /// clockwise).
  #[inline]
  pub fn ellipse_to(
    &mut self,
    center: Point,
    radius: Vector,
    start_angle: Angle,
    end_angle: Angle,
  ) -> &mut Self {
    self
      .path_builder
      .ellipse_to(center, radius, start_angle, end_angle);
    self
  }

  #[inline]
  pub fn segment(&mut self, from: Point, to: Point) -> &mut Self {
    self.path_builder.segment(from, to);
    self
  }

  /// Adds a sub-path containing an ellipse.
  ///
  /// There must be no sub-path in progress when this method is called.
  /// No sub-path is in progress after the method is called.
  #[inline]
  pub fn ellipse(&mut self, center: Point, radius: Vector, rotation: f32) -> &mut Self {
    self.path_builder.ellipse(center, radius, rotation);
    self
  }

  /// Adds a sub-path containing a rectangle.
  ///
  /// There must be no sub-path in progress when this method is called.
  /// No sub-path is in progress after the method is called.
  #[inline]
  pub fn rect(&mut self, rect: &Rect) -> &mut Self {
    self.path_builder.rect(rect);
    self
  }

  /// Adds a sub-path containing a circle.
  ///
  /// There must be no sub-path in progress when this method is called.
  /// No sub-path is in progress after the method is called.
  #[inline]
  pub fn circle(&mut self, center: Point, radius: f32) -> &mut Self {
    self.path_builder.circle(center, radius);
    self
  }

  /// Creates a path for a rectangle by `rect` with `radius`.
  /// #[inline]
  #[inline]
  pub fn rect_round(&mut self, rect: &Rect, radius: &Radius) -> &mut Self {
    self.path_builder.rect_round(rect, radius);
    self
  }
}

impl Painter {
  fn current_state(&self) -> &PainterState {
    self
      .state_stack
      .last()
      .expect("Must have one state in stack!")
  }

  fn current_state_mut(&mut self) -> &mut PainterState {
    self
      .state_stack
      .last_mut()
      .expect("Must have one state in stack!")
  }

  fn stroke_options(&self) -> StrokeOptions { self.current_state().stroke_options }
}

/// An RAII implementation of a "scoped state" of the render layer. When this
/// structure is dropped (falls out of scope), changed state will auto restore.
/// The data can be accessed through this guard via its Deref and DerefMut
/// implementations.
pub struct PainterGuard<'a>(&'a mut Painter);

impl<'a> Drop for PainterGuard<'a> {
  #[inline]
  fn drop(&mut self) {
    debug_assert!(!self.0.state_stack.is_empty());
    self.0.restore();
  }
}

impl<'a> Deref for PainterGuard<'a> {
  type Target = Painter;
  #[inline]
  fn deref(&self) -> &Self::Target { self.0 }
}

impl<'a> DerefMut for PainterGuard<'a> {
  #[inline]
  fn deref_mut(&mut self) -> &mut Self::Target { self.0 }
}

impl Default for PainterState {
  #[inline]
  fn default() -> Self {
    Self {
      stroke_options: <_>::default(),
      font_size: FontSize::Pixel(14.0.into()),
      letter_space: None,
      brush: Brush::Color(Color::BLACK),
      font_face: FontFace::default(),
      text_line_height: None,
      transform: Transform::new(1., 0., 0., 1., 0., 0.),
    }
  }
}

impl PaintCommand {
  pub fn box_rect_without_transform(&self) -> Rect {
    match &self.path {
      PaintPath::Path(path) => path.box_rect(),
      PaintPath::Text { .. } => todo!(),
    }
  }
}

pub fn typography_with_text_style<T: Into<Substr>>(
  store: &TypographyStore,
  text: T,
  style: &TextStyle,
  bounds: Option<Size>,
) -> VisualGlyphs {
  let &TextStyle {
    font_size,
    letter_space,
    line_height,
    ref font_face,
    ..
  } = style;

  let bounds = if let Some(b) = bounds {
    let width: Em = Pixel(b.width.into()).into();
    let height: Em = Pixel(b.width.into()).into();
    Size2D::new(width, height)
  } else {
    let max = Em::absolute(f32::MAX);
    Size2D::new(max, max)
  };

  store.typography(
    text.into(),
    font_size,
    font_face,
    TypographyCfg {
      line_height,
      letter_space,
      text_align: None,
      bounds,
      line_dir: PlaceLineDirection::TopToBottom,
      overflow: Overflow::Clip,
    },
  )
}

impl PaintCommand {
  pub fn style(&self) -> PathStyle {
    match &self.path {
      PaintPath::Path(p) => p.style,
      PaintPath::Text { style, .. } => *style,
    }
  }
}

#[cfg(test)]
mod test {
  use text::shaper::TextShaper;

  use super::*;

  #[test]
  fn save_guard() {
    let mut layer = Painter::new(
      1.,
      TypographyStore::new(
        <_>::default(),
        <_>::default(),
        TextShaper::new(<_>::default()),
      ),
    );
    {
      let mut paint = layer.save_guard();
      let t = Transform::new(1., 1., 1., 1., 1., 1.);
      paint.set_transform(t);
      assert_eq!(&t, paint.get_transform());
      {
        let mut p2 = paint.save_guard();
        let t2 = Transform::new(2., 2., 2., 2., 2., 2.);
        p2.set_transform(t2);
        assert_eq!(&t2, p2.get_transform());
      }
      assert_eq!(&t, paint.get_transform());
    }
    assert_eq!(
      &Transform::new(1., 0., 0., 1., 0., 0.),
      layer.get_transform()
    );
  }
}
