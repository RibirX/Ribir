use crate::{path::*, Brush, Color, DeviceSize, PathStyle, Rect, TextStyle, Transform, Vector};
use algo::CowRc;
use std::ops::{Deref, DerefMut};
use text::FontFace;

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
}

/// `PainterBackend` use to draw the picture what the `commands` described  to
/// the target device. Usually is implemented by graphic library.
pub trait PainterBackend {
  fn submit(&mut self, commands: Vec<PaintCommand>);
  fn resize(&mut self, size: DeviceSize);
  /// Capture the image data of current frame, which encode as rgba(u8x4)
  /// format, the callback provide the image size and a iterator of data row
  /// by row.
  fn capture<'a>(
    &self,
    f: Box<dyn for<'r> FnOnce(DeviceSize, Box<dyn Iterator<Item = &[u8]> + 'r>) + 'a>,
  ) -> Result<(), &str>;
}

#[derive(Clone)]
pub enum PaintPath {
  Path(lyon_path::Path),
  Text {
    text: CowRc<str>,
    font_size: f32,
    font_face: CowRc<FontFace>,
    letter_space: f32,
    line_height: Option<f32>,
  },
}
#[derive(Clone)]
pub struct PaintCommand {
  pub path: PaintPath,
  pub transform: Transform,
  pub brush: Brush,
  pub path_style: PathStyle,
}

#[derive(Clone)]
struct PainterState {
  /// The line width use to stroke path.
  line_width: f32,
  font_size: f32,
  letter_space: f32,
  brush: Brush,
  font_face: CowRc<FontFace>,
  text_line_height: Option<f32>,
  transform: Transform,
}

impl Painter {
  pub fn new(device_scale: f32) -> Self {
    let mut p = Self {
      device_scale,
      state_stack: vec![PainterState::default()],
      commands: vec![],
      path_builder: Path::builder(),
    };
    p.scale(device_scale, device_scale);
    p
  }

