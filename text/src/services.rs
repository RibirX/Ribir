use std::{cell::RefCell, path::Path, rc::Rc};

use unicode_segmentation::{GraphemeCursor, UnicodeSegmentation};

use crate::{
  AttributedText, FontSystem, GlyphRasterSourceRef,
  font::{FontFaceId, FontFaceMetrics, FontLoadError},
  paragraph::Paragraph,
  parley_backend::ParleyFontSystem,
  style::Color,
};

pub fn new_text_services<Brush>() -> Box<dyn TextServices<Brush>>
where
  Brush: Clone + From<Color> + PartialEq + 'static,
{
  Box::new(ParleyTextServices::default())
}

pub trait TextServices<Brush> {
  fn register_font_bytes(&self, data: Vec<u8>) -> Result<(), FontLoadError>;

  fn register_font_file(&self, path: &Path) -> Result<(), FontLoadError>;

  fn face_metrics(&self, face: FontFaceId) -> Option<FontFaceMetrics>;

  fn paragraph(&self, source: AttributedText<Brush>) -> Rc<dyn Paragraph<Brush>>;

  fn raster_source(&self) -> GlyphRasterSourceRef;
}

pub trait TextBuffer {
  fn len_bytes(&self) -> crate::TextByteIndex;

  fn slice(&self, range: crate::TextRange) -> &str;

  fn replace(&mut self, range: crate::TextRange, text: &str) -> crate::TextRange;

  fn prev_grapheme_boundary(&self, at: crate::TextByteIndex) -> crate::TextByteIndex;

  fn next_grapheme_boundary(&self, at: crate::TextByteIndex) -> crate::TextByteIndex;

  fn select_token(&self, at: crate::TextByteIndex) -> crate::TextRange;
}

impl TextBuffer for ribir_algo::CowArc<str> {
  fn len_bytes(&self) -> crate::TextByteIndex { crate::TextByteIndex(self.len()) }

  fn slice(&self, range: crate::TextRange) -> &str { &self[range.start.0..range.end.0] }

  fn replace(&mut self, range: crate::TextRange, text: &str) -> crate::TextRange {
    let start = range.start.0.min(self.len());
    let end = range.end.0.min(self.len());

    let mut s = self.to_string();
    s.replace_range(start..end, text);
    *self = s.into();

    crate::TextRange::new(start, start + text.len())
  }

  fn prev_grapheme_boundary(&self, at: crate::TextByteIndex) -> crate::TextByteIndex {
    let at = at.0.min(self.len());
    let mut cursor = GraphemeCursor::new(at, self.len(), true);
    let prev = cursor
      .prev_boundary(self, 0)
      .unwrap()
      .unwrap_or(0);
    crate::TextByteIndex(prev)
  }

  fn next_grapheme_boundary(&self, at: crate::TextByteIndex) -> crate::TextByteIndex {
    let at = at.0.min(self.len());
    let mut cursor = GraphemeCursor::new(at, self.len(), true);
    let next = cursor
      .next_boundary(self, 0)
      .unwrap()
      .unwrap_or(self.len());
    crate::TextByteIndex(next)
  }

  fn select_token(&self, at: crate::TextByteIndex) -> crate::TextRange {
    let at = at.0.min(self.len());
    if at >= self.len() {
      return crate::TextRange::new(self.len(), self.len());
    }

    let mut cursor = GraphemeCursor::new(at, self.len(), true);
    let is_whitespace = self[at..]
      .chars()
      .next()
      .is_some_and(char::is_whitespace);

    loop {
      let boundary = cursor.prev_boundary(self, 0).unwrap();
      if boundary.is_none() || boundary == Some(0) {
        break;
      }

      let pos = cursor.cur_cursor();
      let c = self[pos..].chars().next().unwrap();
      if is_whitespace != c.is_whitespace() || c == '\r' || c == '\n' {
        break;
      }
    }

    let mut base = cursor.cur_cursor();
    for word in self[base..].split_word_bounds() {
      if base + word.len() > at {
        return crate::TextRange::new(base, base + word.len());
      }
      base += word.len();
    }

    crate::TextRange::new(self.len(), self.len())
  }
}

#[derive(Clone)]
struct ParleyTextServices {
  font_system: ribir_algo::Rc<RefCell<ParleyFontSystem>>,
}

impl Default for ParleyTextServices {
  fn default() -> Self {
    Self { font_system: ribir_algo::Rc::new(RefCell::new(ParleyFontSystem::default())) }
  }
}

impl<Brush> TextServices<Brush> for ParleyTextServices
where
  Brush: Clone + From<Color> + PartialEq + 'static,
{
  fn register_font_bytes(&self, data: Vec<u8>) -> Result<(), FontLoadError> {
    self
      .font_system
      .borrow_mut()
      .register_font_bytes(data)
  }

  fn register_font_file(&self, path: &Path) -> Result<(), FontLoadError> {
    self
      .font_system
      .borrow_mut()
      .register_font_file(path)
  }

  fn face_metrics(&self, face: FontFaceId) -> Option<FontFaceMetrics> {
    self.font_system.borrow().face_metrics(face)
  }

  fn paragraph(&self, source: AttributedText<Brush>) -> Rc<dyn Paragraph<Brush>> {
    self.font_system.borrow().paragraph(source)
  }

  fn raster_source(&self) -> GlyphRasterSourceRef { self.font_system.borrow().raster_source() }
}
