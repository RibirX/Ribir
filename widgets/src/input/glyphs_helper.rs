use std::ops::Range;

use ribir_core::prelude::*;

#[derive(Default)]
pub(crate) struct GlyphsHelper {
  pub(crate) glyphs: Option<VisualGlyphs>,
}

impl GlyphsHelper {
  pub(crate) fn cluster_from_pos(&self, x: f32, y: f32) -> u32 {
    let glyphs: &VisualGlyphs = self.glyphs.as_ref().unwrap();
    let (para, offset) = glyphs.nearest_glyph(x, y);
    glyphs.position_to_cluster(para, offset)
  }

  pub(crate) fn prev_cluster(&self, cursor: usize) -> u32 {
    let glyphs: &VisualGlyphs = self.glyphs.as_ref().unwrap();
    let (mut row, mut col) = glyphs.position_by_cluster(cursor);
    if col > 0 {
      glyphs.position_to_cluster(row, col - 1)
    } else if row > 0 {
      row -= 1;
      col = glyphs.glyph_count(row) - 1;
      glyphs.position_to_cluster(row, col)
    } else {
      0
    }
  }

  pub(crate) fn next_cluster(&self, cursor: usize) -> u32 {
    let glyphs: &VisualGlyphs = self.glyphs.as_ref().unwrap();
    let (mut row, mut col) = glyphs.position_by_cluster(cursor);
    col += 1;
    if col == glyphs.glyph_count(row) && row + 1 < glyphs.glyph_row_count() {
      row += 1;
      col = 0;
    }
    glyphs.position_to_cluster(row, col)
  }

  pub(crate) fn up_cluster(&self, cursor: usize) -> u32 {
    let glyphs: &VisualGlyphs = self.glyphs.as_ref().unwrap();
    let (mut row, mut col) = glyphs.position_by_cluster(cursor);
    if row == 0 {
      return cursor as u32;
    } else {
      row -= 1;
      col = col.min(glyphs.glyph_count(row) - 1);
    }
    glyphs.position_to_cluster(row, col)
  }

  pub(crate) fn down_cluster(&self, cursor: usize) -> u32 {
    let glyphs: &VisualGlyphs = self.glyphs.as_ref().unwrap();
    let (mut row, col) = glyphs.position_by_cluster(cursor);
    if row == glyphs.glyph_row_count() - 1 {
      return cursor as u32;
    }
    row += 1;
    glyphs.position_to_cluster(row, col)
  }

  pub(crate) fn cursor(&self, cursor: usize) -> (Point, f32) {
    let glyphs = self.glyphs.as_ref().unwrap();
    let (para, offset) = glyphs.position_by_cluster(cursor);
    let glphy = glyphs.glyph_rect(para, offset);
    let line_height = glyphs.line_height(para);
    (Point::new(glphy.min_x(), glphy.min_y()), line_height)
  }

  pub(crate) fn selection(&self, rg: &Range<usize>) -> Vec<Rect> {
    if rg.is_empty() {
      return vec![];
    }
    self.glyphs.as_ref().unwrap().select_range(rg)
  }
}
