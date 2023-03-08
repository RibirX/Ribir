use crate::image::ColorFormat;
use crate::{
  path::*, path_builder::PathBuilder, Angle, Brush, Color, DeviceRect, DeviceSize, Point, Rect,
  Size, TextStyle, Transform, Vector,
};
use crate::{DevicePoint, PixelImage};
use euclid::{num::Zero, Size2D};
use ribir_algo::Substr;
use ribir_text::{
  typography::{Overflow, PlaceLineDirection, TypographyCfg},
  Em, FontFace, FontSize, Pixel, TypographyStore, VisualGlyphs,
};
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use std::{
  error::Error,
  future::Future,
  ops::{Deref, DerefMut},
};

/// The painter is a two-dimensional grid. The coordinate (0, 0) is at the
/// upper-left corner of the canvas. Along the X-axis, values increase towards
/// the right edge of the canvas. Along the Y-axis, values increase towards the
/// bottom edge of the canvas.
// #[derive(Default, Debug, Clone)]
pub struct Painter {
  bounds: DeviceRect,
  state_stack: Vec<PainterState>,
  commands: Vec<PaintCommand>,
  path_builder: PathBuilder,
  device_scale: f32,
  typography_store: TypographyStore,
}

/// `PainterBackend` use to draw textures for every frame, All `draw_commands`
/// will called between `begin_frame` and `end_frame`
///
/// -- begin_frame()
///  +--> draw_commands --+
///  ^                    V
///  +----------<---------+
///  |
///  V
/// -+ end_frame()
pub type ImageFuture =
  Pin<Box<dyn Future<Output = Result<PixelImage, Box<dyn Error>>> + Send + Sync>>;
pub trait PainterBackend {
  fn set_anti_aliasing(&mut self, anti_aliasing: AntiAliasing);

  /// A frame start.
  fn begin_frame(&mut self);

  /// Paint `commands` and return a `SubTexture` store the image. This may be
  /// called more than once to get multi sub-images.
  fn draw_commands(&mut self, view_port: DeviceRect, commands: Vec<PaintCommand>) -> ImageFuture;

  /// A frame end.
  fn end_frame(&mut self);
}

/// Texture use to display.
pub trait Texture {
  type Host;

  /// write data to the texture.
  fn write_data(&mut self, dist: &DeviceRect, data: &[u8], host: &mut Self::Host);

  fn copy_from_texture(
    &mut self,
    copy_to: DevicePoint,
    from_texture: &Self,
    from_rect: DeviceRect,
    host: &mut Self::Host,
  );

  fn copy_as_image(&self, rect: &DeviceRect, host: &mut Self::Host) -> ImageFuture;

  fn format(&self) -> ColorFormat;

  fn size(&self) -> DeviceSize;
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AntiAliasing {
  None = 1,
  Msaa2X = 2,
  Msaa4X = 4,
  Msaa8X = 8,
  Msaa16X = 16,
}

/// A path and its geometry information are friendly to paint and cache.
#[derive(Clone, Serialize, Deserialize)]
pub struct PaintPath {
  /// The path to painting, and its axis is relative to the `bounds`.
  pub path: Path,
  /// The device bounds of drawing this path are required.
  pub bounds: DeviceRect,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum PaintCommand {
  Fill { paint_path: PaintPath, brush: Brush },
  Clip(PaintPath),
  PopClip,
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
  opacity: f32,
  clip_cnt: u32,
  bounds: DeviceRect,
}

impl PainterState {
  fn new(device_scale: f32, bounds: DeviceRect) -> PainterState {
    PainterState {
      bounds,
      stroke_options: <_>::default(),
      font_size: FontSize::Pixel(14.0.into()),
      letter_space: None,
      brush: Color::BLACK.into(),
      font_face: FontFace::default(),
      text_line_height: None,
      transform: Transform::new(device_scale, 0., 0., device_scale, 0., 0.),
      clip_cnt: 0,
      opacity: 1.,
    }
  }
}

impl Painter {
  pub fn new(device_scale: f32, typography_store: TypographyStore) -> Self {
    let bounds = DeviceRect::from_size(DeviceSize::new(i32::MAX, i32::MAX));
    let mut p = Self {
      device_scale,
      state_stack: vec![PainterState::new(device_scale, bounds)],
      commands: vec![],
      path_builder: Path::builder(),
      typography_store,
      bounds,
    };
    p.scale(device_scale, device_scale);
    p
  }

