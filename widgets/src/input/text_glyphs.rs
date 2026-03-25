use std::{
  cell::{Ref, RefCell},
  ops::Range,
};

use ribir_core::{
  prelude::*,
  text::{single_style_paragraph_style, single_style_span_style},
};

use super::{CaretPosition, edit_text::BaseText};

/// [`TextGlyphs`]
///
/// The TextGlyphs will give Provider of Stateful<TextGlyphs> to the descendants
/// Widgets TextGlyphs include the text data and the glyphs, and can give you
/// more information about glyph's layout or help you paint the text.
#[derive(Clone, Declare, Default)]
pub struct TextGlyphs<T>
where
  T: 'static,
{
  text: T,
  #[declare(skip)]
  glyphs: RefCell<Option<ParagraphLayoutRef>>,
}

impl<T: 'static> TextGlyphs<T> {
  pub fn new(text: T) -> Self { Self { text, glyphs: Default::default() } }

  pub fn text(&self) -> &T { &self.text }

  pub fn text_mut(&mut self) -> &mut T {
    self.glyphs.take();
    &mut self.text
  }

  pub fn glyphs(&self) -> Option<Ref<'_, ParagraphLayoutRef>> {
    Ref::filter_map(self.glyphs.borrow(), |v| v.as_ref()).ok()
  }
}

pub trait VisualText: BaseText {
  /// return self's glyphs layout info.
  fn layout_glyphs(&self, clamp: BoxClamp, ctx: &MeasureCtx) -> ParagraphLayoutRef;

  /// paint the glyphs in the rect.
  fn paint(
    &self, painter: &mut Painter, style: PaintingStyle, glyphs: &ParagraphLayoutRef, rect: Rect,
  );
}

impl VisualText for CowArc<str> {
  fn layout_glyphs(&self, clamp: BoxClamp, ctx: &MeasureCtx) -> ParagraphLayoutRef {
    let style = Provider::of::<TextStyle>(ctx).unwrap();
    let paragraph_style = single_style_paragraph_style(&style, TextAlign::Start);
    let paragraph = AppCtx::text_services()
      .paragraph(AttributedText::styled(self.to_string(), single_style_span_style(&style, None)));
    paragraph.layout(&style, &paragraph_style, clamp)
  }

  fn paint(
    &self, painter: &mut Painter, style: PaintingStyle, glyphs: &ParagraphLayoutRef, rect: Rect,
  ) {
    let _ = (style, rect);
    let brush = painter.fill_brush().clone();
    if !brush.is_visible() {
      return;
    }
    painter.draw_text_payload(
      Resource::new(glyphs.draw_payload().clone()),
      glyphs.draw_payload().bounds,
    );
  }
}

impl<T: VisualText> TextGlyphs<T> {
  pub fn paint(&self, painter: &mut Painter, style: PaintingStyle, rect: Rect) {
    if let Some(glyphs) = self.glyphs() {
      self.text.paint(painter, style, &glyphs, rect);
    }
  }

  pub fn layout_glyphs(&mut self, clamp: BoxClamp, ctx: &MeasureCtx) {
    *self.glyphs.borrow_mut() = Some(self.text.layout_glyphs(clamp, ctx));
  }
}

impl<T: VisualText + 'static> Render for TextGlyphs<T> {
  fn measure(&self, clamp: BoxClamp, ctx: &mut MeasureCtx) -> Size {
    let glyphs = self.text.layout_glyphs(clamp, ctx);
    let size = glyphs.size();
    *self.glyphs.borrow_mut() = Some(glyphs);
    size
  }

  fn paint(&self, ctx: &mut PaintingCtx) {
    let box_rect = Rect::from_size(ctx.box_size().unwrap());
    if ctx
      .painter()
      .intersection_paint_bounds(&box_rect)
      .is_none()
    {
      return;
    };

    let style = Provider::of::<PaintingStyle>(ctx).map(|p| p.clone());
    let layout = self.glyphs().unwrap();
    self
      .text
      .paint(ctx.painter(), style.unwrap_or(PaintingStyle::Fill), &layout, box_rect);
  }
}

pub trait ParagraphLayoutExt {
  fn caret_position_from_pos(&self, pos: Point) -> CaretPosition;

