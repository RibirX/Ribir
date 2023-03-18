use serde::{Deserialize, Serialize};
use std::borrow::Cow;

#[derive(Clone, Copy, Deserialize, Serialize)]
pub enum ColorFormat {
  Rgba8,
}

impl ColorFormat {
  /// return have many bytes per pixel need
  pub const fn pixel_per_bytes(&self) -> u8 {
    match self {
      ColorFormat::Rgba8 => 4,
    }
  }
}

#[derive(Serialize, Deserialize)]
pub struct PixelImage {
  data: Cow<'static, [u8]>,
  size: (u16, u16),
  format: ColorFormat,
}

impl PixelImage {
  #[inline]
  pub fn new(data: Cow<'static, [u8]>, width: u16, height: u16, format: ColorFormat) -> Self {
    PixelImage { data, size: (width, height), format }
  }
  #[inline]
  pub fn color_format(&self) -> ColorFormat { self.format }
  #[inline]
  pub fn size(&self) -> (u16, u16) { self.size }
  #[inline]
  pub fn pixel_bytes(&self) -> &[u8] { &self.data }
}
