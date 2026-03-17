//! Text primitives and protocol types used by Ribir.
//!
//! The production backend implementation lives in the standalone `ribir_text`
//! crate. This module intentionally keeps only the stable protocol and data
//! model that painter / GPU consume directly.

pub use ribir_algo::Substr;
pub use ribir_text::{
  FontSystem, GlyphRasterSource, RasterBitmap, RasterBitmapFormat,
  font::*,
  paragraph::{
    Caret, CaretAffinity, CaretMotion, ClusterIndex, LineIndex, TextByteIndex, TextHitResult,
    TextRange, VisualPosition,
  },
  style::{GlyphUnit, ParagraphStyle, TextAlign, TextOverflow, TextStyle, TextWrap},
};

use crate::Brush;

pub type AttributedText = ribir_text::AttributedText<Brush>;
pub type AttributedTextBuilder = ribir_text::AttributedTextBuilder<Brush>;
pub type SpanStyle = ribir_text::SpanStyle<Brush>;
pub type TextSpan = ribir_text::TextSpan<Brush>;
pub type DrawGlyph = ribir_text::DrawGlyph;
pub type GlyphId = ribir_text::GlyphId;
pub type DrawGlyphRun = ribir_text::DrawGlyphRun<Brush>;
pub type TextDrawPayload = ribir_text::TextDrawPayload<Brush>;
pub type Paragraph = dyn ribir_text::Paragraph<Brush>;
pub type ParagraphLayout = dyn ribir_text::ParagraphLayout<Brush>;
pub type TextServices = dyn ribir_text::TextServices<Brush>;

pub use ribir_text::{single_style_paragraph_style, single_style_span_style};

pub fn new_text_services() -> Box<TextServices> { ribir_text::new_text_services::<Brush>() }
