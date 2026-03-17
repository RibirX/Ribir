use std::sync::Arc;

use ribir_geom::{Point, Rect, Size};

use crate::{
  paint::TextDrawPayload,
  style::{ParagraphStyle, SpanStyle, TextStyle},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, PartialOrd, Ord)]
pub struct TextByteIndex(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct TextRange {
  pub start: TextByteIndex,
  pub end: TextByteIndex,
}

impl TextRange {
  pub fn new(start: usize, end: usize) -> Self {
    Self { start: TextByteIndex(start), end: TextByteIndex(end) }
  }

  pub fn is_empty(&self) -> bool { self.start == self.end }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, PartialOrd, Ord)]
pub struct ClusterIndex(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, PartialOrd, Ord)]
pub struct LineIndex(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct VisualPosition {
  pub line: LineIndex,
  pub slot: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CaretAffinity {
  Upstream,
  Downstream,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Caret {
  pub byte: TextByteIndex,
  pub affinity: CaretAffinity,
  pub visual: Option<VisualPosition>,
}

impl Default for Caret {
  fn default() -> Self {
    Self { byte: TextByteIndex(0), affinity: CaretAffinity::Downstream, visual: None }
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CaretMotion {
  Prev,
  Next,
  Up,
  Down,
  WordPrev,
  WordNext,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextHitResult {
  pub caret: Caret,
  pub is_inside: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextSpan<Brush> {
  pub range: TextRange,
  pub style: SpanStyle<Brush>,
}

pub trait Paragraph<Brush> {
  fn source_len(&self) -> TextByteIndex;

  fn min_intrinsic_width(&self, text_style: &TextStyle, paragraph_style: &ParagraphStyle) -> f32;

  fn max_intrinsic_width(&self, text_style: &TextStyle, paragraph_style: &ParagraphStyle) -> f32;

  fn layout(
    &self, text_style: &TextStyle, paragraph_style: &ParagraphStyle, max_size: Size,
  ) -> Arc<dyn ParagraphLayout<Brush>>;
}

pub trait ParagraphLayout<Brush> {
  fn size(&self) -> Size;

  fn draw_payload(&self) -> &TextDrawPayload<Brush>;

  fn hit_test_point(&self, point: Point) -> TextHitResult;

  fn caret_rect(&self, caret: Caret) -> Rect;

  fn selection_rects(&self, selection: TextRange) -> Box<[Rect]>;

  fn move_caret(&self, caret: Caret, motion: CaretMotion) -> Caret;

  fn line_start_caret(&self, caret: Caret) -> Caret;

  fn line_end_caret(&self, caret: Caret) -> Caret;
}