  #[inline]
  pub fn finish(&mut self) -> Vec<PaintCommand> {
    self.reset(None);
    std::mem::take(&mut self.commands)
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
  pub fn save(&mut self) {
    let new_state = self.current_state().clone();
    self.state_stack.push(new_state);
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
  pub fn get_line_width(&self) -> f32 { self.current_state().line_width }

  /// Set the line width of the stroke pen with `line_width`
  #[inline]
  pub fn set_line_width(&mut self, line_width: f32) -> &mut Self {
    self.current_state_mut().line_width = line_width;
    self
  }

  /// Set the text line height which is a factor use to multiplied by the font
  /// size
  #[inline]
  pub fn set_text_line_height(&mut self, line_height: f32) -> &mut Self {
    self.current_state_mut().text_line_height = Some(line_height);
    self
  }

  #[inline]
  pub fn get_font(&self) -> &FontFace { &self.current_state().font_face }

  #[inline]
  pub fn set_font<F: Into<CowRc<FontFace>>>(&mut self, font: FontFace) -> &mut Self {
    self.current_state_mut().font_face = font.into();
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

  /// Paint a path with its style.
  pub fn paint_path(&mut self, path: Path) -> &mut Self {
    let transform = self.current_state().transform.clone();
    self.commands.push(PaintCommand {
      path: PaintPath::Path(path.path),
      transform,
      brush: path.brush.clone(),
      path_style: path.path_style,
    });
    self
  }

  /// Strokes (outlines) the current path with the current brush and line width.
  pub fn stroke(&mut self, line_width: Option<f32>, brush: Option<Brush>) -> &mut Self {
    let builder = std::mem::take(&mut self.path_builder);
    let state = self.current_state();
    let line_width = line_width.unwrap_or(self.current_state().line_width);
    let brush = brush.unwrap_or_else(|| state.brush.clone());
    let path = builder.stroke(line_width, brush);
    self.paint_path(path);
    self
  }

  /// Fill the current path with current brush.
  pub fn fill(&mut self, brush: Option<Brush>) -> &mut Self {
    let builder = std::mem::take(&mut self.path_builder);
    let brush = brush.unwrap_or_else(|| self.current_state().brush.clone());
    let path = builder.fill(brush.clone());
    self.paint_path(path);
    self
  }

  /// Paint text with its style
  pub fn paint_text<T: Into<CowRc<str>>>(&mut self, text: T, style: TextStyle) -> &mut Self {
    let transform = self.current_state().transform.clone();
    let TextStyle {
      font_size,
      foreground,
      font_face,
      letter_space,
      path_style,
      line_height,
    } = style;
    self.commands.push(PaintCommand {
      path: PaintPath::Text {
        text: text.into(),
        font_size,
        font_face,
        letter_space,
        line_height,
      },
      transform,
      brush: foreground,
      path_style,
    });

    self
  }

  /// Stroke `text` from left to right, start at let top position, use
  /// [`translate`](Painter::translate) move to the position what you want.
  /// Partially hitting the `max_width` will end the draw. Use `font` and
  /// `font_size` to specify the font and font size. Use
  /// [`fill_complex_texts`](Rendering2DLayer::fill_complex_texts) method to
  /// fill complex text.
  pub fn stroke_text<T: Into<CowRc<str>>>(&mut self, text: T) -> &mut Self {
    self.fill_text(text);
    self.commands.last_mut().unwrap().path_style =
      PathStyle::Stroke(self.current_state().line_width);
    self
  }

  /// Fill `text` from left to right, start at let top position, use
  /// [`translate`](Painter::translate) move to the position what you want.
  /// Partially hitting the `max_width` will end the draw. Use `font` and
  /// `font_size` to specify the font and font size. Use
  /// [`fill_complex_texts`](Rendering2DLayer::fill_complex_texts) method to
  /// fill complex text.
  pub fn fill_text<T: Into<CowRc<str>>>(&mut self, text: T) -> &mut Self {
    let state = self.current_state();
    let cmd = PaintCommand {
      path: PaintPath::Text {
        text: text.into(),
        font_size: state.font_size,
        font_face: state.font_face.clone(),
        letter_space: state.letter_space,
        line_height: state.text_line_height,
      },
      transform: state.transform.clone(),
      brush: state.brush.clone(),
      path_style: PathStyle::Fill,
    };
    self.commands.push(cmd);
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
    let t = self.get_transform().then_translate(Vector::new(x, y));
    self.set_transform(t);
    self
  }

  pub fn scale(&mut self, x: f32, y: f32) -> &mut Self {
    let t = self.get_transform().then_scale(x, y);
    self.set_transform(t);
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

impl<'a, 'b> DerefMut for PainterGuard<'a> {
  #[inline]
  fn deref_mut(&mut self) -> &mut Self::Target { self.0 }
}

impl Default for PainterState {
  #[inline]
  fn default() -> Self {
    Self {
      line_width: 1.,
      font_size: 14.,
      letter_space: 0.,
      brush: Brush::Color(Color::BLACK),
      font_face: CowRc::owned(FontFace::default()),
      text_line_height: None,
      transform: Transform::new(1., 0., 0., 1., 0., 0.),
    }
  }
}

impl Deref for Painter {
  type Target = Builder;

  #[inline]
  fn deref(&self) -> &Self::Target { &self.path_builder }
}

impl DerefMut for Painter {
  fn deref_mut(&mut self) -> &mut Self::Target { &mut self.path_builder }
}

impl PaintCommand {
  pub fn box_rect_without_transform(&self) -> Rect {
    match &self.path {
      PaintPath::Path(path) => path_box_rect(&path, self.path_style),
      PaintPath::Text { .. } => todo!(),
    }
  }
}

#[cfg(test)]
mod test {
  use super::*;

  #[test]
  fn save_guard() {
    let mut layer = Painter::new(1.);
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
