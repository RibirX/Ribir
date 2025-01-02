use std::ops::Range;

use ribir_core::prelude::*;

use super::caret_state::CaretPosition;

pub(crate) trait GlyphsHelper {
  fn caret_position_from_pos(&self, pos: Point) -> CaretPosition;

  fn line_end(&self, caret: CaretPosition) -> CaretPosition;

  fn line_begin(&self, caret: CaretPosition) -> CaretPosition;

  fn cluster_from_glyph_position(&self, row: usize, col: usize) -> usize;

  fn prev(&self, caret: CaretPosition) -> CaretPosition;

  fn next(&self, caret: CaretPosition) -> CaretPosition;

  fn up(&self, caret: CaretPosition) -> CaretPosition;

  fn down(&self, caret: CaretPosition) -> CaretPosition;

  fn cursor(&self, caret: CaretPosition) -> Point;

  fn selection(&self, rg: &Range<usize>) -> Vec<Rect>;

  fn caret_position(&self, caret: CaretPosition) -> (usize, usize);
}

impl GlyphsHelper for VisualGlyphs {
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
  use std::cell::RefCell;

  use ribir_core::prelude::{font_db::FontDB, typography::PlaceLineDirection, *};
  use ribir_geom::Size;

  use super::GlyphsHelper;
  use crate::input::caret_state::CaretPosition;

  fn test_store() -> TypographyStore {
    let font_db = Sc::new(RefCell::new(FontDB::default()));
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
    let glyphs = store.typography(
      "1 23 456 7890\n12345".into(),
      &style,
      Size::new(GlyphUnit::PIXELS_PER_EM as f32 * 5.0, GlyphUnit::PIXELS_PER_EM as f32 * 3.0),
      TextAlign::Start,
      font_db::GlyphBaseline::Alphabetic,
      PlaceLineDirection::TopToBottom,
    );

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