  fn line_end(&self, caret: CaretPosition) -> CaretPosition;

  fn line_begin(&self, caret: CaretPosition) -> CaretPosition;

  fn cluster_from_glyph_position(&self, row: usize, col: usize) -> usize;

  fn prev(&self, caret: CaretPosition) -> CaretPosition;

  fn next(&self, caret: CaretPosition) -> CaretPosition;

  fn up(&self, caret: CaretPosition) -> CaretPosition;

  fn down(&self, caret: CaretPosition) -> CaretPosition;

  fn cursor(&self, caret: CaretPosition) -> Point;

  fn caret_position(&self, caret: CaretPosition) -> (usize, usize);

  fn select_range(&self, rg: &Range<usize>) -> Vec<Rect>;
}

fn to_caret(caret: CaretPosition) -> Caret {
  Caret {
    byte: TextByteIndex(caret.cluster),
    affinity: caret.affinity,
    visual: caret
      .position
      .map(|(line, slot)| VisualPosition { line: LineIndex(line), slot }),
  }
}

fn from_caret(caret: Caret) -> CaretPosition {
  CaretPosition {
    cluster: caret.byte.0,
    affinity: caret.affinity,
    position: caret.visual.map(|p| (p.line.0, p.slot)),
  }
}

impl ParagraphLayoutExt for ParagraphLayout {
  fn caret_position_from_pos(&self, pos: Point) -> CaretPosition {
    from_caret(self.hit_test_point(pos).caret)
  }

  fn line_end(&self, caret: CaretPosition) -> CaretPosition {
    from_caret(self.line_end_caret(to_caret(caret)))
  }

  fn line_begin(&self, caret: CaretPosition) -> CaretPosition {
    from_caret(self.line_start_caret(to_caret(caret)))
  }

  fn cluster_from_glyph_position(&self, row: usize, col: usize) -> usize {
    let mut caret = self.line_start_caret(Caret {
      byte: TextByteIndex(0),
      affinity: CaretAffinity::Downstream,
      visual: Some(VisualPosition { line: LineIndex(row), slot: 0 }),
    });
    for _ in 0..col {
      caret = self.move_caret(caret, CaretMotion::Next);
    }
    caret.byte.0
  }

  fn prev(&self, caret: CaretPosition) -> CaretPosition {
    from_caret(self.move_caret(to_caret(caret), CaretMotion::Prev))
  }

  fn next(&self, caret: CaretPosition) -> CaretPosition {
    from_caret(self.move_caret(to_caret(caret), CaretMotion::Next))
  }

  fn up(&self, caret: CaretPosition) -> CaretPosition {
    from_caret(self.move_caret(to_caret(caret), CaretMotion::Up))
  }

  fn down(&self, caret: CaretPosition) -> CaretPosition {
    from_caret(self.move_caret(to_caret(caret), CaretMotion::Down))
  }

  fn cursor(&self, caret: CaretPosition) -> Point { self.caret_rect(to_caret(caret)).origin }

  fn caret_position(&self, caret: CaretPosition) -> (usize, usize) {
    from_caret(to_caret(caret))
      .position
      .unwrap_or_default()
  }

  fn select_range(&self, rg: &Range<usize>) -> Vec<Rect> {
    self
      .selection_rects(TextRange::new(rg.start, rg.end))
      .into_vec()
  }
}

impl ParagraphLayoutExt for ParagraphLayoutRef {
  fn caret_position_from_pos(&self, pos: Point) -> CaretPosition {
    self.as_ref().caret_position_from_pos(pos)
  }
  fn line_end(&self, caret: CaretPosition) -> CaretPosition { self.as_ref().line_end(caret) }
  fn line_begin(&self, caret: CaretPosition) -> CaretPosition { self.as_ref().line_begin(caret) }
  fn cluster_from_glyph_position(&self, row: usize, col: usize) -> usize {
    self
      .as_ref()
      .cluster_from_glyph_position(row, col)
  }
  fn prev(&self, caret: CaretPosition) -> CaretPosition { self.as_ref().prev(caret) }
  fn next(&self, caret: CaretPosition) -> CaretPosition { self.as_ref().next(caret) }
  fn up(&self, caret: CaretPosition) -> CaretPosition { self.as_ref().up(caret) }
  fn down(&self, caret: CaretPosition) -> CaretPosition { self.as_ref().down(caret) }
  fn cursor(&self, caret: CaretPosition) -> Point { self.as_ref().cursor(caret) }
  fn caret_position(&self, caret: CaretPosition) -> (usize, usize) {
    self.as_ref().caret_position(caret)
  }
  fn select_range(&self, rg: &Range<usize>) -> Vec<Rect> { self.as_ref().select_range(rg) }
}

