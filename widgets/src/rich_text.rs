use std::{
  any::Any,
  cell::{Ref, RefCell},
};

use ribir_core::{
  prelude::*,
  text::{CaretAffinity, LineHeight, TextHitResult, TextRange, single_style_paragraph_style},
};
use rxrust::subscription::BoxedSubscription;
use smallvec::SmallVec;

pub type SpanStyleValue<T> = Option<PipeValue<T>>;

pub type RichTextSpanData = Resource<dyn Any>;

#[derive(Debug, Clone, PartialEq)]
pub struct RichTextSegmentTapData {
  pub index: usize,
  pub range: TextRange,
  pub text: CowArc<str>,
  pub data: Option<RichTextSpanData>,
}

impl RichTextSegmentTapData {
  #[inline]
  pub fn data(&self) -> Option<&RichTextSpanData> { self.data.as_ref() }

  #[inline]
  pub fn data_as<T: Any>(&self) -> Option<&T> { self.data.as_ref()?.downcast_ref::<T>() }
}

pub type RichTextSegmentTapEvent = CustomEvent<RichTextSegmentTapData>;

#[derive(Default)]
pub struct Span {
  pub text: TextValue,
  pub data: Option<RichTextSpanData>,
  pub font: SpanStyleValue<FontFace>,
  pub font_size: SpanStyleValue<f32>,
  pub letter_spacing: SpanStyleValue<f32>,
  pub text_line_height: SpanStyleValue<LineHeight>,
  pub text_decoration: SpanStyleValue<TextDecorationStyle>,
  pub foreground: SpanStyleValue<Brush>,
}

#[derive(Default)]
pub struct SpanDeclarer {
  text: Option<TextValue>,
  data: Option<RichTextSpanData>,
  font: SpanStyleValue<FontFace>,
  font_size: SpanStyleValue<f32>,
  letter_spacing: SpanStyleValue<f32>,
  text_line_height: SpanStyleValue<LineHeight>,
  text_decoration: SpanStyleValue<TextDecorationStyle>,
  foreground: SpanStyleValue<Brush>,
}

impl Declare for Span {
  type Builder = SpanDeclarer;

  #[inline]
  fn declarer() -> Self::Builder { <_>::default() }
}

impl ObjDeclarer for SpanDeclarer {
  type Target = Span;

  #[inline]
  #[track_caller]
  fn finish(self) -> Self::Target {
    Span {
      text: self
        .text
        .expect("Required field `text: TextValue` not set"),
      data: self.data,
      font: self.font,
      font_size: self.font_size,
      letter_spacing: self.letter_spacing,
      text_line_height: self.text_line_height,
      text_decoration: self.text_decoration,
      foreground: self.foreground,
    }
  }
}

impl SpanDeclarer {
  #[inline]
  pub fn with_text<K: ?Sized>(&mut self, v: impl RInto<TextValue, K>) -> &mut Self {
    self.text = Some(v.r_into());
    self
  }

  #[inline]
  pub fn with_data<T: Any>(&mut self, value: T) -> &mut Self {
    self.data = Some(Resource::new(value).into_any());
    self
  }

  #[inline]
  pub fn with_any_data(&mut self, data: RichTextSpanData) -> &mut Self {
    self.data = Some(data);
    self
  }

  #[inline]
  pub fn with_font<K: ?Sized>(&mut self, v: impl RInto<PipeValue<FontFace>, K>) -> &mut Self {
    self.font = Some(v.r_into());
    self
  }

  #[inline]
  pub fn with_font_size<K: ?Sized>(&mut self, v: impl RInto<PipeValue<f32>, K>) -> &mut Self {
    self.font_size = Some(v.r_into());
    self
  }

  #[inline]
  pub fn with_letter_spacing<K: ?Sized>(&mut self, v: impl RInto<PipeValue<f32>, K>) -> &mut Self {
    self.letter_spacing = Some(v.r_into());
    self
  }

  #[inline]
  pub fn with_text_line_height<K: ?Sized>(
    &mut self, v: impl RInto<PipeValue<LineHeight>, K>,
  ) -> &mut Self {
    self.text_line_height = Some(v.r_into());
    self
  }

  #[inline]
  pub fn with_text_decoration<K: ?Sized>(
    &mut self, v: impl RInto<PipeValue<TextDecorationStyle>, K>,
  ) -> &mut Self {
    self.text_decoration = Some(v.r_into());
    self
  }

  #[inline]
  pub fn with_foreground<K: ?Sized>(&mut self, v: impl RInto<PipeValue<Brush>, K>) -> &mut Self {
    self.foreground = Some(v.r_into());
    self
  }
}

#[derive(Debug, Clone, PartialEq, Default)]
struct SpanSnapshot {
  pub text: CowArc<str>,
  pub data: Option<RichTextSpanData>,
  pub font: Option<FontFace>,
  pub font_size: Option<f32>,
  pub letter_spacing: Option<f32>,
  pub text_line_height: Option<LineHeight>,
  pub text_decoration: Option<TextDecorationStyle>,
  pub foreground: Option<Brush>,
}

