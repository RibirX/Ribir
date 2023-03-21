use ribir_core::{
  prelude::{ShellWindow, Theme},
  widget::Widget,
  window::{WindowConfig, WindowId},
};

use ribir_platform::prelude::{PlatformShellWindow, WindowBuilder};

pub struct Application<T: ShellWindow> {
  shell_window: T,
}

/// Application for platform implementations provided by Ribir
#[cfg(any(feature = "crossterm", feature = "winit"))]
impl Application<PlatformShellWindow> {
  pub fn new(theme: Theme) -> Application<PlatformShellWindow> {
    let shell_window = PlatformShellWindow::new(theme);
    Application { shell_window }
  }

  pub fn window_builder(&self, root: Widget, config: WindowConfig) -> WindowBuilder {
    WindowBuilder::new(root, config)
  }

  pub fn build_window(&mut self, window_builder: WindowBuilder) -> Box<dyn WindowId> {
    let window = window_builder.build(&self.shell_window);
    let window_id = window.raw_window.id().box_clone();
    self.shell_window.add_window(window);
    window_id
  }

  pub fn exec(self, wnd_id: Box<dyn WindowId>) { self.shell_window.exec(wnd_id) }
}

/// Application for platform implementations not provided by Ribir
#[cfg(not(any(feature = "crossterm", feature = "winit")))]
impl<T: ShellWindow> Application<T> {
  pub fn new(shell_window: T, /* , painter_backend: Box<dyn PainterBackend> */) -> Application<T> {
    Application { shell_window }
  }

  pub fn add_window(mut self, window: Window) { self.shell_window.add_window(window); }
}
