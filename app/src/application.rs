use ribir_core::{
  prelude::{AppContext, Theme},
  window::{Window, WindowId},
};

use crate::event_loop::EventLoop;

pub trait Application {
  fn with_theme(self, theme: Theme) -> Self;

  fn context(&self) -> &AppContext;

  fn event_loop(&self) -> &EventLoop<()>;

  fn exec(self, wnd_id: Box<dyn WindowId>);

  fn add_window(&mut self, wnd: Window) -> Box<dyn WindowId>;
}