impl Span {
  #[inline]
  pub fn new(text: impl Into<CowArc<str>>) -> Self {
    Self { text: PipeValue::Value(text.into()), ..Default::default() }
  }

  fn push_fragment(
    self, this: &impl StateWriter<Value = RichText>, subscriptions: &mut RichTextSubscriptions,
  ) {
    let Self {
      text,
      data,
      font,
      font_size,
      letter_spacing,
      text_line_height,
      text_decoration,
      foreground,
    } = self;
    let (text, text_stream) = text.unzip();
    let (font, font_stream) = unzip_optional_pipe(font);
    let (font_size, font_size_stream) = unzip_optional_pipe(font_size);
    let (letter_spacing, letter_spacing_stream) = unzip_optional_pipe(letter_spacing);
    let (text_line_height, text_line_height_stream) = unzip_optional_pipe(text_line_height);
    let (text_decoration, text_decoration_stream) = unzip_optional_pipe(text_decoration);
    let (foreground, foreground_stream) = unzip_optional_pipe(foreground);

    let index = append_fragment(
      this,
      RichTextFragment::Span(SpanSnapshot {
        text,
        data,
        font,
        font_size,
        letter_spacing,
        text_line_height,
        text_decoration,
        foreground,
      }),
    );

    push_fragment_subscription(subscriptions, this, index, text_stream, set_span_text);
    push_fragment_subscription(subscriptions, this, index, font_stream, set_span_font);
    push_fragment_subscription(subscriptions, this, index, font_size_stream, set_span_font_size);
    push_fragment_subscription(
      subscriptions,
      this,
      index,
      letter_spacing_stream,
      set_span_letter_spacing,
    );
    push_fragment_subscription(
      subscriptions,
      this,
      index,
      text_line_height_stream,
      set_span_text_line_height,
    );
    push_fragment_subscription(
      subscriptions,
      this,
      index,
      text_decoration_stream,
      set_span_text_decoration,
    );
    push_fragment_subscription(subscriptions, this, index, foreground_stream, set_span_foreground);
  }
}

fn unzip_optional_pipe<T: 'static>(
  value: Option<PipeValue<T>>,
) -> (Option<T>, Option<ValueStream<T>>) {
  value.map_or((None, None), |value| {
    let (value, stream) = value.unzip();
    (Some(value), stream)
  })
}

impl SpanSnapshot {
  #[inline]
  fn has_style_override(&self, inherited_decoration: Option<&TextDecorationStyle>) -> bool {
    self.font.is_some()
      || self.font_size.is_some()
      || self.letter_spacing.is_some()
      || self.text_line_height.is_some()
      || self.foreground.is_some()
      || self
        .decoration_style(inherited_decoration)
        .is_some()
  }

  #[inline]
  fn decoration_style(
    &self, inherited_decoration: Option<&TextDecorationStyle>,
  ) -> Option<TextDecorationStyle> {
    let decoration = self
      .text_decoration
      .as_ref()
      .map(|style| {
        if style.decoration.is_empty() {
          inherited_decoration
            .map(|style| style.decoration)
            .unwrap_or_default()
        } else {
          style.decoration
        }
      })
      .or_else(|| inherited_decoration.map(|style| style.decoration))
      .unwrap_or_default();
    if decoration.is_empty() {
      return None;
    }

    Some(TextDecorationStyle {
      decoration,
      decoration_color: self
        .text_decoration
        .as_ref()
        .and_then(|style| style.decoration_color)
        .or_else(|| inherited_decoration.and_then(|style| style.decoration_color)),
    })
  }

  #[inline]
  fn span_style(&self, inherited_decoration: Option<&TextDecorationStyle>) -> SpanStyle {
    SpanStyle {
      font: self.font.clone().map(|face| FontRequest { face }),
      font_size: self.font_size,
      letter_spacing: self.letter_spacing,
      line_height: self.text_line_height,
      brush: self.foreground.clone(),
      decoration: self.decoration_style(inherited_decoration),
    }
  }

  fn append_to(
    &self, builder: &mut AttributedTextBuilder, inherited_decoration: Option<&TextDecorationStyle>,
  ) {
    if self.has_style_override(inherited_decoration) {
      builder.write_styled_text(&*self.text, self.span_style(inherited_decoration));
    } else {
      builder.write_text(&*self.text);
    }
  }
}

#[derive(Template)]
pub enum RichTextChild {
  Text(TextValue),
  Span(Box<Span>),
}

#[derive(Debug, Clone, PartialEq)]
enum RichTextFragment {
  Text(CowArc<str>),
  Span(SpanSnapshot),
}

type RichTextSubscriptions = SmallVec<[BoxedSubscription; 1]>;

