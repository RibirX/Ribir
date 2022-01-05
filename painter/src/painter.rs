use crate::{
  path::*, Brush, Color, DeviceSize, FontFace, PathStyle, Rect, TextStyle, Transform, Vector,
};
use algo::CowRc;
use std::ops::{Deref, DerefMut};

/// The painter is a two-dimensional grid. The coordinate (0, 0) is at the
/// upper-left corner of the canvas. Along the X-axis, values increase towards
/// the right edge of the canvas. Along the Y-axis, values increase towards the
/// bottom edge of the canvas.
// #[derive(Default, Debug, Clone)]
pub struct Painter {
  init_transform: Transform,
  state_stack: Vec<PainterState>,
  commands: Vec<PaintCommand>,
  path_builder: Builder,
}

/// `PainterBackend` use to draw the picture what the `commands` described  to
/// the target device. Usually is implemented by graphic library.
pub trait PainterBackend {
  fn submit(&mut self, commands: Vec<PaintCommand>);
  fn resize(&mut self, size: DeviceSize);
}

#[derive(Clone)]
pub enum PaintPath {
  Path(lyon_path::Path),
  Text {
    text: CowRc<str>,
    font_size: f32,
    font_face: CowRc<FontFace>,
    letter_space: f32,
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
  line_width: f32,
  font_size: f32,
  letter_space: f32,
  brush: Brush,
  font_face: CowRc<FontFace>,
  transform: Transform,
}

impl Painter {
  pub fn new(transform: Transform) -> Self {
    let mut state = PainterState::default();
    state.transform = transform.clone();

    Self {
      init_transform: transform,
      state_stack: vec![PainterState::default()],
      commands: vec![],
      path_builder: Path::builder(),
    }
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

  pub fn reset(&mut self, transform: Option<Transform>) {
    if let Some(t) = transform {
      self.init_transform = t;
    }
    self.state_stack.clear();
    let mut state = PainterState::default();
    state.transform = self.init_transform;
    self.state_stack.push(state);
  }

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
    } = style;
    self.commands.push(PaintCommand {
      path: PaintPath::Text {
        text: text.into(),
        font_size,
        font_face,
        letter_space,
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
    let t = &mut self.current_state_mut().transform;
    *t = t.then_translate(Vector::new(x, y));
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
  use canvas::{mem_texture::MemTexture, Size};
  use futures::executor::block_on;

  struct MockRender;

  impl CanvasRender for MockRender {
    fn draw(&mut self, _: &RenderData, _: &mut MemTexture<u8>, _: &mut MemTexture<u32>) {}
    fn resize(&mut self, _: DeviceSize) {}
  }

  #[test]
  fn save_guard() {
    let mut layer = Painter::new();
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

  #[test]
  fn buffer() {
    let mut layer = Painter::new();
    let mut canvas = Canvas::new(None);
    let mut builder = Builder::new();
    builder.rect(&euclid::Rect::from_size((100., 100.).into()));
    let path = builder.build();
    layer.stroke_path(path.clone());
    layer.fill_path(path);

    canvas.consume_2d_layer(
      layer,
      &mut tessellator_2d::Tessellator::new(),
      &mut MockRender {},
    );

    assert!(canvas.render_data().has_data());
  }

  #[test]
  #[should_panic(expected = "not support in early develop")]
  fn path_merge() {
    let mut layer = Painter::new();

    let mut canvas = Canvas::new(None);
    let mut tessellator = tessellator_2d::Tessellator::new();
    let mut mock_render = MockRender {};

    let sample_path = Builder::new().build();
    // The stroke path both style and line width same should be merge.
    layer.stroke_path(sample_path.clone());
    layer.stroke_path(sample_path.clone());
    canvas.consume_2d_layer(layer, &mut tessellator, &mut mock_render);
    assert_eq!(canvas.render_data().primitives.len(), 1);

    let mut layer = Painter::new();
    // Different line width with same color pen can be merged.
    layer.set_line_width(2.);
    layer.stroke_path(sample_path.clone());
    canvas.consume_2d_layer(layer, &mut tessellator, &mut mock_render);
    assert_eq!(canvas.render_data().primitives.len(), 1);

    let mut layer = Painter::new();
    // Different color can't be merged.
    layer.set_brush(Brush::Color(Color::YELLOW));
    layer.fill_path(sample_path.clone());
    canvas.consume_2d_layer(layer, &mut tessellator, &mut mock_render);
    assert_eq!(canvas.render_data().primitives.len(), 2);

    let mut layer = Painter::new();
    // Different type style can't be merged
    layer.set_brush(Brush::Image);
    layer.fill_path(sample_path.clone());
    layer.stroke_path(sample_path);
    canvas.consume_2d_layer(layer, &mut tessellator, &mut mock_render);
    // image not not support now, should panic.
    assert_eq!(canvas.render_data().primitives.len(), 4);
  }

  #[test]
  #[ignore = "gpu need"]
  fn fill_text_hello() {
    let (mut canvas, mut render) = block_on(crate::create_canvas_with_render_headless(
      DeviceSize::new(400, 400),
    ));

    let mut layer = canvas.new_2d_layer();
    let font = FontInfo::new();
    layer.set_font(font);
    layer.fill_text("Nice to meet you!", None);
    {
      let mut frame = canvas.next_frame(&mut render);
      frame.compose_2d_layer(layer);
    }

    unit_test::assert_canvas_eq!(render, "../../test_imgs/text_hello.png");
  }

  #[test]
  #[ignore = "gpu need"]
  fn fill_text_complex() {
    let (mut canvas, mut render) = block_on(crate::create_canvas_with_render_headless(
      DeviceSize::new(400, 400),
    ));
    let serif = FontInfo::new();

    let mut layer = canvas.new_2d_layer();
    layer.fill_complex_texts(
      vec![(
        Text {
          text: "Hi, nice to meet you!",
          font: serif.clone(),
          font_size: 36.,
        },
        Color::BLACK,
      )],
      Some(Rect::from_size(Size::new(400., 400.))),
      None,
    );

    let arial = FontInfo::new().with_family("Arial".to_owned());

    layer.fill_complex_texts(
      vec![(
        Text {
          text: r#"To be, or not to be, that is the question!
Whether it’s nobler in the mind to suffer
The slings and arrows of outrageous fortune,
Or to take arms against a sea of troubles,
And by opposing end them? To die: to sleep;
"#,
          font: arial,
          font_size: 24.,
        },
        Color::GRAY,
      )],
      Some(Rect::from_size(Size::new(400., 400.))),
      Some(TextLayout {
        h_align: HorizontalAlign::Center,
        v_align: VerticalAlign::Center,
        wrap: LineWrap::Wrap,
      }),
    );

    layer.fill_complex_texts(
      vec![(
        Text {
          text: "Bye!",
          font: serif,
          font_size: 48.,
        },
        Color::RED,
      )],
      Some(Rect::from_size(Size::new(400., 400.))),
      Some(TextLayout {
        h_align: HorizontalAlign::Right,
        v_align: VerticalAlign::Bottom,
        wrap: LineWrap::SingleLine,
      }),
    );

    {
      let mut frame = canvas.next_frame(&mut render);
      frame.compose_2d_layer(layer);
    }

    unit_test::assert_canvas_eq!(render, "../../test_imgs/complex_text.png");
  }

  #[test]
  #[ignore = "gpu need"]
  fn fill_text_complex_single_style() {
    let (mut canvas, mut render) = block_on(crate::create_canvas_with_render_headless(
      DeviceSize::new(400, 400),
    ));
    let arial = FontInfo::new().with_family("Arial".to_owned());
    let serif = FontInfo::new();
    let mut layer = canvas.new_2d_layer();

    layer.set_style(Brush::Color(Color::GRAY));
    layer.fill_complex_texts_by_style(
      vec![
        Text {
          text: "Hi, nice to meet you!\n",
          font: serif.clone(),
          font_size: 36.,
        },
        Text {
          text: "\nTo be, or not to be, that is the question!
Whether it’s nobler in the mind to suffer
The slings and arrows of outrageous fortune,
Or to take arms against a sea of troubles,
And by opposing end them? To die: to sleep;\n",
          font: arial,
          font_size: 24.,
        },
        Text {
          text: "Bye!",
          font: serif,
          font_size: 48.,
        },
      ],
      Some(Rect::from_size(Size::new(400., 400.))),
      Some(TextLayout {
        h_align: HorizontalAlign::Right,
        v_align: VerticalAlign::Bottom,
        wrap: LineWrap::Wrap,
      }),
    );

    {
      let mut frame = canvas.next_frame(&mut render);
      frame.compose_2d_layer(layer);
    }

    unit_test::assert_canvas_eq!(render, "../../test_imgs/complex_text_single_style.png");
  }

  #[test]
  #[ignore = "gpu need"]
  fn update_texture_on_processing() {
    let text = include_str!("../../fonts/loads-of-unicode.txt");
    let (mut canvas, mut render) = block_on(crate::create_canvas_with_render_headless(
      DeviceSize::new(400, 400),
    ));
    let crate_root = env!("CARGO_MANIFEST_DIR").to_owned();
    canvas
      .text_brush()
      .load_font_from_path(crate_root + "/fonts/DejaVuSans.ttf", 0)
      .unwrap();
    let deja = FontInfo::new().with_family("DejaVu Sans".to_owned());
    let mut layer = canvas.new_2d_layer();
    layer.fill_complex_texts_by_style(
      vec![Text { text, font: deja, font_size: 36. }],
      Some(Rect::from_size(Size::new(1600., 1600.))),
      Some(TextLayout {
        h_align: HorizontalAlign::Right,
        v_align: VerticalAlign::Bottom,
        wrap: LineWrap::Wrap,
      }),
    );

    {
      let mut frame = canvas.next_frame(&mut render);
      frame.compose_2d_layer(layer);
    }

    unit_test::assert_canvas_eq!(render, "../../test_imgs/texture_cache_update.png");
  }

  #[test]
  #[ignore = "gpu need"]
  fn round_rect() {
    let (mut canvas, mut render) = block_on(crate::create_canvas_with_render_headless(
      DeviceSize::new(800, 200),
    ));

    let mut layer = canvas.new_2d_layer();
    layer.set_style(Color::RED);
    let rect = Rect::from_size(Size::new(80., 40.));

    let radius = Vector::new(20., 10.);
    [
      BorderRadius::all(Vector::zero()),
      BorderRadius::all(Vector::new(10., 10.)),
      BorderRadius {
        top_left: radius,
        ..Default::default()
      },
      BorderRadius {
        top_right: radius,
        ..Default::default()
      },
      BorderRadius {
        bottom_right: radius,
        ..Default::default()
      },
      BorderRadius {
        bottom_left: radius,
        ..Default::default()
      },
      BorderRadius {
        top_left: Vector::new(50., 50.),
        bottom_right: Vector::new(50., 50.),
        ..Default::default()
      },
    ]
    .iter()
    .for_each(|radius| {
      layer.save_guard().rect_round(&rect, radius).stroke();
      layer
        .translate(0., rect.height() + 5.)
        .rect_round(&rect, radius)
        .fill();

      layer.translate(rect.width() + 5., -(rect.height() + 5.));
    });

    {
      let mut frame = canvas.next_frame(&mut render);
      frame.compose_2d_layer(layer);
    }

    unit_test::assert_canvas_eq!(render, "../../test_imgs/rect_round.png");
  }
}
