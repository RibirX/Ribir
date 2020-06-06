use crate::{
  text_brush::GlyphStatistics, Canvas, Color, FontProperties, FontStretch, FontStyle, FontWeight,
  Point, Rect, Transform, DEFAULT_FONT_FAMILY,
};
pub use glyph_brush::{GlyphCruncher, HorizontalAlign, Layout, VerticalAlign};
pub use lyon::{
  path::{builder::PathBuilder, traits::PathIterator, Path, Winding},
  tessellation::*,
};
mod process;
use std::{
  cmp::PartialEq,
  ops::{Deref, DerefMut},
};

/// The 2d layer is a two-dimensional grid. The coordinate (0, 0) is at the
/// upper-left corner of the canvas. Along the X-axis, values increase towards
/// the right edge of the canvas. Along the Y-axis, values increase towards the
/// bottom edge of the canvas.
#[derive(Debug, Clone)]
pub struct Rendering2DLayer<'a> {
  state_stack: Vec<State>,
  commands: Vec<Command<'a>>,
}

impl<'a> Rendering2DLayer<'a> {
  pub fn new() -> Self {
    Self {
      state_stack: vec![State::new()],
      commands: vec![],
    }
  }

  /// Saves the entire state of the canvas by pushing the current drawing state
  /// onto a stack.
  #[must_use]
  pub fn save<'l>(&'l mut self) -> LayerGuard<'l, 'a> {
    let new_state = self.current_state().clone();
    self.state_stack.push(new_state);
    LayerGuard(self)
  }

  /// Returns the color, gradient, or pattern used for draw. Only `Color`
  /// support now.
  #[inline]
  pub fn get_style(&self) -> &FillStyle { &self.current_state().style }

  /// Change the style of pen that used to draw path.
  #[inline]
  pub fn set_style(&mut self, pen_style: FillStyle) -> &mut Self {
    self.current_state_mut().style = pen_style;
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
  pub fn get_font(&self) -> &FontInfo { &self.current_state().font }

  #[inline]
  pub fn set_font(&mut self, font: FontInfo) -> &mut Self {
    self.current_state_mut().font = font;
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

  /// Renders the specified path by using the current pen.
  pub fn stroke_path(&mut self, path: Path) {
    let cmd = self.command_from_path(path, true);
    self.commands.push(cmd);
  }

  /// Use current brush fill the interior of the `path`.
  pub fn fill_path(&mut self, path: Path) {
    let cmd = self.command_from_path(path, false);
    self.commands.push(cmd);
  }

  /// Fill `text` from left to right, start at let top position, use translate
  /// move to the position what you want. Partially hitting the `max_width`
  /// will end the draw. Use `font` and `font_size` to specify the font and
  /// font size. Use [`fill_text_with_desc`](Rendering2DLayer::
  /// fill_text_with_desc) method to fill complex text.
  pub fn fill_text(&mut self, text: &'a str, max_width: Option<f32>) {
    let cmd = self.command_from_text(text, max_width);
    self.commands.push(cmd);
  }

  /// Draw multi texts with different font and color, and specify how to layout
  /// it. Use [`fill_text`](Rendering2DLayer::fill_text) if just draw a single
  /// line simple text.
  ///
  /// # Arguments.
  ///
  /// * `texts` -  Pairs of Text and its color to render, rendered next to one
  ///   another.
  /// * `bounds` - Box bounds, in pixels from top-left.
  /// * `layout` - Layout info of the texts
  pub fn fill_complex_texts(
    &mut self,
    texts: Vec<(Text<'a>, Color)>,
    bounds: Option<Rect>,
    layout: Option<TextLayout>,
  ) {
    let cmd = self.command_from(|_| CommandInfo::ComplexTexts {
      texts,
      bounds,
      layout,
    });
    self.commands.push(cmd)
  }

  /// Draw multi texts with different font, and specify how to layout it. Its
  /// behavior is similar with
  /// [`fill_complex_texts`](Rendering2DLayer::fill_complex_texts), but use
  /// current style to draw and texts can't specify color.
  ///
  /// # Arguments.
  ///
  /// * `texts` -  texts to render, rendered next to one another.
  /// * `bounds` - Box bounds, in pixels from top-left.
  /// * `layout` - Layout info of the texts
  pub fn fill_complex_texts_by_style(
    &mut self,
    texts: Vec<Text<'a>>,
    bounds: Option<Rect>,
    layout: Option<TextLayout>,
  ) {
    let cmd = self.command_from(|state| CommandInfo::ComplexTextsByStyle {
      texts,
      bounds,
      layout,
      style: state.style.clone(),
    });
    self.commands.push(cmd)
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
    *t = t.post_translate(euclid::Vector2D::new(x, y));
    self
  }

  /// All drawing of this layer has finished, commit it to canvas and convert
  /// the layer to an intermediate render buffer data that will provide to
  /// render process and can directly commit to canvas.
  ///
  /// Return
  /// If the canvas texture cache is update in the process, will return a
  /// None-Value that means there is no buffer data can be cached.
  pub fn finish(self, canvas: &mut Canvas) -> Option<RenderCommand> {
    process::ProcessLayer2d::new(canvas)
      .process_layer(self)
      .and_then(|cmd| {
        canvas.upload_render_command(&cmd);
        Some(cmd)
      })
  }
}

/// Describe render the text as single line or break as multiple lines.
#[derive(Debug, Clone, PartialEq)]
pub enum LineWrap {
  /// Renders a single line from left-to-right according to the inner
  /// alignment. Hard breaking will end the line, partially hitting the width
  /// bound will end the line.
  SingleLine,
  /// Renders multiple lines from left-to-right according to the inner
  /// alignment. Hard breaking characters will cause advancement to another
  /// line. A characters hitting the width bound will also cause another line
  /// to start.
  Wrap,
}

/// Describes how to layout the text.
#[derive(Debug, Clone)]
pub struct TextLayout {
  /// horizontal alignment preference
  pub h_align: HorizontalAlign,
  /// vertical alignment preference
  pub v_align: VerticalAlign,
  /// text render in single line a multiple lins.
  pub wrap: LineWrap,
}

#[derive(Debug, Clone)]
pub struct Text<'a> {
  /// Text to render
  pub text: &'a str,
  /// Text pixel size.
  pub font_size: f32,
  /// The font info
  pub font: FontInfo,
}

#[derive(Debug, Clone)]
pub(crate) struct RenderAttr {
  pub(crate) count: Count,
  pub(crate) transform: Transform,
  pub(crate) style: FillStyle,
  pub(crate) align_bounds: Rect,
}
#[derive(Debug, Clone, PartialEq)]
pub enum FillStyle {
  Color(Color),
  Image,    // todo
  Gradient, // todo,
}

impl From<Color> for FillStyle {
  #[inline]
  fn from(c: Color) -> Self { FillStyle::Color(c) }
}

#[derive(Debug, Clone)]
pub(crate) struct Vertex {
  pub(crate) pixel_coords: Point,
  pub(crate) texture_coords: Point,
}

#[derive(Debug, Clone)]
pub struct RenderCommand {
  pub(crate) geometry: VertexBuffers<Vertex, u32>,
  pub(crate) attrs: Vec<RenderAttr>,
}

impl<'a> Rendering2DLayer<'a> {
  fn current_state(&self) -> &State {
    self
      .state_stack
      .last()
      .expect("Must have one state in stack!")
  }

  fn current_state_mut(&mut self) -> &mut State {
    self
      .state_stack
      .last_mut()
      .expect("Must have one state in stack!")
  }

  fn command_from_path<'l>(&self, path: Path, stroke_or_fill: bool) -> Command<'l> {
    self.command_from(|state| {
      let stroke_width = if stroke_or_fill {
        Some(self.current_state().line_width)
      } else {
        None
      };
      CommandInfo::Path {
        path,
        style: state.style.clone(),
        stroke_width,
      }
    })
  }

  fn command_from_text<'l>(&self, text: &'l str, max_width: Option<f32>) -> Command<'l> {
    self.command_from(|state| CommandInfo::SimpleText {
      text: Text {
        text,
        font_size: state.font.font_size,
        font: state.font.clone(),
      },
      style: state.style.clone(),
      max_width,
    })
  }

  #[inline]
  fn command_from<'l, F: FnOnce(&State) -> CommandInfo<'l>>(&self, ctor_info: F) -> Command<'l> {
    let state = self.current_state();
    Command {
      info: ctor_info(state),
      transform: state.transform,
    }
  }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FontInfo {
  /// CSS Fonts Level 3 specification of family.
  family: String,
  props: FontProperties,
  font_size: f32,
}

