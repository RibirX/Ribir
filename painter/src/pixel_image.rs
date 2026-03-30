use std::borrow::Cow;

use image_webp::{ColorType as WebPColorType, WebPDecoder, WebPEncoder};
use ribir_types::DeviceSize;
use serde::{Deserialize, Serialize};

type BoxError = Box<dyn std::error::Error>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum ColorFormat {
  Rgba8,
  Alpha8,
}

impl ColorFormat {
  /// return have many bytes per pixel need
  pub const fn bytes_per_pixel(&self) -> u8 {
    match self {
      ColorFormat::Rgba8 => 4,
      ColorFormat::Alpha8 => 1,
    }
  }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct PixelImage {
  data: Cow<'static, [u8]>,
  width: u32,
  height: u32,
  format: ColorFormat,
}

impl PixelImage {
  #[inline]
  pub fn new(data: Cow<'static, [u8]>, width: u32, height: u32, format: ColorFormat) -> Self {
    PixelImage { data, width, height, format }
  }
  #[inline]
  pub fn color_format(&self) -> ColorFormat { self.format }
  #[inline]
  pub fn width(&self) -> u32 { self.width }
  #[inline]
  pub fn height(&self) -> u32 { self.height }
  pub fn size(&self) -> DeviceSize { DeviceSize::new(self.width as i32, self.height as i32) }
  #[inline]
  pub fn pixel_bytes(&self) -> &[u8] { &self.data }

  fn expand_rgb(bytes: Vec<u8>) -> Vec<u8> {
    bytes
      .chunks_exact(3)
      .flat_map(|rgb| [rgb[0], rgb[1], rgb[2], 255])
      .collect()
  }

  fn rgba_bytes(&self) -> Cow<'_, [u8]> {
    match self.format {
      ColorFormat::Rgba8 => self.data.clone(),
      ColorFormat::Alpha8 => Cow::Owned(
        self
          .data
          .iter()
          .flat_map(|&alpha| [255u8, 255, 255, alpha])
          .collect(),
      ),
    }
  }

  /// Decode WebP data to PixelImage (first frame for animated WebP).
  pub fn from_webp(bytes: &[u8]) -> Result<Self, BoxError> {
    let mut decoder = WebPDecoder::new(std::io::Cursor::new(bytes))?;
    let (width, height) = decoder.dimensions();
    let mut data = vec![
      0;
      decoder
        .output_buffer_size()
        .ok_or_else(|| std::io::Error::other("WebP image too large"))?
    ];
    decoder.read_image(&mut data)?;

    let data = if decoder.has_alpha() { data } else { Self::expand_rgb(data) };

    Ok(Self::new(data.into(), width, height, ColorFormat::Rgba8))
  }

  /// Encode PixelImage to WebP (single frame). Only supports RGBA8.
  pub fn write_as_webp(&self, w: &mut impl std::io::Write) -> Result<(), BoxError> {
    let rgba = self.rgba_bytes();
    WebPEncoder::new(w).encode(&rgba, self.width, self.height, WebPColorType::Rgba8)?;
    Ok(())
  }
}

impl std::fmt::Debug for PixelImage {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_tuple("PixelImage")
      .field(&format!("{}x{}", self.width, self.height))
      .finish()
  }
}