fn push_fragment_subscription<T: 'static>(
  subscriptions: &mut RichTextSubscriptions, this: &impl StateWriter<Value = RichText>,
  index: usize, stream: Option<ValueStream<T>>, update: fn(&mut RichTextFragment, T),
) {
  if let Some(stream) = stream {
    let writer = this.clone_boxed_writer();
    subscriptions.push(BoxedSubscription::new(stream.subscribe(move |value| {
      let mut rich_text = writer.write();
      update(
        rich_text
          .fragments
          .get_mut(index)
          .expect("rich text fragment should exist"),
        value,
      );
    })));
  }
}

fn append_fragment(this: &impl StateWriter<Value = RichText>, fragment: RichTextFragment) -> usize {
  let mut rich_text = this.write();
  let index = rich_text.fragments.len();
  rich_text.fragments.push(fragment);
  index
}

fn unsubscribe_subscriptions(subscriptions: RichTextSubscriptions) {
  subscriptions
    .into_iter()
    .for_each(|subscription| subscription.unsubscribe());
}

fn set_fragment_text(fragment: &mut RichTextFragment, text: CowArc<str>) {
  match fragment {
    RichTextFragment::Text(current) => *current = text,
    RichTextFragment::Span(_) => unreachable!("expected a plain text fragment"),
  }
}

fn set_span_text(fragment: &mut RichTextFragment, text: CowArc<str>) {
  match fragment {
    RichTextFragment::Span(span) => span.text = text,
    RichTextFragment::Text(_) => unreachable!("expected a span fragment"),
  }
}

fn set_span_font(fragment: &mut RichTextFragment, font: FontFace) {
  match fragment {
    RichTextFragment::Span(span) => span.font = Some(font),
    RichTextFragment::Text(_) => unreachable!("expected a span fragment"),
  }
}

fn set_span_font_size(fragment: &mut RichTextFragment, font_size: f32) {
  match fragment {
    RichTextFragment::Span(span) => span.font_size = Some(font_size),
    RichTextFragment::Text(_) => unreachable!("expected a span fragment"),
  }
}

fn set_span_letter_spacing(fragment: &mut RichTextFragment, letter_spacing: f32) {
  match fragment {
    RichTextFragment::Span(span) => span.letter_spacing = Some(letter_spacing),
    RichTextFragment::Text(_) => unreachable!("expected a span fragment"),
  }
}

fn set_span_text_line_height(fragment: &mut RichTextFragment, text_line_height: LineHeight) {
  match fragment {
    RichTextFragment::Span(span) => span.text_line_height = Some(text_line_height),
    RichTextFragment::Text(_) => unreachable!("expected a span fragment"),
  }
}

fn set_span_text_decoration(fragment: &mut RichTextFragment, text_decoration: TextDecorationStyle) {
  match fragment {
    RichTextFragment::Span(span) => span.text_decoration = Some(text_decoration),
    RichTextFragment::Text(_) => unreachable!("expected a span fragment"),
  }
}

fn set_span_foreground(fragment: &mut RichTextFragment, foreground: Brush) {
  match fragment {
    RichTextFragment::Span(span) => span.foreground = Some(foreground),
    RichTextFragment::Text(_) => unreachable!("expected a span fragment"),
  }
}

fn push_child_fragment(
  child: RichTextChild, this: &impl StateWriter<Value = RichText>,
  subscriptions: &mut RichTextSubscriptions,
) {
  match child {
    RichTextChild::Text(text) => {
      let (text, stream) = text.unzip();
      let index = append_fragment(this, RichTextFragment::Text(text));
      push_fragment_subscription(subscriptions, this, index, stream, set_fragment_text);
    }
    RichTextChild::Span(span) => span.push_fragment(this, subscriptions),
  }
}

fn fragments_from_children(
  this: &impl StateWriter<Value = RichText>, children: Vec<RichTextChild>,
) -> RichTextSubscriptions {
  let mut subscriptions = RichTextSubscriptions::default();
  this.write().fragments.clear();
  children
    .into_iter()
    .for_each(|child| push_child_fragment(child, this, &mut subscriptions));
  subscriptions
}

fn append_declared_fragments(
  fragments: &[RichTextFragment], default_decoration: Option<&TextDecorationStyle>,
) -> AttributedText {
  if fragments.is_empty() {
    return AttributedText::default();
  }

  let mut builder = AttributedText::builder();
  fragments
    .iter()
    .for_each(|fragment| match fragment {
      RichTextFragment::Text(text) => {
        if let Some(default_decoration) =
          default_decoration.filter(|style| !style.decoration.is_empty())
        {
          builder.write_styled_text(
            &**text,
            SpanStyle { decoration: Some(default_decoration.clone()), ..Default::default() },
          );
        } else {
          builder.write_text(&**text);
        }
      }
      RichTextFragment::Span(span) => span.append_to(&mut builder, default_decoration),
    });
  builder.build()
}

