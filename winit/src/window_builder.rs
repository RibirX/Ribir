use ribir_core::{
  prelude::AppContext,
  widget::Widget,
  window::WindowConfig,
  window::{ShellWindow, Window as RibirWindow},
};
use ribir_geometry::DeviceSize;

use crate::{
  prelude::{WrappedLogicalPosition, WrappedLogicalSize, WrappedPhysicalSize, WrappedWindow},
  shell_window::PlatformShellWindow,
};

pub struct WindowBuilder {
  inner_builder: winit::window::WindowBuilder,
  root: Widget,
}

impl WindowBuilder {
  #[inline]
  pub fn new(root_widget: Widget, config: WindowConfig) -> WindowBuilder {
    let mut inner_builder = winit::window::WindowBuilder::default();

    if let Some(size) = config.inner_size {
      let size: winit::dpi::LogicalSize<f32> = WrappedLogicalSize::from(size).into();
      inner_builder = inner_builder.with_inner_size(size);
    }

    if let Some(size) = config.min_inner_size {
      let size: winit::dpi::LogicalSize<f32> = WrappedLogicalSize::from(size).into();
      inner_builder = inner_builder.with_min_inner_size(size);
    }

    if let Some(size) = config.max_inner_size {
      let size: winit::dpi::LogicalSize<f32> = WrappedLogicalSize::from(size).into();
      inner_builder = inner_builder.with_max_inner_size(size);
    }

    if let Some(position) = config.position {
      let position: winit::dpi::LogicalPosition<f32> =
        WrappedLogicalPosition::from(position).into();
      inner_builder = inner_builder.with_position(position);
    }

    if let Some(resizable) = config.resizable {
      inner_builder = inner_builder.with_resizable(resizable);
    }

    if let Some(title) = config.title {
      inner_builder = inner_builder.with_title(title);
    }

    if let Some(maximized) = config.resizable {
      inner_builder = inner_builder.with_maximized(maximized);
    }

    if let Some(visible) = config.resizable {
      inner_builder = inner_builder.with_visible(visible);
    }

    if let Some(transparent) = config.transparent {
      inner_builder = inner_builder.with_visible(transparent);
    }

    if let Some(decorations) = config.decorations {
      inner_builder = inner_builder.with_visible(decorations);
    }

    // if let Some(window_icon) = config.window_icon {
    //   inner_builder = inner_builder.with_window_icon(window_icon);
    // }

    WindowBuilder { root: root_widget, inner_builder }
  }

  pub fn build(self, shell_window: &PlatformShellWindow) -> RibirWindow {
    let native_wnd = self.inner_builder.build(shell_window.event_loop()).unwrap();
    let size: DeviceSize = WrappedPhysicalSize::<u32>::from(native_wnd.inner_size()).into();
    let ctx = shell_window.context().clone();
    let p_backend = AppContext::wait_future(ribir_gpu::wgpu_backend_with_wnd(
      &native_wnd,
      size,
      None,
      None,
      ctx.shaper.clone(),
    ));
    RibirWindow::new(WrappedWindow::from(native_wnd), p_backend, self.root, ctx)
  }

  #[cfg(feature = "wgpu_gl")]
  #[inline]
  pub fn build_headless(self, shell_window: &PlatformShellWindow, size: DeviceSize) -> RibirWindow {
    let native_wnd = self.inner_builder.build(shell_window.event_loop()).unwrap();
    let ctx = shell_window.context().clone();
    let p_backend = AppContext::wait_future(ribir_gpu::wgpu_backend_with_wnd(
      &native_wnd,
      size,
      None,
      None,
      ctx.shaper.clone(),
    ));
    RibirWindow::new(WrappedWindow::from(native_wnd), p_backend, self.root, ctx)
  }
}