impl<T> std::ops::Deref for TextGlyphs<T> {
  type Target = T;

  fn deref(&self) -> &Self::Target { &self.text }
}

impl<T> std::ops::DerefMut for TextGlyphs<T> {
  fn deref_mut(&mut self) -> &mut Self::Target { &mut self.text }
}

#[cfg(test)]
mod tests {
  use ribir_core::{prelude::*, text::LineHeight};
  use ribir_types::Size;

  use crate::{input::text_glyphs::ParagraphLayoutExt, prelude::CaretPosition};

  fn assert_caret(caret: CaretPosition, cluster: usize, position: Option<(usize, usize)>) {
    assert_eq!((caret.cluster, caret.position), (cluster, position));
  }

  fn build_glyphs(text: &str, wrap: TextWrap, size: Size) -> ParagraphLayoutRef {
    let text_services = new_text_services();
    let path = env!("CARGO_MANIFEST_DIR").to_owned() + "/../fonts/DejaVuSans.ttf";
    let _ = text_services.register_font_file(std::path::Path::new(&path));

    let paragraph_style = ParagraphStyle { text_align: TextAlign::Start, wrap };
    let paragraph = text_services.paragraph(AttributedText::styled(
      text.to_owned(),
      SpanStyle {
        font: Some(FontRequest {
          face: FontFace {
            families: Box::new([FontFamily::Name("DejaVu Sans".into())]),
            ..<_>::default()
          },
        }),
        font_size: Some(16.),
        letter_spacing: Some(0.),
        line_height: Some(LineHeight::Px(16.)),
        brush: None,
        decoration: None,
      },
    ));
    let text_style = TextStyle {
      font_size: 16.,
      font_face: FontFace {
        families: Box::new([FontFamily::Name("DejaVu Sans".into())]),
        ..<_>::default()
      },
      letter_space: 0.,
      line_height: LineHeight::Px(16.),
      overflow: TextOverflow::AutoWrap,
    };
    paragraph.layout(&text_style, &paragraph_style, BoxClamp::max_size(size))
  }

  fn build_test_glyphs() -> ParagraphLayoutRef {
    build_glyphs(
      "1 23 456 7890\n12345",
      TextWrap::Wrap,
      Size::new(GlyphUnit::PIXELS_PER_EM as f32 * 5.0, GlyphUnit::PIXELS_PER_EM as f32 * 3.0),
    )
  }

  fn build_three_line_glyphs() -> ParagraphLayoutRef {
    build_glyphs(
      "abc\ndef\nghi",
      TextWrap::NoWrap,
      Size::new(GlyphUnit::PIXELS_PER_EM as f32 * 20.0, GlyphUnit::PIXELS_PER_EM as f32 * 4.0),
    )
  }

  fn build_trailing_empty_line_glyphs() -> ParagraphLayoutRef {
    build_glyphs(
      "abc\ndef\n",
      TextWrap::NoWrap,
      Size::new(GlyphUnit::PIXELS_PER_EM as f32 * 20.0, GlyphUnit::PIXELS_PER_EM as f32 * 4.0),
    )
  }

  fn build_increasing_line_glyphs() -> ParagraphLayoutRef {
    build_glyphs(
      "a\nab\nabc",
      TextWrap::NoWrap,
      Size::new(GlyphUnit::PIXELS_PER_EM as f32 * 20.0, GlyphUnit::PIXELS_PER_EM as f32 * 4.0),
    )
  }

  fn build_wrapped_multiline_glyphs() -> ParagraphLayoutRef {
    build_glyphs(
      "1 23 456 7890 12345 67890",
      TextWrap::Wrap,
      Size::new(GlyphUnit::PIXELS_PER_EM as f32 * 5.0, GlyphUnit::PIXELS_PER_EM as f32 * 4.0),
    )
  }