fn segment_from_hit(
  fragments: &[RichTextFragment], hit: TextHitResult,
) -> Option<RichTextSegmentTapData> {
  if !hit.is_inside {
    return None;
  }

  let byte = hit.caret.byte.0;
  let prefer_next = matches!(hit.caret.affinity, CaretAffinity::Downstream);
  let mut start = 0;
  let mut last_non_empty = None;

  for (index, fragment) in fragments.iter().enumerate() {
    let text = fragment.text();
    let len = text.len();
    let end = start + len;

    if len > 0 {
      if byte < end || (byte == end && !prefer_next) {
        return Some(RichTextSegmentTapData {
          index,
          range: TextRange::new(start, end),
          text: text.clone(),
          data: fragment.data().cloned(),
        });
      }
      last_non_empty = Some((index, start, end, text.clone(), fragment.data().cloned()));
    }

    start = end;
  }

  if byte == start && !prefer_next {
    return last_non_empty.map(|(index, start, end, text, data)| RichTextSegmentTapData {
      index,
      range: TextRange::new(start, end),
      text,
      data,
    });
  }

  None
}

impl RichTextFragment {
  fn text(&self) -> &CowArc<str> {
    match self {
      RichTextFragment::Text(text) => text,
      RichTextFragment::Span(span) => &span.text,
    }
  }

  fn data(&self) -> Option<&RichTextSpanData> {
    match self {
      RichTextFragment::Text(_) => None,
      RichTextFragment::Span(span) => span.data.as_ref(),
    }
  }
}

/// A multi-style text widget that renders one logical string with styled
/// ranges.
///
/// Like [`Text`], `RichText` uses inherited `text_style`, `text_align`, and
/// `foreground` built-in widgets for its paragraph defaults. Individual spans
/// only override the specific fields they set. Content comes entirely from the
/// declared text and span children. `text_decoration` is inherited the same
/// way, while spans can override decoration flags or just the decoration color.
///
/// # Example
///
/// ```rust
/// use ribir::prelude::*;
///
/// fn_widget! {
///   @RichText {
///     @ { "Hello " }
///     @Span { text: "Ribir", foreground: Color::RED }
///     @ { "!" }
///   }
/// };
/// ```
///
/// To react to a tapped segment, listen for [`RichTextSegmentTapEvent`] and
/// update your own state. This keeps RichText aligned with Ribir's controlled
/// interaction model: the widget emits intent, and your model drives the visual
/// update.
///
/// ```rust
/// use ribir::prelude::*;
///
/// struct LinkMeta {
///   href: CowArc<str>,
///   visited: bool,
/// }
/// let meta = Stateful::new(LinkMeta { href: "https://ribir.org".into(), visited: false });
/// fn_widget! {
///   @RichText {
///     on_custom: move |e: &mut RichTextSegmentTapEvent| {
///       if let Some(meta) = e.data().data_as::<Stateful<LinkMeta>>() {
///         println!("open {}", meta.read().href);
///         meta.write().visited = true;
///       }
///     },
///     @Span {
///       text: "Ribir",
///       foreground: pipe! {
///         if $read(meta).visited {
///           Brush::from(Color::RED)
///         } else {
///           Brush::from(Color::BLUE)
///         }
///       },
///       data: meta,
///     }
///   }
/// };
/// ```
#[declare]
#[derive(Default)]
pub struct RichText {
  #[declare(skip)]
  fragments: Vec<RichTextFragment>,

  #[declare(skip)]
  layout: RefCell<Option<ParagraphLayoutRef>>,
}

fn rich_text_layout(
  text: AttributedText, text_style: &TextStyle, text_align: TextAlign, clamp: BoxClamp,
) -> ParagraphLayoutRef {
  let paragraph_style = single_style_paragraph_style(text_style, text_align);
  let paragraph = AppCtx::text_services().paragraph(text);
  paragraph.layout(text_style, &paragraph_style, clamp)
}

impl Render for RichText {
  fn measure(&self, clamp: BoxClamp, ctx: &mut MeasureCtx) -> Size {
    let style = Provider::of::<TextStyle>(ctx).unwrap();
    let text_decoration = Provider::of::<TextDecorationStyle>(ctx)
      .map(|style| (*style).clone())
      .filter(|style| !style.decoration.is_empty());
    let text_align = Provider::of::<TextAlign>(ctx)
      .map(|align| *align)
      .unwrap_or_default();
    let text = self.combined_text(text_decoration.as_ref());
    let layout = rich_text_layout(text, &style, text_align, clamp);
    let size = layout.size();
    *self.layout.borrow_mut() = Some(layout);
    size
  }

  #[inline]
  fn size_affected_by_child(&self) -> bool { false }

  fn paint(&self, ctx: &mut PaintingCtx) {
    let style = Provider::of::<PaintingStyle>(ctx).map(|p| p.clone());
    let Some(layout) = self.layout.borrow().clone() else {
      return;
    };
    let brush = ctx.painter().fill_brush().clone();
    if !brush.is_visible() {
      return;
    }

    let payload = Resource::new(layout.draw_payload().clone());
    let rect = layout.draw_payload().bounds;
    let _ = style;
    ctx.painter().draw_text_payload(payload, rect);
  }

  #[cfg(feature = "debug")]
  fn debug_name(&self) -> std::borrow::Cow<'static, str> { std::borrow::Cow::Borrowed("rich_text") }

