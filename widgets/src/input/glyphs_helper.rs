use std::ops::Range;

use ribir_core::prelude::*;

#[derive(Default)]
pub(crate) struct GlyphsHelper {
  pub(crate) glyphs: Option<VisualGlyphs>,
}

impl GlyphsHelper {
  pub(crate) fn cluster_from_pos(&self, x: f32, y: f32) -> usize {
    let glyphs: &VisualGlyphs = self.glyphs.as_ref().unwrap();
    let (para, offset) = glyphs.nearest_glyph(x, y);
    glyphs.position_to_cluster(para, offset)
  }

  pub(crate) fn glyph_position(&self, cluster: usize) -> (usize, usize) {
    let glyphs: &VisualGlyphs = self.glyphs.as_ref().unwrap();
    glyphs.position_by_cluster(cluster)
  }

  pub(crate) fn cluster_from_glyph_position(&self, row: usize, col: usize) -> usize {
    let glyphs: &VisualGlyphs = self.glyphs.as_ref().unwrap();
    glyphs.position_to_cluster(row, col)
  }

  pub(crate) fn col_count(&self, row: usize) -> usize {
    let glyphs: &VisualGlyphs = self.glyphs.as_ref().unwrap();
    glyphs.glyph_count(row)
  }

  pub(crate) fn prev_cluster(&self, cursor: usize) -> usize {
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

  pub(crate) fn next_cluster(&self, cursor: usize) -> usize {
    let glyphs: &VisualGlyphs = self.glyphs.as_ref().unwrap();
    let (mut row, mut col) = glyphs.position_by_cluster(cursor);

    if col + 1 < glyphs.glyph_count(row) {
      col += 1;
    } else {
      row += 1;
      col = 0;
    }

    glyphs.position_to_cluster(row, col)
  }

  pub(crate) fn up_cluster(&self, cursor: usize) -> usize {
    let glyphs: &VisualGlyphs = self.glyphs.as_ref().unwrap();
    let (mut row, mut col) = glyphs.position_by_cluster(cursor);
    if row == 0 {
      return cursor;
    } else {
      row -= 1;
      col = col.min(glyphs.glyph_count(row) - 1);
    }
    glyphs.position_to_cluster(row, col)
  }

  pub(crate) fn down_cluster(&self, cursor: usize) -> usize {
    let glyphs: &VisualGlyphs = self.glyphs.as_ref().unwrap();
    let (mut row, col) = glyphs.position_by_cluster(cursor);
    if row == glyphs.glyph_row_count() - 1 {
      return cursor;
    }
    row += 1;
    glyphs.position_to_cluster(row, col)
  }

  pub(crate) fn cursor(&self, cursor: usize) -> (Point, f32) {
    if let Some(glyphs) = self.glyphs.as_ref() {
      let (para, offset) = glyphs.position_by_cluster(cursor);
      let glphy = glyphs.glyph_rect(para, offset);
      let line_height = glyphs.line_height(para);
      (Point::new(glphy.min_x(), glphy.min_y()), line_height)
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
