use std::ops::Range;

use ribir_core::prelude::*;

use super::caret_state::CaretPosition;

#[derive(Default)]
pub(crate) struct GlyphsHelper {
  pub(crate) glyphs: Option<VisualGlyphs>,
}

impl GlyphsHelper {
  pub(crate) fn caret_position_from_pos(&self, x: f32, y: f32) -> CaretPosition {
    let glyphs: &VisualGlyphs = self.glyphs.as_ref().unwrap();
    let (para, mut offset) = glyphs.nearest_glyph(x, y);
    let rc = glyphs.glyph_rect(para, offset);
    if (rc.min_x() - x).abs() > (rc.max_x() - x).abs() {
      offset += 1;
    }
    let cluster = glyphs.position_to_cluster(para, offset);
    CaretPosition {
      cluster,
      position: Some((para, offset)),
    }
  }

  pub(crate) fn line_end(&self, caret: CaretPosition) -> CaretPosition {
    let glyphs: &VisualGlyphs = self.glyphs.as_ref().unwrap();

    let row = caret_position(glyphs, caret).0;
    let col = glyphs.glyph_count(row, true);
    let cluster = self.cluster_from_glyph_position(row, col);
    CaretPosition { cluster, position: Some((row, col)) }
  }

  pub(crate) fn line_begin(&self, caret: CaretPosition) -> CaretPosition {
    let glyphs: &VisualGlyphs = self.glyphs.as_ref().unwrap();
    let row = caret_position(glyphs, caret).0;
    let cluster: usize = self.cluster_from_glyph_position(row, 0);
    CaretPosition { cluster, position: Some((row, 0)) }
  }

  pub(crate) fn cluster_from_glyph_position(&self, row: usize, col: usize) -> usize {
    let glyphs: &VisualGlyphs = self.glyphs.as_ref().unwrap();
    glyphs.position_to_cluster(row, col)
  }

  pub(crate) fn prev(&self, caret: CaretPosition) -> CaretPosition {
    let glyphs: &VisualGlyphs = self.glyphs.as_ref().unwrap();
    let (mut row, mut col) = caret_position(glyphs, caret);

    (row, col) = match (row > 0, col > 0) {
      (_, true) => (row, col - 1),
      (true, false) => (row - 1, glyphs.glyph_count(row - 1, true)),
      (false, false) => (0, 0),
    };

    let cluster = glyphs.position_to_cluster(row, col);
    CaretPosition { cluster, position: Some((row, col)) }
  }

  pub(crate) fn next(&self, caret: CaretPosition) -> CaretPosition {
    let glyphs: &VisualGlyphs = self.glyphs.as_ref().unwrap();
    let (mut row, mut col) = caret_position(glyphs, caret);
    (row, col) = match (
      row + 1 < glyphs.glyph_row_count(),
      col < glyphs.glyph_count(row, true),
    ) {
      (_, true) => (row, col + 1),
      (true, false) => (row + 1, 0),
      (false, false) => (row, glyphs.glyph_count(row, true)),
    };

    let cluster = glyphs.position_to_cluster(row, col);
    CaretPosition { cluster, position: Some((row, col)) }
  }

  pub(crate) fn up(&self, caret: CaretPosition) -> CaretPosition {
    let glyphs: &VisualGlyphs = self.glyphs.as_ref().unwrap();
    let (mut row, mut col) = caret_position(glyphs, caret);

    (row, col) = match row > 0 {
      true => (row - 1, col.min(glyphs.glyph_count(row - 1, true))),
      false => (row, col),
    };
    let cluster = glyphs.position_to_cluster(row, col);
    CaretPosition { cluster, position: Some((row, col)) }
  }

  pub(crate) fn down(&self, caret: CaretPosition) -> CaretPosition {
    let glyphs: &VisualGlyphs = self.glyphs.as_ref().unwrap();
    let (mut row, mut col) = caret_position(glyphs, caret);
    (row, col) = match row + 1 < glyphs.glyph_row_count() {
      true => (row + 1, col.min(glyphs.glyph_count(row + 1, true))),
      false => (row, col),
    };
    let cluster = glyphs.position_to_cluster(row, col);
    CaretPosition { cluster, position: Some((row, col)) }
  }

  pub(crate) fn cursor(&self, caret: CaretPosition) -> (Point, f32) {
    if let Some(glyphs) = self.glyphs.as_ref() {
      let (row, col) = caret_position(glyphs, caret);

      let line_height = glyphs.line_height(row);
      if col == 0 {
        let glphy = glyphs.glyph_rect(row, col);
        (Point::new(glphy.min_x(), glphy.min_y()), line_height)
      } else {
        let glphy = glyphs.glyph_rect(row, col - 1);
        (Point::new(glphy.max_x(), glphy.min_y()), line_height)
      }
    } else {
      (Point::zero(), 0.)
    }
  }

  pub(crate) fn selection(&self, rg: &Range<usize>) -> Vec<Rect> {
    if rg.is_empty() {
      return vec![];
    }
    self
      .glyphs
      .as_ref()
      .map_or(vec![], |glyphs| glyphs.select_range(rg))
  }
}

fn caret_position(glyphs: &VisualGlyphs, caret: CaretPosition) -> (usize, usize) {
  caret
    .position
    .unwrap_or_else(|| glyphs.position_by_cluster(caret.cluster))
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

  use crate::input::caret_state::CaretPosition;

  use super::GlyphsHelper;

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

    let face = FontFace {
      families: Box::new([FontFamily::Name("DejaVu Sans".into())]),
      ..<_>::default()
    };

    let glyphs = store.typography(
      "1 23 456 7890\n12345".into(),
      FontSize::Em(Em::absolute(1.0)),
      &face,
      cfg,
    );
    let helper = GlyphsHelper { glyphs: Some(glyphs) };
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
