use std::{borrow::Cow, hash::Hash, path::Path};

pub use parley::fontique::{FontStyle, FontWeight, FontWidth};
use ribir_algo::CowArc;

use crate::GlyphRasterSourceRef;

pub type FontStretch = FontWidth;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum FontFamily {
  Name(Cow<'static, str>),
  Serif,
  SansSerif,
  Cursive,
  Fantasy,
  Monospace,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FontFace {
  pub families: Box<[FontFamily]>,
  pub stretch: FontStretch,
  pub style: FontStyle,
  pub weight: FontWeight,
}

impl Eq for FontFace {}

impl Hash for FontFace {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.families.hash(state);
    self.stretch.ratio().to_bits().hash(state);

    match self.style {
      FontStyle::Normal => 0u8.hash(state),
      FontStyle::Italic => 1u8.hash(state),
      FontStyle::Oblique(None) => 2u8.hash(state),
      FontStyle::Oblique(Some(angle)) => {
        3u8.hash(state);
        angle.to_bits().hash(state);
      }
    }

    self.weight.value().to_bits().hash(state);
  }
}

impl Default for FontFace {
  fn default() -> Self {
    Self {
      families: Box::new([FontFamily::Serif]),
      stretch: Default::default(),
      style: Default::default(),
      weight: Default::default(),
    }
  }
}

impl<T: Into<Cow<'static, str>>> From<T> for FontFamily {
  #[inline]
  fn from(value: T) -> Self { FontFamily::Name(value.into()) }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct FontRequest {
  pub face: FontFace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FontFaceId {
  pub blob_id: u64,
  pub index: u32,
}

impl FontFaceId {
  pub fn new(blob_id: u64, index: u32) -> Self { Self { blob_id, index } }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontLoadError {
  pub message: CowArc<str>,
}

impl FontLoadError {
  pub fn new(message: impl Into<CowArc<str>>) -> Self { Self { message: message.into() } }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FontFaceMetrics {
  pub units_per_em: u16,
  pub vertical_height: Option<f32>,
  pub ascender: f32,
  pub descender: f32,
  pub line_gap: f32,
  pub x_height: Option<f32>,
  pub cap_height: Option<f32>,
  pub underline_offset: f32,
  pub strikeout_offset: f32,
  pub stroke_size: f32,
}

pub trait FontSystem {
  fn register_font_bytes(&mut self, data: Vec<u8>) -> Result<(), FontLoadError>;

  fn register_font_file(&mut self, path: &Path) -> Result<(), FontLoadError>;

  fn face_metrics(&self, face: FontFaceId) -> Option<FontFaceMetrics>;

  fn raster_source(&self) -> GlyphRasterSourceRef;
}
