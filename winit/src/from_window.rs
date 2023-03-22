use std::any::Any;

use ribir_core::{
  prelude::{CursorIcon, Point, Size},
  window::{RawWindow as RibirRawWindow, WindowId},
};
use winit::{dpi::Pixel, window::Window as WinitWindow};

use crate::{
  from_cursor_icon::WrappedCursorIcon,
  from_size::{WinitLogicalSize, WrappedLogicalPosition, WrappedLogicalSize},
  prelude::WrappedWindowId,
};

// #[derive(PartialEq, Eq)]
pub struct WrappedWindow(WinitWindow);

impl RibirRawWindow for WrappedWindow {
  fn inner_size(&self) -> Size {
    WrappedLogicalSize::<u32>::from(self.0.inner_size().to_logical(self.scale_factor())).into()
  }

  fn set_inner_size(&mut self, size: Size) {
    self.0.set_inner_size(
      WinitLogicalSize::<u32>::new(size.width.cast(), size.height.cast())
        .to_physical::<u32>(self.scale_factor()),
    );
  }

  fn outer_size(&self) -> Size {
    WrappedLogicalSize::<u32>::from(self.0.outer_size().to_logical(self.scale_factor())).into()
  }

  fn inner_position(&self) -> Point {
    WrappedLogicalPosition::<i32>::from(
      self
        .0
        .inner_position()
        .expect(" Can only be called on the main thread")
        .to_logical(self.scale_factor()),
    )
    .into()
  }

  fn outer_position(&self) -> Point {
    WrappedLogicalPosition::<i32>::from(
      self
        .0
        .outer_position()
        .expect(" Can only be called on the main thread")
        .to_logical(self.scale_factor()),
    )
    .into()
  }

  fn id(&self) -> Box<dyn WindowId> { Box::new(WrappedWindowId::from(self.0.id())) }

  fn request_redraw(&self) { self.0.request_redraw(); }

  fn set_cursor(&mut self, cursor: CursorIcon) {
    self
      .0
      .set_cursor_icon(WrappedCursorIcon::from(cursor).into());
  }

  fn scale_factor(&self) -> f64 { self.0.scale_factor() }

  fn into_any(self: Box<Self>) -> Box<dyn Any> { self }
}

impl From<WinitWindow> for WrappedWindow {
  fn from(value: WinitWindow) -> Self { WrappedWindow(value) }
}
