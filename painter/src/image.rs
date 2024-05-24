use std::borrow::Cow;

use ribir_geom::DeviceSize;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum ColorFormat {
  Rgba8,
  Alpha8,
}

impl ColorFormat {
  /// return have many bytes per pixel need
  pub const fn pixel_per_bytes(&self) -> u8 {
    match self {
      ColorFormat::Rgba8 => 4,
      ColorFormat::Alpha8 => 1,
    }
  }
}

#[derive(Serialize, Deserialize, PartialEq, Eq)]
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

  #[cfg(feature = "png")]
  pub fn from_png(bytes: &[u8]) -> Self {
    let img = ::image::load(std::io::Cursor::new(bytes), image::ImageFormat::Png)
      .unwrap()
      .to_rgba8();
    let width = img.width();
    let height = img.height();
    PixelImage::new(img.into_raw().into(), width, height, ColorFormat::Rgba8)
  }

  #[cfg(feature = "png")]
  pub fn write_as_png(
    &self, w: &mut impl std::io::Write,
  ) -> Result<(), Box<dyn std::error::Error>> {
    use image::ImageEncoder;
    let png_encoder = ::image::codecs::png::PngEncoder::new(w);
    let fmt = match self.format {
      ColorFormat::Rgba8 => ::image::ColorType::Rgba8,
      ColorFormat::Alpha8 => ::image::ColorType::L8,
    };
    png_encoder.write_image(&self.data, self.width, self.height, fmt)?;
    Ok(())
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
}

impl std::fmt::Debug for PixelImage {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_tuple("PixelImage")
      .field(&format!("{}x{}", self.width, self.height))
      .finish()
  }
}