#[derive(Clone, Debug)]
struct State {
  transform: Transform,
  line_width: f32,
  style: FillStyle,
  font: FontInfo,
}

#[derive(Debug, Clone)]
enum CommandInfo<'a> {
  Path {
    path: Path,
    style: FillStyle,
    // A some value means stroke with the line width in it, None means fill.
    stroke_width: Option<f32>,
  },
  SimpleText {
    text: Text<'a>,
    style: FillStyle,
    max_width: Option<f32>,
  },
  ComplexTexts {
    texts: Vec<(Text<'a>, Color)>,
    bounds: Option<Rect>,
    layout: Option<TextLayout>,
  },
  ComplexTextsByStyle {
    texts: Vec<Text<'a>>,
    bounds: Option<Rect>,
    layout: Option<TextLayout>,
    style: FillStyle,
  },
}

#[derive(Debug, Clone)]
struct Command<'a> {
  info: CommandInfo<'a>,
  transform: Transform,
}

/// An RAII implementation of a "scoped state" of the render layer. When this
/// structure is dropped (falls out of scope), changed state will auto restore.
/// The data can be accessed through this guard via its Deref and DerefMut
/// implementations.
pub struct LayerGuard<'a, 'b>(&'a mut Rendering2DLayer<'b>);

impl<'a, 'b> Drop for LayerGuard<'a, 'b> {
  #[inline]
  fn drop(&mut self) {
    debug_assert!(!self.0.state_stack.is_empty());
    self.0.state_stack.pop();
  }
}

