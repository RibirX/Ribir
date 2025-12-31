use std::cell::{Ref, RefCell};

use ribir_core::prelude::*;

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
  glyphs: RefCell<Option<VisualGlyphs>>,
}

impl<T: 'static> TextGlyphs<T> {
  pub fn new(text: T) -> Self { Self { text, glyphs: Default::default() } }

  pub fn text(&self) -> &T { &self.text }

  pub fn text_mut(&mut self) -> &mut T {
    self.glyphs.take();
    &mut self.text
  }

  pub fn glyphs(&self) -> Option<Ref<'_, VisualGlyphs>> {
    Ref::filter_map(self.glyphs.borrow(), |v| v.as_ref()).ok()
  }
}

pub trait VisualText: BaseText {
  /// return self's glyphs layout info.
  fn layout_glyphs(&self, clamp: BoxClamp, ctx: &LayoutCtx) -> VisualGlyphs;

  /// paint the glyphs in the rect.
  fn paint(&self, painter: &mut Painter, style: PaintingStyle, glyphs: &VisualGlyphs, rect: Rect);
}

impl VisualText for CowArc<str> {
  fn layout_glyphs(&self, clamp: BoxClamp, ctx: &LayoutCtx) -> VisualGlyphs {
    let style = Provider::of::<TextStyle>(ctx).unwrap();
    text_glyph(self.substr(..), &style, TextAlign::Start, clamp.max)
  }

  fn paint(&self, painter: &mut Painter, style: PaintingStyle, glyphs: &VisualGlyphs, rect: Rect) {
    paint_text(painter, glyphs, style, rect);
  }
}

impl<T: VisualText> TextGlyphs<T> {
  pub fn paint(&self, painter: &mut Painter, style: PaintingStyle, rect: Rect) {
    if let Some(glyphs) = self.glyphs() {
      self.text.paint(painter, style, &glyphs, rect);
    }
  }

  pub fn layout_glyphs(&mut self, clamp: BoxClamp, ctx: &LayoutCtx) {
    *self.glyphs.borrow_mut() = Some(self.text.layout_glyphs(clamp, ctx));
  }
}

impl<T: VisualText + 'static> Render for TextGlyphs<T> {
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    let glyphs = self.text.layout_glyphs(clamp, ctx);
    let size = glyphs.visual_rect().size;
    *self.glyphs.borrow_mut() = Some(glyphs);
    size
  }

  fn visual_box(&self, _: &mut VisualCtx) -> Option<Rect> {
    let visual_glyphs = self.glyphs()?;
    Some(visual_glyphs.visual_rect())
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
    let visual_glyphs = self.glyphs().unwrap();
    paint_text(ctx.painter(), &visual_glyphs, style.unwrap_or(PaintingStyle::Fill), box_rect);
  }
}

pub trait VisualGlyphsHelper {
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
}

impl VisualGlyphsHelper for VisualGlyphs {
  fn caret_position_from_pos(&self, pos: Point) -> CaretPosition {
    let (para, mut offset) = self.nearest_glyph(pos.x, pos.y);
    let rc = self.glyph_rect(para, offset);
    if (rc.min_x() - pos.x).abs() > (rc.max_x() - pos.x).abs() {
      offset += 1;
    }
    let cluster = self.position_to_cluster(para, offset);
    CaretPosition { cluster, position: Some((para, offset)) }
  }

  fn line_end(&self, caret: CaretPosition) -> CaretPosition {
    let row = self.caret_position(caret).0;
    let col = self.glyph_count(row, true);
    let cluster = self.cluster_from_glyph_position(row, col);
    CaretPosition { cluster, position: Some((row, col)) }
  }

  fn line_begin(&self, caret: CaretPosition) -> CaretPosition {
    let row = self.caret_position(caret).0;
    let cluster: usize = self.cluster_from_glyph_position(row, 0);
    CaretPosition { cluster, position: Some((row, 0)) }
  }

  fn cluster_from_glyph_position(&self, row: usize, col: usize) -> usize {
    self.position_to_cluster(row, col)
  }

  fn prev(&self, caret: CaretPosition) -> CaretPosition {
    let (mut row, mut col) = self.caret_position(caret);

    (row, col) = match (row > 0, col > 0) {
      (_, true) => (row, col - 1),
      (true, false) => (row - 1, self.glyph_count(row - 1, true)),
      (false, false) => (0, 0),
    };

    let cluster = self.position_to_cluster(row, col);
    CaretPosition { cluster, position: Some((row, col)) }
  }