  fn last_line_start(glyphs: &ParagraphLayoutRef) -> CaretPosition {
    let mut caret = CaretPosition::default();
    for _ in 0..16 {
      let next = glyphs.down(caret);
      if next.position == caret.position {
        break;
      }
      caret = next;
    }
    glyphs.line_begin(caret)
  }

  #[test]
  fn glyph_move() {
    let glyphs = build_test_glyphs();

    let mut caret = CaretPosition::default();
    caret = glyphs.prev(caret);
    assert_caret(caret, 0, Some((0, 0)));
    caret = glyphs.line_end(caret);
    assert_caret(caret, 9, Some((0, 9)));
    caret = glyphs.next(caret);
    assert_caret(caret, 9, Some((1, 0)));
    caret = glyphs.prev(caret);
    assert_caret(caret, 9, Some((0, 9)));
    caret = glyphs.down(caret);
    assert_caret(caret, 13, Some((1, 4)));
    caret = glyphs.next(caret);
    assert_caret(caret, 14, Some((2, 0)));
    caret = glyphs.prev(caret);
    assert_caret(caret, 13, Some((1, 4)));
    caret = glyphs.line_begin(caret);
    assert_caret(caret, 9, Some((1, 0)));
    caret = glyphs.up(caret);
    assert_caret(caret, 0, Some((0, 0)));
  }

  #[test]
  fn glyph_move_keeps_direction_without_visual_position() {
    let glyphs = build_test_glyphs();

    let line_end = glyphs.line_end(CaretPosition::default());
    let wrapped_start = glyphs.next(line_end);
    let wrapped_next = glyphs.next(wrapped_start);
    let wrapped_prev = glyphs.prev(wrapped_start);
    let wrapped_up = glyphs.up(wrapped_start);
    let wrapped_down = glyphs.down(wrapped_start);

    assert_eq!(glyphs.next(CaretPosition { position: None, ..line_end }), wrapped_start);
    assert_eq!(glyphs.next(CaretPosition { position: None, ..wrapped_start }), wrapped_next);
    assert_eq!(glyphs.prev(CaretPosition { position: None, ..wrapped_start }), wrapped_prev);
    assert_eq!(glyphs.up(CaretPosition { position: None, ..wrapped_start }), wrapped_up);
    assert_eq!(glyphs.down(CaretPosition { position: None, ..wrapped_start }), wrapped_down);
  }

  #[test]
  fn glyph_cursor_uses_visual_position_for_wrapped_boundary() {
    let glyphs = build_test_glyphs();

    let line_end = glyphs.line_end(CaretPosition::default());
    let wrapped_start = glyphs.next(line_end);

    assert_ne!(line_end.position, wrapped_start.position);
    assert_ne!(glyphs.cursor(line_end), glyphs.cursor(wrapped_start));
  }

  #[test]
  fn glyph_hit_test_round_trips_wrapped_last_line_boundary() {
    let glyphs = build_wrapped_multiline_glyphs();
    let last_line_start = last_line_start(&glyphs);
    let prev = glyphs.prev(last_line_start);

    assert_eq!(glyphs.caret_position_from_pos(glyphs.cursor(last_line_start)), last_line_start);
    assert_eq!(glyphs.caret_position_from_pos(glyphs.cursor(prev)), prev);
  }

  #[test]
  fn glyph_cursor_stays_on_same_visual_line_around_wrapped_boundaries() {
    let glyphs = build_wrapped_multiline_glyphs();
    let last_line_start = last_line_start(&glyphs);
    let prev = glyphs.prev(last_line_start);
    let prev_prev = glyphs.prev(prev);
    let up = glyphs.up(last_line_start);
    let up_next = glyphs.next(up);

    assert_eq!(prev.position.unwrap().0, prev_prev.position.unwrap().0);
    assert_eq!(up.position.unwrap().0, up_next.position.unwrap().0);
    assert_eq!(glyphs.cursor(prev).y, glyphs.cursor(prev_prev).y);
    assert_eq!(glyphs.cursor(up).y, glyphs.cursor(up_next).y);
  }