impl<'a, 'b> Deref for LayerGuard<'a, 'b> {
  type Target = Rendering2DLayer<'b>;
  #[inline]
  fn deref(&self) -> &Self::Target { &self.0 }
}

impl<'a, 'b> DerefMut for LayerGuard<'a, 'b> {
  #[inline]
  fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}

impl Vertex {
  fn from_stroke_vertex(v: StrokeVertex) -> Self {
    Self {
      pixel_coords: Point::from_untyped(v.position()),
      texture_coords: Point::new(-1., -1.),
    }
  }

  fn from_fill_vertex(v: FillVertex) -> Self {
    Self {
      pixel_coords: Point::from_untyped(v.position()),
      texture_coords: Point::new(-1., -1.),
    }
  }
}

impl Default for FillStyle {
  #[inline]
  fn default() -> Self { FillStyle::Color(Color::WHITE) }
}

impl Default for State {
  #[inline]
  fn default() -> Self { Self::new() }
}

impl State {
  pub fn new() -> Self {
    Self {
      transform: Transform::row_major(1., 0., 0., 1., 0., 0.),
      style: FillStyle::Color(Color::BLACK),
      line_width: 1.,
      font: FontInfo::new(),
    }
  }
}

impl Default for TextLayout {
  #[inline]
  fn default() -> Self { Self::new() }
}

impl TextLayout {
  const fn new() -> Self {
    Self {
      v_align: VerticalAlign::Top,
      h_align: HorizontalAlign::Left,
      wrap: LineWrap::SingleLine,
    }
  }
}

impl From<TextLayout> for glyph_brush::Layout<glyph_brush::BuiltInLineBreaker> {
  fn from(layout: TextLayout) -> Self {
    let TextLayout {
      h_align,
      v_align,
      wrap,
    } = layout;
    let line_breaker = glyph_brush::BuiltInLineBreaker::default();
    if LineWrap::SingleLine == wrap {
      glyph_brush::Layout::SingleLine {
        h_align,
        v_align,
        line_breaker,
      }
    } else {
      glyph_brush::Layout::Wrap {
        h_align,
        v_align,
        line_breaker,
      }
    }
  }
}

impl<'a> Text<'a> {
  fn to_glyph_text(&self, canvas: &mut Canvas) -> glyph_brush::Text<'a, GlyphStatistics> {
    let Text {
      text,
      font,
      font_size,
    } = self;
    let font_id = canvas
      .select_best_match(font.family.as_str(), &font.props)
      .map(|f| f.id)
      .unwrap_or_else(|_| canvas.default_font().id);

    glyph_brush::Text {
      text,
      font_id,
      scale: (*font_size).into(),
      extra: <_>::default(),
    }
  }
}

impl FontInfo {
  pub fn new() -> Self {
    FontInfo {
      family: DEFAULT_FONT_FAMILY.to_owned(),
      props: <_>::default(),
      font_size: 14.,
    }
  }

  #[inline]
  pub fn with_family(mut self, family: String) -> Self {
    self.family = family;
    self
  }

  #[inline]
  pub fn with_weight(mut self, weight: FontWeight) -> Self {
    self.props.weight(weight);
    self
  }

  #[inline]
  pub fn with_style(mut self, style: FontStyle) -> Self {
    self.props.style(style);
    self
  }

  #[inline]
  pub fn with_stretch(mut self, stretch: FontStretch) -> Self {
    self.props.stretch(stretch);
    self
  }

  #[inline]
  pub fn with_font_size(mut self, font_size: f32) -> Self {
    self.font_size = font_size;
    self
  }
}

#[cfg(test)]
mod test {
  use super::*;
  use crate::{Size, *};

  use futures::executor::block_on;

  #[test]
  fn save_guard() {
    let mut layer = Rendering2DLayer::new();
    {
      let mut paint = layer.save();
      let t = Transform::row_major(1., 1., 1., 1., 1., 1.);
      paint.set_transform(t.clone());
      assert_eq!(&t, paint.get_transform());
      {
        let mut p2 = paint.save();
        let t2 = Transform::row_major(2., 2., 2., 2., 2., 2.);
        p2.set_transform(t2);
        assert_eq!(&t2, p2.get_transform());
      }
      assert_eq!(&t, paint.get_transform());
    }
    assert_eq!(
      &Transform::row_major(1., 0., 0., 1., 0., 0.),
      layer.get_transform()
    );
  }

