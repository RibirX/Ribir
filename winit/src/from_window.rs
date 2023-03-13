use std::any::Any;

use ribir_core::{
  prelude::{CursorIcon, Point, Size},
  window::{RawWindow as CoreWindow, WindowId},
};
use winit::{dpi::Pixel, window::Window as WinitWindow};

use crate::{
  from_cursor_icon::WrappedCursorIcon,
  from_size::{WinitLogicalSize, WrappedLogicalPosition, WrappedLogicalSize},
  prelude::WrappedWindowId,
};

// #[derive(PartialEq, Eq)]
pub struct WrappedWindow(WinitWindow);

impl CoreWindow for WrappedWindow {
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

  fn as_any(&self) -> &dyn Any { self }
}

impl From<WinitWindow> for WrappedWindow {
  fn from(value: WinitWindow) -> Self { WrappedWindow(value) }
}

impl From<WrappedWindow> for WinitWindow {
  fn from(val: WrappedWindow) -> Self { val.0 }
}

// impl From<Box<dyn CoreWindow>> for WrappedWindow {
//   fn from(value: Box<dyn CoreWindow>) -> Self {
//     let core_window = value
//       .as_ref()
//       .as_any()
//       .downcast_ref::<WrappedWindow>()
//       .map(|v| v)
//       .unwrap();
//     (*core_window).into()
//   }
// }

// impl From<&Box<dyn CoreWindow>> for WrappedWindow {
//   fn from(value: &Box<dyn CoreWindow>) -> Self {
//     let winit_window = value
//       .as_ref()
//       .as_any()
//       .downcast_ref::<WinitWindow>()
//       .map(|v| *(v.to_owned()))
//       .unwrap();
//     winit_window.into()
//   }
// }

// impl From<&dyn CoreWindow> for WrappedWindow {
//   fn from(value: &dyn CoreWindow) -> Self {
//     let winit_window = value
//       .as_any()
//       .downcast_ref::<WinitWindow>()
//       .map(|v| *(v.to_owned()))
//       .unwrap();
//     winit_window.into()
//   }
// }
