use std::any::Any;

use ribir_core::{
  prelude::{CursorIcon, Point, Size},
  window::{RawWindow as RibirRawWindow, WindowId},
};

use crate::prelude::CrosstermWindowId;

pub struct CrosstermWindow {
  id: CrosstermWindowId,
}

impl CrosstermWindow {
  pub fn new(id: CrosstermWindowId) -> Self { Self { id } }

  pub fn request_redraw(&self) {
    println!("request_redraw");
  }

  pub fn id(&self) -> CrosstermWindowId { self.id }
}

impl RibirRawWindow for CrosstermWindow {
  fn inner_size(&self) -> Size { Size::new(100., 100.) }

  fn set_inner_size(&mut self, _size: Size) {
    todo!();
  }

  fn outer_size(&self) -> Size { Size::new(0., 0.) }

  fn inner_position(&self) -> Point { Point::new(0., 0.) }

  fn outer_position(&self) -> Point { Point::new(0., 0.) }

  fn id(&self) -> Box<dyn WindowId> { Box::new(self.id()) }

  fn request_redraw(&self) { self.request_redraw(); }

  fn set_cursor(&mut self, _cursor: CursorIcon) {
    todo!();
  }

  fn scale_factor(&self) -> f64 { 1.0 }

  fn into_any(self: Box<Self>) -> Box<dyn Any> { self }
}

impl From<Box<dyn RibirRawWindow>> for CrosstermWindow {
  fn from(value: Box<dyn RibirRawWindow>) -> Self {
    *value.into_any().downcast::<CrosstermWindow>().unwrap()
  }
}

#[cfg(test)]
mod tests {
  // use std::io::stdout;

  // use crate::prelude::CrosstermWindowId;

  // use super::CrosstermWindow;

  #[test]
  fn boxed_raw_window_into_crossterm_window() {
    // let boxed_raw_window = CrosstermWindow::new(
    //   CrosstermWindowId::zero(),
    //   // Box::new(stdout()), //.into_raw_mode().unwrap(),
    // );

    // let crossterm_window: CrosstermWindow = boxed_raw_window.into();
  }
}