  #[test]
  #[ignore = "gpu need"]
  fn buffer() {
    let mut layer = Rendering2DLayer::new();
    let mut canvas = block_on(Canvas::new(DeviceSize::new(400, 400)));
    let mut builder = Path::builder();
    builder.add_rectangle(
      &euclid::Rect::from_size((100., 100.).into()),
      Winding::Positive,
    );
    let path = builder.build();
    layer.stroke_path(path.clone());
    layer.fill_path(path);
    let buffer = layer.finish(&mut canvas).unwrap();

    assert!(!buffer.geometry.vertices.is_empty());
    assert_eq!(buffer.attrs.len(), 1);
  }

  #[test]
  #[ignore = "gpu need"]
  fn path_merge() {
    let mut layer = Rendering2DLayer::new();

    let mut canvas = block_on(Canvas::new(DeviceSize::new(400, 400)));

    let sample_path = Path::builder().build();
    // The stroke path both style and line width same should be merge.
    layer.stroke_path(sample_path.clone());
    layer.stroke_path(sample_path.clone());
    assert_eq!(layer.clone().finish(&mut canvas).unwrap().attrs.len(), 1);

    // Different line width with same color pen can be merged.
    layer.set_line_width(2.);
    layer.stroke_path(sample_path.clone());
    assert_eq!(layer.clone().finish(&mut canvas).unwrap().attrs.len(), 1);

    // Different color can't be merged.
    layer.set_style(FillStyle::Color(Color::YELLOW));
    layer.fill_path(sample_path.clone());
    assert_eq!(layer.clone().finish(&mut canvas).unwrap().attrs.len(), 2);

    // Different type style can't be merged
    layer.set_style(FillStyle::Image);
    layer.fill_path(sample_path.clone());
    layer.stroke_path(sample_path);

    // fixme: image not support now
    // assert_eq!(layer.clone().finish(&mut canvas).unwrap().attrs.len(), 4);
  }

  fn canvas() -> Canvas<crate::canvas::surface::TextureSurface> {
    block_on(Canvas::new(DeviceSize::new(400, 400)))
  }

  #[test]
  #[ignore = "gpu need"]
  fn fill_text_hello() {
    let mut canvas = canvas();

    let mut layer = canvas.new_2d_layer();
    let font = FontInfo::new();
    layer.set_font(font);
    layer.fill_text("Nice to meet you!", None);
    canvas.compose_2d_layer(layer);
    canvas.submit();

    unit_test::assert_canvas_eq!(canvas, "./test_imgs/text_hello.png");
  }

  #[test]
  #[ignore = "gpu need"]
  fn fill_text_complex() {
    let mut canvas = canvas();
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

    canvas.compose_2d_layer(layer);
    canvas.submit();

    unit_test::assert_canvas_eq!(canvas, "./test_imgs/complex_text.png");
  }

  #[test]
  #[ignore = "gpu need"]
  fn fill_text_complex_single_style() {
    let mut canvas = canvas();
    let arial = FontInfo::new().with_family("Arial".to_owned());
    let serif = FontInfo::new();
    let mut layer = canvas.new_2d_layer();

    layer.set_style(FillStyle::Color(Color::GRAY));
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

    canvas.compose_2d_layer(layer);
    canvas.submit();

    unit_test::assert_canvas_eq!(canvas, "./test_imgs/complex_text_single_style.png");
  }

  #[test]
  #[ignore = "gpu need"]
  fn update_texture_on_processing() {
    let text = include_str!("../fonts/loads-of-unicode.txt");
    let mut canvas = canvas();
    let crate_root = env!("CARGO_MANIFEST_DIR").to_owned();
    canvas
      .load_font_from_path(crate_root.clone() + "/fonts/DejaVuSans.ttf", 0)
      .unwrap();
    let deja = FontInfo::new().with_family("DejaVu Sans".to_owned());
    let mut layer = canvas.new_2d_layer();
    layer.fill_complex_texts_by_style(
      vec![Text {
        text,
        font: deja,
        font_size: 36.,
      }],
      Some(Rect::from_size(Size::new(1600., 1600.))),
      Some(TextLayout {
        h_align: HorizontalAlign::Right,
        v_align: VerticalAlign::Bottom,
        wrap: LineWrap::Wrap,
      }),
    );

    let buffer = layer.finish(&mut canvas);
    assert!(buffer.is_none());

    canvas.submit();

    unit_test::assert_canvas_eq!(canvas, "./test_imgs/texture_cache_update.png");
  }
}
