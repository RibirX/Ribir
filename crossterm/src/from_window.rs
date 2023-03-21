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

// #[derive(PartialEq, Eq)]
// pub struct WrappedWindow(CrosstermWindow);

impl RibirRawWindow for CrosstermWindow {
  fn inner_size(&self) -> Size {
    // WrappedLogicalSize::<u32>::from(self.0.inner_size().to_logical(self.
    // scale_factor())).into()
    Size::new(100., 100.)
    // let size = ScreenBuffer::current()?.info()?.terminal_size();
  }

  fn set_inner_size(&mut self, _size: Size) {
    // self.0.set_inner_size(
    //   CrosstermLogicalSize::<u32>::new(size.width.cast(), size.height.cast())
    //     .to_physical::<u32>(self.scale_factor()),
    // );
  }

  fn outer_size(&self) -> Size {
    // WrappedLogicalSize::<u32>::from(self.0.outer_size().to_logical(self.
    // scale_factor())).into()
    Size::new(0., 0.)
  }

  fn inner_position(&self) -> Point {
    // WrappedLogicalPosition::<i32>::from(
    //   self
    //     .0
    //     .inner_position()
    //     .expect(" Can only be called on the main thread")
    //     .to_logical(self.scale_factor()),
    // )
    // .into()
    Point::new(0., 0.)
  }

  fn outer_position(&self) -> Point {
    // WrappedLogicalPosition::<i32>::from(
    //   self
    //     .0
    //     .outer_position()
    //     .expect(" Can only be called on the main thread")
    //     .to_logical(self.scale_factor()),
    // )
    // .into()
    Point::new(0., 0.)
  }

  fn id(&self) -> Box<dyn WindowId> { Box::new(self.id()) }

  fn request_redraw(&self) { self.request_redraw(); }

  fn set_cursor(&mut self, _cursor: CursorIcon) {
    // self
    //   .0
    //   .set_cursor_icon(WrappedCursorIcon::from(cursor).into());
  }

  fn scale_factor(&self) -> f64 {
    // self.0.scale_factor()
    1.0
  }

  fn into_any(self: Box<Self>) -> Box<dyn Any> { self }
}

// impl From<CrosstermWindow> for CrosstermWindow {
//   fn from(value: CrosstermWindow) -> Self { WrappedWindow(value) }
// }

// impl From<WrappedWindow> for CrosstermWindow {
//   fn from(val: WrappedWindow) -> Self { val.0 }
// }

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
