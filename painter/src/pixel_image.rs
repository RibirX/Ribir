use std::borrow::Cow;

use ribir_geom::DeviceSize;
use serde::{Deserialize, Serialize};

#[cfg(any(feature = "jpeg", feature = "png"))]
pub type ImageFormat = image::ImageFormat;

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
  #[inline]
  pub fn color_format(&self) -> ColorFormat { self.format }
  #[inline]
  pub fn width(&self) -> u32 { self.width }
  #[inline]
  pub fn height(&self) -> u32 { self.height }
  pub fn size(&self) -> DeviceSize { DeviceSize::new(self.width as i32, self.height as i32) }
  #[inline]
  pub fn pixel_bytes(&self) -> &[u8] { &self.data }

  // TODO: Move these image format methods out of painter crate
  // These should be in a separate utility crate or in core
  // Blocker: Many usages across examples, tests, and internal code

  #[cfg(feature = "jpeg")]
  pub fn from_jpeg(bytes: &[u8]) -> Self {
    Self::parse_img(bytes, image::ImageFormat::Jpeg).unwrap()
  }

  #[cfg(feature = "jpeg")]
  pub fn write_as_jpeg(
    &self, w: &mut impl std::io::Write,
  ) -> Result<(), Box<dyn std::error::Error>> {
    use image::{DynamicImage, GrayImage, RgbaImage};

    let encoder = ::image::codecs::jpeg::JpegEncoder::new(w);
    match self.format {
      ColorFormat::Rgba8 => DynamicImage::from(
        RgbaImage::from_raw(self.width, self.height, self.data.to_vec()).ok_or("Invalid image")?,
      )
      .to_rgb8()
      .write_with_encoder(encoder)?,
      ColorFormat::Alpha8 => GrayImage::from_raw(self.width, self.height, self.data.to_vec())
        .ok_or("Invalid image")?
        .write_with_encoder(encoder)?,
    };
    Ok(())
  }

  #[cfg(feature = "png")]
  pub fn from_png(bytes: &[u8]) -> Self { Self::parse_img(bytes, image::ImageFormat::Png).unwrap() }

  #[cfg(feature = "png")]
  pub fn write_as_png(
    &self, w: &mut impl std::io::Write,
  ) -> Result<(), Box<dyn std::error::Error>> {
    use image::ImageEncoder;

    let encoder = ::image::codecs::png::PngEncoder::new(w);
    let fmt = match self.format {
      ColorFormat::Rgba8 => ::image::ColorType::Rgba8,
      ColorFormat::Alpha8 => ::image::ColorType::L8,
    };
    encoder.write_image(&self.data, self.width, self.height, fmt.into())?;
    Ok(())
  }

  #[cfg(any(feature = "jpeg", feature = "png"))]
  pub fn parse_img(bytes: &[u8], format: image::ImageFormat) -> image::ImageResult<PixelImage> {
    let img = ::image::load(std::io::Cursor::new(bytes), format)?.to_rgba8();
    let width = img.width();
    let height = img.height();
    Ok(PixelImage::new(img.into_raw().into(), width, height, ColorFormat::Rgba8))
  }
}

impl std::fmt::Debug for PixelImage {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_tuple("PixelImage")
      .field(&format!("{}x{}", self.width, self.height))
      .finish()
  }
}