  #[test]
  fn glyph_move_three_lines_keeps_expected_line_when_visual_position_is_missing() {
    let glyphs = build_three_line_glyphs();

    let mut last_line_start = glyphs.down(CaretPosition::default());
    last_line_start = glyphs.down(last_line_start);
    last_line_start = glyphs.line_begin(last_line_start);

    let expected_prev = glyphs.prev(last_line_start);
    let expected_prev_prev = glyphs.prev(expected_prev);
    let expected_up = glyphs.up(last_line_start);

    assert_eq!(glyphs.prev(CaretPosition { position: None, ..last_line_start }), expected_prev);
    assert_eq!(glyphs.prev(CaretPosition { position: None, ..expected_prev }), expected_prev_prev);
    assert_eq!(glyphs.up(CaretPosition { position: None, ..last_line_start }), expected_up);
  }

  #[test]
  fn glyph_move_wrapped_last_line_keeps_expected_line_when_visual_position_is_missing() {
    let glyphs = build_wrapped_multiline_glyphs();
    let last_line_start = last_line_start(&glyphs);

    assert!(last_line_start.position.unwrap().0 >= 2);

    let expected_prev = glyphs.prev(last_line_start);
    let expected_prev_prev = glyphs.prev(expected_prev);
    let expected_up = glyphs.up(last_line_start);

    assert_eq!(glyphs.prev(CaretPosition { position: None, ..last_line_start }), expected_prev);
    assert_eq!(glyphs.prev(CaretPosition { position: None, ..expected_prev }), expected_prev_prev);
    assert_eq!(glyphs.up(CaretPosition { position: None, ..last_line_start }), expected_up);
  }

  #[test]
  fn glyph_move_trailing_empty_line_keeps_expected_line() {
    let glyphs = build_trailing_empty_line_glyphs();

    let mut last_line_start = glyphs.down(CaretPosition::default());
    last_line_start = glyphs.down(last_line_start);
    last_line_start = glyphs.line_begin(last_line_start);

    assert_eq!(last_line_start.position, Some((2, 0)));

    let prev = glyphs.prev(last_line_start);
    let prev_prev = glyphs.prev(prev);
    let up = glyphs.up(last_line_start);

    assert_eq!(prev.position.unwrap().0, 1);
    assert_eq!(prev_prev.position.unwrap().0, 1);
    assert_eq!(up.position.unwrap().0, 1);
    assert_eq!(glyphs.cursor(prev).y, glyphs.cursor(prev_prev).y);
    assert_eq!(glyphs.cursor(up).y, glyphs.cursor(prev).y);
  }

  #[test]
  fn glyph_move_across_increasing_hard_lines_does_not_bounce() {
    let glyphs = build_increasing_line_glyphs();

    let second_line_start = glyphs.line_begin(glyphs.down(CaretPosition::default()));
    let third_line_start = glyphs.line_begin(glyphs.down(second_line_start));
    let second_line_end = glyphs.prev(third_line_start);
    let second_line_interior = glyphs.prev(second_line_end);
    let first_line_end = glyphs.prev(second_line_start);
    let second_line_next = glyphs.next(second_line_start);
    let second_line_next_next = glyphs.next(second_line_next);

    assert_eq!(third_line_start.position, Some((2, 0)));
    assert_eq!(second_line_end.position.unwrap().0, 1);
    assert_eq!(second_line_interior.position.unwrap().0, 1);
    assert_eq!(first_line_end.position.unwrap().0, 0);
    assert_eq!(second_line_next.position.unwrap().0, 1);
    assert_eq!(second_line_next_next.position.unwrap().0, 1);
    let second_line_end_cursor = glyphs.cursor(second_line_end);
    let second_line_interior_cursor = glyphs.cursor(second_line_interior);
    assert_ne!(second_line_end_cursor, second_line_interior_cursor);
    assert!(
      second_line_interior_cursor.x < second_line_end_cursor.x,
      "expected interior x < end x, got interior={second_line_interior_cursor:?}, \
       end={second_line_end_cursor:?}, start={:?}, next={:?}, next_next={:?}",
      glyphs.cursor(second_line_start),
      glyphs.cursor(second_line_next),
      glyphs.cursor(second_line_next_next),
    );
    assert_eq!(glyphs.cursor(first_line_end).y, glyphs.cursor(CaretPosition::default()).y);
  }
}