  /// Change the bounds of the painter can draw.But it won't take effect until
  /// the next time you call [`Painter::reset`]!.
  pub fn set_bounds(&mut self, bounds: DeviceRect) { self.bounds = bounds; }

  pub fn paint_bounds(&self) -> &DeviceRect { &self.current_state().bounds }

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
  pub fn restore(&mut self) {
    let clip_cnt = self.current_state().clip_cnt;
    self.state_stack.pop();

    (self.current_state().clip_cnt..clip_cnt).for_each(|_| {
      if matches!(self.commands.last(), Some(PaintCommand::Clip(_))) {
        self.commands.pop();
      } else {
        self.commands.push(PaintCommand::PopClip)
      }
    });
  }

  pub fn reset(&mut self, device_scale: Option<f32>) {
    if let Some(scale) = device_scale {
      self.device_scale = scale;
    }
    self.state_stack.clear();
    self
      .state_stack
      .push(PainterState::new(self.device_scale, self.bounds));
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

  pub fn apply_alpha(&mut self, alpha: f32) -> &mut Self {
    self.current_state_mut().opacity *= alpha;
    self
  }

  pub fn alpha(&self) -> f32 { self.current_state().opacity }

  /// Return the line width of the stroke pen.
  #[inline]
  pub fn get_line_width(&self) -> f32 { self.stroke_options().width }

  /// Set the line width of the stroke pen with `line_width`
  #[inline]
  pub fn set_line_width(&mut self, line_width: f32) -> &mut Self {
    self.current_state_mut().stroke_options.width = line_width;
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
  pub fn get_line_cap(&mut self) -> LineCap { self.stroke_options().line_cap }

  #[inline]
  pub fn set_line_cap(&mut self, line_cap: LineCap) -> &mut Self {
    self.current_state_mut().stroke_options.line_cap = line_cap;
    self
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
  pub fn get_letter_space(&mut self) -> Pixel {
    self
      .current_state()
      .letter_space
      .unwrap_or_else(Pixel::zero)
  }

  #[inline]
  pub fn set_letter_space(&mut self, letter_space: Pixel) -> &mut Self {
    self.current_state_mut().letter_space = Some(letter_space);
    self
  }

  #[inline]
  pub fn get_font(&self) -> &FontFace { &self.current_state().font_face }

  #[inline]
  pub fn set_font(&mut self, font: FontFace) -> &mut Self {
    self.current_state_mut().font_face = font;
    self
  }

  #[inline]
  pub fn get_font_size(&self) -> &FontSize { &self.current_state().font_size }

  #[inline]
  pub fn set_font_size(&mut self, font_size: FontSize) -> &mut Self {
    self.current_state_mut().font_size = font_size;
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

  pub fn clip(&mut self, path: Path) -> &mut Self {
    let paint_path = PaintPath::new(path, self.get_transform());

    self.current_state_mut().bounds = self
      .current_state()
      .bounds
      .intersection(&paint_path.bounds)
      .unwrap_or(DeviceRect::zero());

    self.commands.push(PaintCommand::Clip(paint_path));
    self.current_state_mut().clip_cnt += 1;
    self
  }

  /// Fill a path with its style.
  pub fn fill_path(&mut self, path: Path) -> &mut Self {
    let alpha = self.alpha();
    let mut brush = self.current_state().brush.clone();
    brush.apply_opacify(alpha);
    let cmd = PaintCommand::fill(path, self.get_transform(), brush);
    let PaintCommand::Fill { paint_path,.. } = &cmd else { unreachable!(); };
    if paint_path.bounds.intersects(self.paint_bounds()) {
      self.commands.push(cmd);
    }
    self
  }

  /// Fill a path with its style.
  pub fn stroke_path(&mut self, path: Path) -> &mut Self {
    if let Some(stroke_path) = path.stroke(self.stroke_options(), Some(self.get_transform())) {
      self.fill_path(stroke_path);
    }
    self
  }

  /// Strokes (outlines) the current path with the current brush and line width.
  pub fn stroke(&mut self) -> &mut Self {
    let builder = std::mem::take(&mut self.path_builder);
    self.stroke_path(builder.build())
  }

  /// Fill the current path with current brush.
  pub fn fill(&mut self) -> &mut Self {
    let builder = std::mem::take(&mut self.path_builder);
    self.fill_path(builder.build())
  }

  /// Stroke `text` from left to right, start at let top position, use
  /// [`translate`](Painter::translate) move to the position what you want.
  /// Partially hitting the `max_width` will end the draw. Use `font` and
  /// `font_size` to specify the font and font size. Use
  /// [`fill_complex_texts`](Rendering2DLayer::fill_complex_texts) method to
  /// fill complex text.
  pub fn stroke_text<T: Into<Substr>>(&mut self, text: T, bounds: Option<Size>) -> &mut Self {
    self.paint_text_command(text, true, bounds);
    self
  }

  /// Fill `text` from left to right, start at let top position, use
  /// [`translate`](Painter::translate) move to the position what you want.
  /// Partially hitting the `max_width` will end the draw. Use `font` and
  /// `font_size` to specify the font and font size. Use
  /// [`fill_complex_texts`](Rendering2DLayer::fill_complex_texts) method to
  /// fill complex text.
  pub fn fill_text<T: Into<Substr>>(&mut self, text: T, bounds: Option<Size>) -> &mut Self {
    self.paint_text_command(text, false, bounds);
    self
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
  #[inline]
  pub fn end_path(&mut self, close: bool) -> &mut Self {
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

  fn stroke_options(&self) -> &StrokeOptions { &self.current_state().stroke_options }

  fn paint_text_command<T: Into<Substr>>(&mut self, text: T, stroke: bool, bounds: Option<Size>) {
    let &PainterState {
      font_size,
      letter_space,
      ref font_face,
      text_line_height,
      ..
    } = self.current_state();
    let visual_glyphs = typography_with_text_style(
      &self.typography_store,
      text,
      &TextStyle {
        font_size,
        font_face: font_face.clone(),
        letter_space,
        line_height: text_line_height,
      },
      bounds,
    );

    visual_glyphs.pixel_glyphs().for_each(|g| {
      let face = self
        .typography_store
        .font_db_mut()
        .face_data_or_insert(g.face_id)
        .expect("Font face not exist!");

      if let Some(path) = face.outline_glyph(g.glyph_id).map(Path) {
        // todo: mark glyph
        if stroke {
          self.stroke_path(path);
        } else {
          self.fill_path(path);
        }
      } else {
        todo!("image or svg fallback");
      }
    });
  }
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

impl PaintPath {
  pub fn new(mut path: Path, ts: &Transform) -> Self {
    if ts != &Transform::identity() {
      path = path.transform(ts);
    }
    let bounds = lyon_algorithms::aabb::bounding_box(&path.0)
      .round()
      .to_rect()
      .to_i32()
      .cast_unit();

    if bounds.min() != (0, 0).into() {
      path = path.transform(&Transform::translation(
        -bounds.origin.x as f32,
        -bounds.origin.y as f32,
      ));
    }

    PaintPath { path, bounds }
  }
}

impl PaintCommand {
  pub fn fill(path: Path, ts: &Transform, mut brush: Brush) -> Self {
    let paint_path = PaintPath::new(path, &ts);
    if let Brush::Image { transform, .. } = &mut brush {
      let mut ts = ts.clone();
      ts.m31 = 0.;
      ts.m32 = 0.;
      if let Some(ts) = ts.inverse() {
        *transform = ts.then(&transform);
      }
    }

    PaintCommand::Fill { paint_path, brush }
  }
}

#[cfg(test)]
mod test {
  use ribir_text::shaper::TextShaper;

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
