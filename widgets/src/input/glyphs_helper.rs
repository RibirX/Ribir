use std::ops::Range;

use ribir_core::prelude::*;

use super::caret_state::CaretPosition;

impl<K, V> SingleKeyMap<K, V>
where
  K: Eq,
{
  fn get(&self, key: &K) -> Option<&V> {
    self
      .0
      .as_ref()
      .filter(|(k, _)| k == key)
      .map(|(_, v)| v)
  }
}

struct SingleKeyMap<K, V>(Option<(K, V)>);

impl<K, V> Default for SingleKeyMap<K, V> {
  fn default() -> Self { Self(None) }
}

#[derive(Default)]
pub(crate) struct TextGlyphsHelper {
  helper: SingleKeyMap<CowArc<str>, VisualGlyphs>,
}

impl TextGlyphsHelper {
  pub(crate) fn new(text: CowArc<str>, glyphs: VisualGlyphs) -> Self {
    Self { helper: SingleKeyMap(Some((text, glyphs))) }
  }

  pub(crate) fn line_end(&self, text: &CowArc<str>, caret: CaretPosition) -> Option<CaretPosition> {
    self.helper.get(text)?.line_end(caret).into()
  }

  pub(crate) fn line_begin(
    &self, text: &CowArc<str>, caret: CaretPosition,
  ) -> Option<CaretPosition> {
    self.helper.get(text)?.line_begin(caret).into()
  }

  pub(crate) fn prev(&self, text: &CowArc<str>, caret: CaretPosition) -> Option<CaretPosition> {
    self.helper.get(text)?.prev(caret).into()
  }

  pub(crate) fn next(&self, text: &CowArc<str>, caret: CaretPosition) -> Option<CaretPosition> {
    self.helper.get(text)?.next(caret).into()
  }

  pub(crate) fn up(&self, text: &CowArc<str>, caret: CaretPosition) -> Option<CaretPosition> {
    self.helper.get(text)?.up(caret).into()
  }

  pub(crate) fn down(&self, text: &CowArc<str>, caret: CaretPosition) -> Option<CaretPosition> {
    self.helper.get(text)?.down(caret).into()
  }

  pub(crate) fn cursor(&self, text: &CowArc<str>, caret: CaretPosition) -> Option<Point> {
    let this = self.helper.get(text)?;
    this.cursor(caret).into()
  }

  pub(crate) fn line_height(&self, text: &CowArc<str>, caret: CaretPosition) -> Option<f32> {
    let this = self.helper.get(text)?;
    this.line_height_by_caret(caret).into()
  }

  pub(crate) fn selection(&self, text: &CowArc<str>, rg: &Range<usize>) -> Option<Vec<Rect>> {
    self.helper.get(text)?.selection(rg).into()
  }
}

pub(crate) trait GlyphsHelper {
  fn caret_position_from_pos(&self, x: f32, y: f32) -> CaretPosition;

  fn line_end(&self, caret: CaretPosition) -> CaretPosition;

  fn line_begin(&self, caret: CaretPosition) -> CaretPosition;

  fn cluster_from_glyph_position(&self, row: usize, col: usize) -> usize;

  fn prev(&self, caret: CaretPosition) -> CaretPosition;

  fn next(&self, caret: CaretPosition) -> CaretPosition;

  fn up(&self, caret: CaretPosition) -> CaretPosition;

  fn down(&self, caret: CaretPosition) -> CaretPosition;

  fn cursor(&self, caret: CaretPosition) -> Point;

  fn line_height_by_caret(&self, caret: CaretPosition) -> f32;

  fn selection(&self, rg: &Range<usize>) -> Vec<Rect>;

  fn caret_position(&self, caret: CaretPosition) -> (usize, usize);
}

impl GlyphsHelper for VisualGlyphs {
  fn caret_position_from_pos(&self, x: f32, y: f32) -> CaretPosition {
    let (para, mut offset) = self.nearest_glyph(x, y);
    let rc = self.glyph_rect(para, offset);
    if (rc.min_x() - x).abs() > (rc.max_x() - x).abs() {
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
      let glphy = self.glyph_rect(row, col);
      Point::new(glphy.min_x(), glphy.min_y())
    } else {
      let glphy = self.glyph_rect(row, col - 1);
      Point::new(glphy.max_x(), glphy.min_y())
    }
  }

  fn line_height_by_caret(&self, caret: CaretPosition) -> f32 {
    let (row, _col) = self.caret_position(caret);
    self.line_height(row)
  }

  fn selection(&self, rg: &Range<usize>) -> Vec<Rect> {
    if rg.is_empty() {
      return vec![];
    }
    self.select_range(rg)
  }

  fn caret_position(&self, caret: CaretPosition) -> (usize, usize) {
    caret
      .position
      .unwrap_or_else(|| self.position_by_cluster(caret.cluster))
  }
}

#[cfg(test)]
mod tests {
  use std::{cell::RefCell, rc::Rc};

  use ribir_core::prelude::{
    font_db::FontDB,
    shaper::TextShaper,
    typography::{PlaceLineDirection, TypographyCfg},
    Em, FontFace, FontFamily, FontSize, Overflow, TextAlign, TypographyStore,
  };
  use ribir_geom::Size;

  use super::GlyphsHelper;
  use crate::input::caret_state::CaretPosition;

  fn test_store() -> TypographyStore {
    let font_db = Rc::new(RefCell::new(FontDB::default()));
    let path = env!("CARGO_MANIFEST_DIR").to_owned() + "/../fonts/DejaVuSans.ttf";
    let _ = font_db.borrow_mut().load_font_file(path);
    let shaper = TextShaper::new(font_db.clone());
    TypographyStore::new(<_>::default(), font_db, shaper)
  }
  #[test]
  fn glyph_move() {
    let store = test_store();

    let cfg = TypographyCfg {
      line_height: None,
      letter_space: None,
      text_align: TextAlign::Start,
      bounds: Size::new(Em::absolute(5.0), Em::absolute(3.0)),
      line_dir: PlaceLineDirection::TopToBottom,
      overflow: Overflow::AutoWrap,
    };

    let face =
      FontFace { families: Box::new([FontFamily::Name("DejaVu Sans".into())]), ..<_>::default() };

    let glyphs =
      store.typography("1 23 456 7890\n12345".into(), FontSize::Em(Em::absolute(1.0)), &face, cfg);
    let helper = glyphs;
    let mut caret = CaretPosition { cluster: 0, position: None };
    caret = helper.prev(caret);
    assert!(caret == CaretPosition { cluster: 0, position: Some((0, 0)) });
    caret = helper.line_end(caret);
    assert!(caret == CaretPosition { cluster: 9, position: Some((0, 9)) });
    caret = helper.next(caret);
    assert!(caret == CaretPosition { cluster: 9, position: Some((1, 0)) });
    caret = helper.prev(caret);
    assert!(caret == CaretPosition { cluster: 9, position: Some((0, 9)) });
    caret = helper.down(caret);
    assert!(caret == CaretPosition { cluster: 13, position: Some((1, 4)) });
    caret = helper.next(caret);
    assert!(caret == CaretPosition { cluster: 14, position: Some((2, 0)) });
    caret = helper.prev(caret);
    assert!(caret == CaretPosition { cluster: 13, position: Some((1, 4)) });
    caret = helper.line_begin(caret);
    assert!(caret == CaretPosition { cluster: 9, position: Some((1, 0)) });
    caret = helper.up(caret);
    assert!(caret == CaretPosition { cluster: 0, position: Some((0, 0)) });
  }
}