  #[cfg(feature = "debug")]
  fn debug_properties(&self) -> serde_json::Value {
    let text = self.combined_text(None);
    serde_json::json!({
      "text": &*text.text,
      "span_count": text.spans.len(),
      "fragment_count": self.fragments.len(),
    })
  }
}

impl ComposeChild<'static> for RichText {
  type Child = Vec<RichTextChild>;

  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'static> {
    let subscriptions = fragments_from_children(&this, child);
    fn_widget! {
      @FatObj {
        on_tap: move |e| {
          if let Some(segment) = $read(this).hit_test_segment(e.position()) {
            e.window().bubble_custom_event(e.current_target(), segment);
          }
        },
        on_disposed: move |_| {
          unsubscribe_subscriptions(subscriptions);
        },
        @ { this.clone_boxed_watcher() }
      }
    }
    .into_widget()
  }
}

impl RichText {
  pub fn layout(&self) -> Option<Ref<'_, ParagraphLayoutRef>> {
    Ref::filter_map(self.layout.borrow(), |v| v.as_ref()).ok()
  }

  pub fn hit_test_segment(&self, pos: Point) -> Option<RichTextSegmentTapData> {
    let layout = self.layout()?;
    segment_from_hit(&self.fragments, layout.hit_test_point(pos))
  }

  #[inline]
  fn combined_text(&self, default_decoration: Option<&TextDecorationStyle>) -> AttributedText {
    append_declared_fragments(&self.fragments, default_decoration)
  }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
  use ribir_core::{
    prelude::*,
    test_helper::*,
    text::{LineHeight, TextRange},
  };

  use crate::prelude::*;

  fn dejavu_face() -> FontFace {
    FontFace { families: Box::new([FontFamily::Name("DejaVu Sans".into())]), ..Default::default() }
  }

  fn register_test_font() {
    let path = env!("CARGO_MANIFEST_DIR").to_owned() + "/../fonts/DejaVuSans.ttf";
    let _ = AppCtx::text_services().register_font_file(std::path::Path::new(&path));
  }

  fn test_text_style() -> TextStyle {
    TextStyle {
      font_size: 16.,
      font_face: dejavu_face(),
      letter_space: 0.,
      line_height: LineHeight::Px(16.),
      overflow: TextOverflow::Overflow,
    }
  }

  fn last_text_command(frame: Frame) -> TextCommand {
    frame
      .commands
      .into_iter()
      .find_map(|cmd| match cmd {
        PaintCommand::Text(text) => Some(text),
        _ => None,
      })
      .expect("expected a text command")
  }

  fn decoration_kinds(cmd: &TextCommand) -> Vec<TextDecoration> {
    cmd
      .payload
      .decorations
      .iter()
      .map(|decoration| decoration.decoration)
      .collect()
  }

  fn tap_on(wnd: &Window, pos: Point) {
    wnd.process_cursor_move(pos);
    wnd.process_mouse_press(Box::new(DummyDeviceId), MouseButtons::PRIMARY);
    wnd.process_mouse_release(Box::new(DummyDeviceId), MouseButtons::PRIMARY);
  }

  fn tap_pos_for_run(cmd: &TextCommand, run_idx: usize) -> Point {
    let run = &cmd.payload.runs[run_idx];
    let glyph = run
      .glyphs
      .first()
      .expect("expected at least one glyph");
    Point::new(
      glyph.baseline_origin.x + glyph.advance.x.max(1.) * 0.5,
      cmd.payload.bounds.center().y,
    )
  }

  #[test]
  fn rich_text_composes_declared_spans() {
    reset_test_env!();
    register_test_font();

    let mut wnd = TestWindow::new_with_size(
      fn_widget! {
        @RichText {
          text_style: test_text_style(),
          foreground: Color::WHITE,
          @ { "plain " }
          @Span {
            text: "accent",
            font: dejavu_face(),
            font_size: 16.,
            letter_spacing: 0.,
            foreground: Color::RED,
          }
          @ { " tail" }
        }
      },
      Size::new(200., 40.),
    );

    wnd.draw_frame();
    let cmd = last_text_command(wnd.take_last_frame().expect("expected a frame"));

    assert_eq!(cmd.payload.runs.len(), 3);
    assert_eq!(cmd.payload.runs[0].brush, None);
    assert_eq!(cmd.payload.runs[1].brush, Some(Color::RED.into()));
    assert_eq!(cmd.payload.runs[2].brush, None);
  }

  #[test]
  fn rich_text_center_alignment_uses_content_width_for_measurement() {
    reset_test_env!();
    register_test_font();

    let wnd_size = Size::new(200., 40.);

    let mut start_wnd = TestWindow::new_with_size(
      fn_widget! {
        @RichText {
          text_style: test_text_style(),
          foreground: Color::WHITE,
          text_align: TextAlign::Start,
          @ { "All" }
        }
      },
      wnd_size,
    );
    start_wnd.draw_frame();
    let start = last_text_command(
      start_wnd
        .take_last_frame()
        .expect("expected a frame"),
    );

    let mut center_wnd = TestWindow::new_with_size(
      fn_widget! {
        @RichText {
          text_style: test_text_style(),
          foreground: Color::WHITE,
          text_align: TextAlign::Center,
          @ { "All" }
        }
      },
      wnd_size,
    );
    center_wnd.draw_frame();
    let center = last_text_command(
      center_wnd
        .take_last_frame()
        .expect("expected a frame"),
    );

    assert!((center.paint_bounds.width() - start.paint_bounds.width()).abs() < 1.);
  }

  #[test]
  fn rich_text_pipe_keeps_declared_spans() {
    reset_test_env!();
    register_test_font();

    let (content, writer) = split_value("Hi");
    let mut wnd = TestWindow::new_with_size(
      fn_widget! {
        @RichText {
          text_style: test_text_style(),
          foreground: Color::WHITE,
          @{ pipe!(*$read(content)) }
          @Span {
            text: "!",
            font: dejavu_face(),
            font_size: 16.,
            letter_spacing: 0.,
            foreground: Color::GREEN,
          }
        }
      },
      Size::new(200., 40.),
    );

    wnd.draw_frame();
    let initial = last_text_command(
      wnd
        .take_last_frame()
        .expect("expected initial frame"),
    );
    assert_eq!(initial.payload.runs.len(), 2);

    *writer.write() = "Hello";
    wnd.draw_frame();

    let updated = last_text_command(
      wnd
        .take_last_frame()
        .expect("expected updated frame"),
    );
    assert_eq!(updated.payload.runs.len(), 2);
    assert_eq!(updated.payload.runs[1].brush, Some(Color::GREEN.into()));
    assert!(updated.payload.bounds.width() > initial.payload.bounds.width());
  }

  #[test]
  fn rich_text_plain_text_child_pipe_updates_payload() {
    reset_test_env!();
    register_test_font();

    let (content, writer) = split_value::<CowArc<str>>(CowArc::from("Hi"));
    let mut wnd = TestWindow::new_with_size(
      fn_widget! {
        @RichText {
          text_style: test_text_style(),
          foreground: Color::WHITE,
          @ { pipe!($read(content).clone()) }
          @Span {
            text: "!",
            font: dejavu_face(),
            font_size: 16.,
            letter_spacing: 0.,
            foreground: Color::GREEN,
          }
        }
      },
      Size::new(200., 40.),
    );

    wnd.draw_frame();
    let initial = last_text_command(
      wnd
        .take_last_frame()
        .expect("expected initial frame"),
    );
    assert_eq!(initial.payload.runs.len(), 2);

    *writer.write() = "Hello".into();
    wnd.draw_frame();

    let updated = last_text_command(
      wnd
        .take_last_frame()
        .expect("expected updated frame"),
    );
    assert_eq!(updated.payload.runs.len(), 2);
    assert_eq!(updated.payload.runs[1].brush, Some(Color::GREEN.into()));
    assert!(updated.payload.bounds.width() > initial.payload.bounds.width());
  }

  #[test]
  fn rich_text_span_pipe_updates_payload() {
    reset_test_env!();
    register_test_font();

    let (content, writer) = split_value::<CowArc<str>>(CowArc::from("Ribir"));
    let mut wnd = TestWindow::new_with_size(
      fn_widget! {
        @RichText {
          text_style: test_text_style(),
          foreground: Color::WHITE,
          @ { "Hello " }
          @Span {
            text: pipe!($read(content).clone()),
            font: dejavu_face(),
            font_size: 16.,
            letter_spacing: 0.,
            foreground: Color::RED,
          }
        }
      },
      Size::new(200., 40.),
    );

    wnd.draw_frame();
    let initial = last_text_command(
      wnd
        .take_last_frame()
        .expect("expected initial frame"),
    );
    assert_eq!(initial.payload.runs.len(), 2);
    assert_eq!(initial.payload.runs[1].brush, Some(Color::RED.into()));

    *writer.write() = "Framework".into();
    wnd.draw_frame();

    let updated = last_text_command(
      wnd
        .take_last_frame()
        .expect("expected updated frame"),
    );
    assert_eq!(updated.payload.runs.len(), 2);
    assert_eq!(updated.payload.runs[1].brush, Some(Color::RED.into()));
    assert!(updated.payload.bounds.width() > initial.payload.bounds.width());
  }

  #[test]
  fn rich_text_span_foreground_pipe_updates_payload() {
    reset_test_env!();
    register_test_font();

    let (color, writer) = split_value::<Brush>(Color::BLUE.into());
    let mut wnd = TestWindow::new_with_size(
      fn_widget! {
        @RichText {
          text_style: test_text_style(),
          foreground: Color::WHITE,
          @Span {
            text: "Ribir",
            font: dejavu_face(),
            font_size: 16.,
            letter_spacing: 0.,
            foreground: pipe!($read(color).clone()),
          }
        }
      },
      Size::new(200., 40.),
    );

    wnd.draw_frame();
    let initial = last_text_command(
      wnd
        .take_last_frame()
        .expect("expected initial frame"),
    );
    assert_eq!(initial.payload.runs.len(), 1);
    assert_eq!(initial.payload.runs[0].brush, Some(Color::BLUE.into()));

    *writer.write() = Color::RED.into();
    wnd.draw_frame();

    let updated = last_text_command(
      wnd
        .take_last_frame()
        .expect("expected updated frame"),
    );
    assert_eq!(updated.payload.runs.len(), 1);
    assert_eq!(updated.payload.runs[0].brush, Some(Color::RED.into()));
  }

  #[test]
  fn rich_text_span_font_size_pipe_updates_payload() {
    reset_test_env!();
    register_test_font();

    let (font_size, writer) = split_value(16.);
    let mut wnd = TestWindow::new_with_size(
      fn_widget! {
        @RichText {
          text_style: test_text_style(),
          foreground: Color::WHITE,
          @Span {
            text: "Wide",
            font: dejavu_face(),
            letter_spacing: 0.,
            font_size: pipe!(*$read(font_size)),
          }
        }
      },
      Size::new(200., 80.),
    );

    wnd.draw_frame();
    let initial = last_text_command(
      wnd
        .take_last_frame()
        .expect("expected initial frame"),
    );

    *writer.write() = 32.;
    wnd.draw_frame();

    let updated = last_text_command(
      wnd
        .take_last_frame()
        .expect("expected updated frame"),
    );
    assert!(updated.payload.bounds.width() > initial.payload.bounds.width());
  }

  #[test]
  fn rich_text_tap_emits_segment_event() {
    reset_test_env!();
    register_test_font();

    let tapped = Stateful::new(None::<RichTextSegmentTapData>);
    let tapped_writer = tapped.clone_writer();
    let mut wnd = TestWindow::new_with_size(
      fn_widget! {
        @RichText {
          text_style: test_text_style(),
          foreground: Color::WHITE,
          on_custom: move |e: &mut RichTextSegmentTapEvent| {
            *$write(tapped_writer) = Some(e.data().clone());
          },
          @ { "Hello " }
          @Span {
            text: "Docs",
            font: dejavu_face(),
            font_size: 16.,
            letter_spacing: 0.,
            foreground: Color::BLUE,
          }
        }
      },
      Size::new(240., 40.),
    );

    wnd.draw_frame();
    let initial = last_text_command(
      wnd
        .take_last_frame()
        .expect("expected initial frame"),
    );
    let tap_pos = tap_pos_for_run(&initial, 1);
    tap_on(&wnd, tap_pos);
    wnd.draw_frame();

    let tapped = tapped.read();
    let tapped = tapped.as_ref().expect("expected segment tap");
    assert_eq!(tapped.index, 1);
    assert_eq!(tapped.range, TextRange::new(6, 10));
    assert_eq!(&*tapped.text, "Docs");
    assert!(tapped.data().is_none());
  }

  #[test]
  fn rich_text_tap_emits_typed_span_data() {
    #[derive(Clone)]
    struct LinkMeta {
      href: CowArc<str>,
    }

    reset_test_env!();
    register_test_font();

    let tapped = Stateful::new(None::<RichTextSegmentTapData>);
    let tapped_writer = tapped.clone_writer();
    let mut wnd = TestWindow::new_with_size(
      fn_widget! {
        @RichText {
          text_style: test_text_style(),
          foreground: Color::WHITE,
          on_custom: move |e: &mut RichTextSegmentTapEvent| {
            *$write(tapped_writer) = Some(e.data().clone());
          },
          @ { "Hello " }
          @Span {
            data: LinkMeta { href: "https://ribir.org".into() },
            text: "Docs",
            font: dejavu_face(),
            font_size: 16.,
            letter_spacing: 0.,
            foreground: Color::BLUE,
          }
        }
      },
      Size::new(240., 40.),
    );

    wnd.draw_frame();
    let initial = last_text_command(
      wnd
        .take_last_frame()
        .expect("expected initial frame"),
    );
    let tap_pos = tap_pos_for_run(&initial, 1);
    tap_on(&wnd, tap_pos);
    wnd.draw_frame();

    let tapped = tapped.read();
    let tapped = tapped.as_ref().expect("expected segment tap");
    assert_eq!(tapped.index, 1);
    assert_eq!(tapped.range, TextRange::new(6, 10));
    assert_eq!(&*tapped.text, "Docs");
    assert!(tapped.data().is_some());
    assert_eq!(
      tapped
        .data_as::<LinkMeta>()
        .map(|link| link.href.as_ref()),
      Some("https://ribir.org"),
    );
  }

  #[test]
  fn rich_text_link_style_updates_from_segment_tap_event() {
    struct LinkMeta {
      href: CowArc<str>,
      visited: bool,
    }

    reset_test_env!();
    register_test_font();

    let link = Stateful::new(LinkMeta { href: "https://ribir.org/docs".into(), visited: false });
    let link_check = link.clone_writer();
    let mut wnd = TestWindow::new_with_size(
      fn_widget! {
        @RichText {
          text_style: test_text_style(),
          foreground: Color::WHITE,
          on_custom: move |e: &mut RichTextSegmentTapEvent| {
            if let Some(link) = e.data().data_as::<Stateful<LinkMeta>>() {
              let href = { link.read().href.clone() };
              assert_eq!(&*href, "https://ribir.org/docs");
              $write(link.clone_writer()).visited = true;
            }
          },
          @Span {
            data: link.clone_writer(),
            text: "Docs",
            font: dejavu_face(),
            font_size: 16.,
            letter_spacing: 0.,
            foreground: pipe! {
              if $read(link).visited {
                Brush::from(Color::RED)
              } else {
                Brush::from(Color::BLUE)
              }
            },
          }
        }
      },
      Size::new(200., 40.),
    );

    wnd.draw_frame();
    let initial = last_text_command(
      wnd
        .take_last_frame()
        .expect("expected initial frame"),
    );
    assert_eq!(initial.payload.runs.len(), 1);
    assert_eq!(initial.payload.runs[0].brush, Some(Color::BLUE.into()));

    let tap_pos = tap_pos_for_run(&initial, 0);
    tap_on(&wnd, tap_pos);
    wnd.draw_frame();

    let updated = last_text_command(
      wnd
        .take_last_frame()
        .expect("expected updated frame"),
    );
    assert_eq!(updated.payload.runs.len(), 1);
    assert_eq!(updated.payload.runs[0].brush, Some(Color::RED.into()));
    assert!(link_check.read().visited);
  }

  #[test]
  fn rich_text_auto_wrap_breaks_long_continuous_text() {
    reset_test_env!();
    register_test_font();

    let mut narrow = TestWindow::new_with_size(
      fn_widget! {
        @RichText {
          size: Size::new(120., 200.),
          text_style: TextStyle {
            font_size: 32.,
            font_face: dejavu_face(),
            letter_space: 0.,
            line_height: LineHeight::Px(32.),
            overflow: TextOverflow::AutoWrap,
          },
          foreground: Color::WHITE,
          @Span { text: "text:" }
          @Span { text: "font32", font_size: 32. }
          @Span { text: "font64", font_size: 64. }
          @Span { text: ",123", font_size: 64. }
        }
      },
      Size::new(120., 200.),
    );

    let mut wide = TestWindow::new_with_size(
      fn_widget! {
        @RichText {
          size: Size::new(320., 200.),
          text_style: TextStyle {
            font_size: 32.,
            font_face: dejavu_face(),
            letter_space: 0.,
            line_height: LineHeight::Px(32.),
            overflow: TextOverflow::AutoWrap,
          },
          foreground: Color::WHITE,
          @Span { text: "text:" }
          @Span { text: "font32", font_size: 32. }
          @Span { text: "font64", font_size: 64. }
          @Span { text: ",123", font_size: 64. }
        }
      },
      Size::new(320., 200.),
    );

    narrow.draw_frame();
    wide.draw_frame();

    let narrow_cmd = last_text_command(
      narrow
        .take_last_frame()
        .expect("expected narrow frame"),
    );
    let wide_cmd = last_text_command(
      wide
        .take_last_frame()
        .expect("expected wide frame"),
    );

    assert!(narrow_cmd.payload.bounds.height() > wide_cmd.payload.bounds.height());
  }

  #[test]
  fn rich_text_inherits_text_decoration() {
    reset_test_env!();
    register_test_font();

    let mut wnd = TestWindow::new_with_size(
      fn_widget! {
        @RichText {
          text_style: test_text_style(),
          foreground: Color::WHITE,
          text_decoration: TextDecoration::UNDERLINE,
          @ { "plain " }
          @Span {
            text: "accent",
            text_decoration: TextDecorationStyle::color(Color::BLUE),
          }
        }
      },
      Size::new(200., 40.),
    );

    wnd.draw_frame();
    let cmd = last_text_command(wnd.take_last_frame().expect("expected a frame"));

    assert_eq!(decoration_kinds(&cmd), vec![TextDecoration::UNDERLINE, TextDecoration::UNDERLINE]);
    assert_eq!(cmd.payload.decorations[0].brush, None);
    assert_eq!(cmd.payload.decorations[1].brush, Some(Color::BLUE.into()));
  }

  #[test]
  fn rich_text_span_text_decoration_overrides_parent() {
    reset_test_env!();
    register_test_font();

    let mut wnd = TestWindow::new_with_size(
      fn_widget! {
        @RichText {
          text_style: test_text_style(),
          foreground: Color::WHITE,
          text_decoration: TextDecoration::UNDERLINE,
          @ { "plain " }
          @Span {
            text: "gone",
            text_decoration: TextDecoration::THROUGHLINE,
          }
        }
      },
      Size::new(200., 40.),
    );

    wnd.draw_frame();
    let cmd = last_text_command(wnd.take_last_frame().expect("expected a frame"));

    assert_eq!(
      decoration_kinds(&cmd),
      vec![TextDecoration::UNDERLINE, TextDecoration::THROUGHLINE]
    );
  }
}
