use std::{
  borrow::Cow,
  io::{Error, ErrorKind},
};

use arboard::ImageData;
use ribir_core::prelude::{image::ColorFormat, log::warn, PixelImage};

pub struct Clipboard {
  pub clipboard: arboard::Clipboard,
}

impl Clipboard {
  /// Creates an instance of the clipboard
  pub fn new() -> Result<Self, Error> {
    match arboard::Clipboard::new() {
      Ok(clipboard) => Ok(Clipboard { clipboard }),
      Err(e) => Err(error_convert(e)),
    }
  }
}

impl ribir_core::clipboard::Clipboard for Clipboard {
  fn read_text(&mut self) -> Result<String, Error> {
    self.clipboard.get_text().map_err(error_convert)
  }

  fn write_text(&mut self, text: &str) -> Result<(), Error> {
    self
      .clipboard
      .set_text(text)
      .map_err(error_convert)
  }

  fn read_img(&mut self) -> Result<PixelImage, Error> {
    match self.clipboard.get_image() {
      Ok(img) => {
        Ok(PixelImage::new(img.bytes, img.width as u32, img.height as u32, ColorFormat::Rgba8))
      }
      Err(e) => Err(error_convert(e)),
    }
  }

  fn write_img(&mut self, img: &PixelImage) -> Result<(), Error> {
    self
      .clipboard
      .set_image(ImageData {
        width: img.width() as usize,
        height: img.height() as usize,
        bytes: Cow::Owned(img.pixel_bytes().to_vec()),
      })
      .map_err(error_convert)
  }

  fn read(&mut self, format: &str) -> Result<Cow<[u8]>, Error> {
    warn!("read {format} data from clipboard");
    Err(Error::new(std::io::ErrorKind::Unsupported, "clipboard read format {format}"))
  }

  fn write(&mut self, format: &str, _data: &[u8]) -> Result<(), Error> {
    warn!("write {format} data to clipboard");
    Err(Error::new(std::io::ErrorKind::Unsupported, "clipboard write format {format}"))
  }

  fn clear(&mut self) -> Result<(), Error> { self.clipboard.clear().map_err(error_convert) }
}

fn error_convert(err: arboard::Error) -> Error {
  match err {
    arboard::Error::ContentNotAvailable => Error::new(ErrorKind::Other, "ContentNotAvailable"),
    arboard::Error::ClipboardNotSupported => Error::new(ErrorKind::Other, "ClipboardNotSupported"),
    arboard::Error::ClipboardOccupied => Error::new(ErrorKind::Other, "ClipboardOccupied"),
    arboard::Error::ConversionFailure => Error::new(ErrorKind::Other, "ConversionFailure"),
    arboard::Error::Unknown { description } => Error::new(ErrorKind::Other, description),
    e => Error::new(ErrorKind::Other, e),
  }
}
