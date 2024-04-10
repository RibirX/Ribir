use std::{borrow::Cow, io::Error};

use log::warn;
use ribir_painter::PixelImage;

pub trait Clipboard {
  // read the string from the clipboard
  fn read_text(&mut self) -> Result<String, Error>;

  // write the string to the clipboard
  fn write_text(&mut self, text: &str) -> Result<(), Error>;

  // read the img_data from the clipboard
  fn read_img(&mut self) -> Result<PixelImage, Error>;

  // write the img_data to the clipboard
  fn write_img(&mut self, img: &PixelImage) -> Result<(), Error>;

  // read the custom format from the clipboard
  fn read(&mut self, format: &str) -> Result<Cow<[u8]>, Error>;

  // write the custom format from the clipboard
  fn write(&mut self, format: &str, data: &[u8]) -> Result<(), Error>;

  // clear all content in the clipboard
  fn clear(&mut self) -> Result<(), Error>;
}

pub(crate) struct MockClipboard {}

impl Clipboard for MockClipboard {
  fn read_text(&mut self) -> Result<String, Error> {
    warn!("read text from clipboard");
    Err(Error::new(std::io::ErrorKind::Unsupported, "clipboard read_text"))
  }

  fn write_text(&mut self, _text: &str) -> Result<(), Error> {
    warn!("write text to clipboard");
    Err(Error::new(std::io::ErrorKind::Unsupported, "clipboard write_text"))
  }

  fn read_img(&mut self) -> Result<PixelImage, Error> {
    warn!("read img from clipboard");
    Err(Error::new(std::io::ErrorKind::Unsupported, "clipboard read_img"))
  }

  fn write_img(&mut self, _img: &PixelImage) -> Result<(), Error> {
    warn!("write img to clipboard");
    Err(Error::new(std::io::ErrorKind::Unsupported, "clipboard write_img"))
  }

  fn read(&mut self, format: &str) -> Result<Cow<[u8]>, Error> {
    warn!("read {format} data from clipboard");
    Err(Error::new(std::io::ErrorKind::Unsupported, "clipboard read format {format}"))
  }

  fn write(&mut self, format: &str, _data: &[u8]) -> Result<(), Error> {
    warn!("write {format} data to clipboard");
    Err(Error::new(std::io::ErrorKind::Unsupported, "clipboard write format {format}"))
  }

  fn clear(&mut self) -> Result<(), Error> {
    warn!("clear content of clipboard");
    Err(Error::new(std::io::ErrorKind::Unsupported, "clipboard clear"))
  }
}
