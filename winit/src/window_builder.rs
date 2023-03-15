use ribir_core::{prelude::AppContext, widget::Widget, window::Window as RibirWindow};
use ribir_geometry::{DeviceSize, Point, Size};

use crate::{
  application::WinitApplication,
  prelude::{WrappedPhysicalSize, WrappedWindow},
};

pub struct WinitWindowBuilder {
  inner_builder: winit::window::WindowBuilder,
  root: Widget,
}

impl WinitWindowBuilder {
  #[inline]
  pub fn new(root: Widget) -> WindowBuilder {
    WindowBuilder {
      root,
      inner_builder: winit::window::WindowBuilder::default(),
    }
  }
}

impl WindowBuilder for WinitWindowBuilder {
  #[inline]
  fn build(self, app: &WinitApplication) -> RibirWindow {
    let native_wnd = self.inner_builder.build(app.event_loop()).unwrap();
    let size: DeviceSize = WrappedPhysicalSize::<u32>::from(native_wnd.inner_size()).into();
    let ctx = app.context().clone();
    let p_backend = AppContext::wait_future(ribir_gpu::wgpu_backend_with_wnd(
      &native_wnd,
      size,
      None,
      None,
      ctx.shaper.clone(),
    ));
    RibirWindow::new(WrappedWindow::from(native_wnd), p_backend, self.root, ctx)
  }

  /// Requests the window to be of specific dimensions.
  #[inline]
  fn with_inner_size(mut self, size: Size) -> Self {
    let size = winit::dpi::LogicalSize::new(size.width, size.height);
    self.inner_builder = self.inner_builder.with_inner_size(size);
    self
  }

  /// Sets a minimum dimension size for the window.
  #[inline]
  fn with_min_inner_size(mut self, min_size: Size) -> Self {
    let size = winit::dpi::LogicalSize::new(min_size.width, min_size.height);
    self.inner_builder = self.inner_builder.with_min_inner_size(size);
    self
  }

  /// Sets a maximum dimension size for the window.
  #[inline]
  fn with_max_inner_size(mut self, max_size: Size) -> Self {
    let size = winit::dpi::LogicalSize::new(max_size.width, max_size.height);
    self.inner_builder = self.inner_builder.with_max_inner_size(size);
    self
  }

  /// Sets a desired initial position for the window.
  #[inline]
  fn with_position(mut self, position: Point) -> Self {
    let position = winit::dpi::LogicalPosition::new(position.x, position.y);
    self.inner_builder = self.inner_builder.with_position(position);
    self
  }

  /// Sets whether the window is resizable or not.
  #[inline]
  fn with_resizable(mut self, resizable: bool) -> Self {
    self.inner_builder = self.inner_builder.with_resizable(resizable);
    self
  }

  /// Requests a specific title for the window.
  #[inline]
  fn with_title<T: Into<String>>(mut self, title: T) -> Self {
    self.inner_builder = self.inner_builder.with_title(title);
    self
  }

  /// Requests maximized mode.
  #[inline]
  fn with_maximized(mut self, maximized: bool) -> Self {
    self.inner_builder = self.inner_builder.with_maximized(maximized);
    self
  }

  /// Sets whether the window will be initially hidden or visible.
  #[inline]
  fn with_visible(mut self, visible: bool) -> Self {
    self.inner_builder = self.inner_builder.with_visible(visible);
    self
  }

  /// Sets whether the background of the window should be transparent.
  #[inline]
  fn with_transparent(mut self, transparent: bool) -> Self {
    self.inner_builder = self.inner_builder.with_transparent(transparent);
    self
  }

  /// Sets whether the window should have a border, a title bar, etc.
  #[inline]
  fn with_decorations(mut self, decorations: bool) -> Self {
    self.inner_builder = self.inner_builder.with_decorations(decorations);
    self
  }

  // /// Sets the window icon.
  // #[inline]
  // pub fn with_window_icon(mut self, window_icon:Option<winit::window::Icon>)
  // // -> Self {   self.inner_builder = self.inner_builder.
  // with_window_icon(window_icon);   self }
}