  fn next(&self, caret: CaretPosition) -> CaretPosition {
    let (mut row, mut col) = self.caret_position(caret);

    (row, col) = match (row + 1 < self.glyph_row_count(), col < self.glyph_count(row, true)) {
      (_, true) => (row, col + 1),
      (true, false) => (row + 1, 0),
      (false, false) => (row, self.glyph_count(row, true)),
    };

    let cluster = self.position_to_cluster(row, col);
    CaretPosition { cluster, position: Some((row, col)) }
  }

  fn up(&self, caret: CaretPosition) -> CaretPosition {
    let (mut row, mut col) = self.caret_position(caret);

    (row, col) = match row > 0 {
      true => (row - 1, col.min(self.glyph_count(row - 1, true))),
      false => (row, col),
    };
    let cluster = self.position_to_cluster(row, col);
    CaretPosition { cluster, position: Some((row, col)) }
  }

  fn down(&self, caret: CaretPosition) -> CaretPosition {
    let (mut row, mut col) = self.caret_position(caret);

    (row, col) = match row + 1 < self.glyph_row_count() {
      true => (row + 1, col.min(self.glyph_count(row + 1, true))),
      false => (row, col),
    };
    let cluster = self.position_to_cluster(row, col);
    CaretPosition { cluster, position: Some((row, col)) }
  }

  fn cursor(&self, caret: CaretPosition) -> Point {
    let (row, col) = self.caret_position(caret);

    if col == 0 {
      let glyph = self.glyph_rect(row, col);
      Point::new(glyph.min_x(), glyph.min_y())
    } else {
      let glyph = self.glyph_rect(row, col - 1);
      Point::new(glyph.max_x(), glyph.min_y())
    }
  }

  fn caret_position(&self, caret: CaretPosition) -> (usize, usize) {
    caret
      .position
      .unwrap_or_else(|| self.position_by_cluster(caret.cluster))
  }
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
  use std::cell::RefCell;

  use ribir_core::prelude::{font_db::FontDB, typography::PlaceLineDirection, *};
  use ribir_geom::Size;

  use crate::{input::text_glyphs::VisualGlyphsHelper, prelude::CaretPosition};

  fn test_store() -> TypographyStore {
    let font_db = Rc::new(RefCell::new(FontDB::default()));
    let path = env!("CARGO_MANIFEST_DIR").to_owned() + "/../fonts/DejaVuSans.ttf";
    let _ = font_db.borrow_mut().load_font_file(path);
    TypographyStore::new(font_db)
  }
  #[test]
  fn glyph_move() {
    let mut store = test_store();

    let style = TextStyle {
      font_size: 16.,
      font_face: FontFace {
        families: Box::new([FontFamily::Name("DejaVu Sans".into())]),
        ..<_>::default()
      },
      letter_space: 0.,
      line_height: 16.,
      overflow: TextOverflow::AutoWrap,
    };
    let text: CowArc<str> = "1 23 456 7890\n12345".into();
    let glyphs = store.typography(
      text.substr(..),
      &style,
      Size::new(GlyphUnit::PIXELS_PER_EM as f32 * 5.0, GlyphUnit::PIXELS_PER_EM as f32 * 3.0),
      TextAlign::Start,
      font_db::GlyphBaseline::Alphabetic,
      PlaceLineDirection::TopToBottom,
    );

    let mut caret = CaretPosition { cluster: 0, position: None };
    caret = glyphs.prev(caret);
    assert!(caret == CaretPosition { cluster: 0, position: Some((0, 0)) });
    caret = glyphs.line_end(caret);
    assert!(caret == CaretPosition { cluster: 9, position: Some((0, 9)) });
    caret = glyphs.next(caret);
    assert!(caret == CaretPosition { cluster: 9, position: Some((1, 0)) });
    caret = glyphs.prev(caret);
    assert!(caret == CaretPosition { cluster: 9, position: Some((0, 9)) });
    caret = glyphs.down(caret);
    assert!(caret == CaretPosition { cluster: 13, position: Some((1, 4)) });
    caret = glyphs.next(caret);
    assert!(caret == CaretPosition { cluster: 14, position: Some((2, 0)) });
    caret = glyphs.prev(caret);
    assert!(caret == CaretPosition { cluster: 13, position: Some((1, 4)) });
    caret = glyphs.line_begin(caret);
    assert!(caret == CaretPosition { cluster: 9, position: Some((1, 0)) });
    caret = glyphs.up(caret);
    assert!(caret == CaretPosition { cluster: 0, position: Some((0, 0)) });
  }
}
