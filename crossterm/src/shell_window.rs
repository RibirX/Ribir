use crossterm::event::{poll, read};

use crate::{from_window_id::CrosstermWindowId, prelude::WrappedWindowEvent};

use ribir_core::{
  prelude::{AppContext, Theme},
  window::{ShellWindow, Window, WindowId as RibirWindowId},
};

use std::{collections::HashMap, marker::PhantomData, rc::Rc, time::Duration};

#[derive(Default)]
pub struct EventLoop<T: Default = ()> {
  _pd: PhantomData<T>,
}

impl<T: Default> EventLoop<T> {
  fn new() -> Self { EventLoop::default() }
}

// struct ControlFlow {}

pub struct PlatformShellWindow {
  windows: HashMap<CrosstermWindowId, Window>,
  ctx: AppContext,
  event_loop: EventLoop<()>,
}

impl PlatformShellWindow {
  #[inline]
  pub fn new(theme: Theme) -> Self {
    // todo: theme can provide fonts to load.
    let ctx = AppContext {
      app_theme: Rc::new(theme),
      ..Default::default()
    };
    Self { ctx, ..Default::default() }
  }

  pub fn context(&self) -> &AppContext { &self.ctx }

  #[inline]
  pub fn event_loop(&self) -> &EventLoop<()> { &self.event_loop }
}

impl ShellWindow for PlatformShellWindow {
  #[inline]
  fn set_theme(mut self, theme: Theme) { self.ctx.app_theme = Rc::new(theme); }

  #[inline]
  fn context(&self) -> &AppContext { &self.ctx }

  fn exec(self, wnd_id: Box<dyn RibirWindowId>) {
    println!("exec");
    let Self { mut windows /* event_loop, */, .. } = self;

    if let Some(wnd) = windows.get_mut(&CrosstermWindowId::from(wnd_id.clone())) {
      wnd.draw_frame();
    }

    loop {
      // `poll()` waits for an `Event` for a given time period
      match poll(Duration::from_millis(100)) {
        Ok(true) => {
          // It's guaranteed that the `read()` won't block when the `poll()`
          // function returns `true`
          if let Ok(event) = read() {
            let evt = CrosstermWindowId::from(wnd_id.clone());
            if let Some(wnd) = windows.get_mut(&evt) {
              wnd.processes_native_event(WrappedWindowEvent::from(event).into());
              wnd.draw_frame();
            } else {
              println!("process event no window");
            }
          }
        }
        Ok(false) => {
          // if let Some(wnd) =
          // windows.get_mut(&CrosstermWindowId::from(wnd_id.clone())) {
          //   wnd.draw_frame();
          // }
        }
        _ => {}
      }
    }
  }

  fn add_window(&mut self, wnd: Window) -> Box<dyn RibirWindowId> {
    let id = wnd.raw_window.id();
    self
      .windows
      .insert(CrosstermWindowId::from(id.clone()), wnd);

    id
  }
}

impl Default for PlatformShellWindow {
  fn default() -> Self {
    Self {
      windows: Default::default(),
      event_loop: EventLoop::new(),
      ctx: <_>::default(),
    }
  }
}

#[cfg(test)]
mod tests {
  #[test]
  fn test() {
    // let x = WinitApplication::new();
  }
}
