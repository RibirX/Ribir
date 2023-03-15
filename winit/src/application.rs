use ribir_app::prelude::Application;
use ribir_core::prelude::*;

use std::{collections::HashMap, rc::Rc};
pub use winit::window::WindowId;
use winit::{
  event::{Event, WindowEvent},
  event_loop::{ControlFlow, EventLoop},
  platform::run_return::EventLoopExtRunReturn,
};

use crate::{from_event::WrappedWindowEvent, prelude::WrappedWindowId};
use ribir_core::window::WindowId as RibirWindowId;

pub struct WinitApplication {
  windows: HashMap<WindowId, Window>,
  ctx: AppContext,
  event_loop: EventLoop<()>,
}

impl WinitApplication {
  #[inline]
  fn new(theme: Theme) -> Self {
    // todo: theme can provide fonts to load.
    let ctx = AppContext {
      app_theme: Rc::new(theme),
      ..Default::default()
    };
    Self { ctx, ..Default::default() }
  }
}
impl Application for WinitApplication {
  #[inline]
  fn with_theme(mut self, theme: Theme) -> Self {
    self.ctx.app_theme = Rc::new(theme);
    self
  }

  #[inline]
  fn context(&self) -> &AppContext { &self.ctx }

  fn event_loop(&self) -> &EventLoop<()> { &self.event_loop }

  fn exec(mut self, wnd_id: Box<dyn RibirWindowId>) {
    if let Some(wnd) = self.windows.get_mut(&WrappedWindowId::from(wnd_id).into()) {
      wnd.draw_frame();
    } else {
      panic!("application at least have one window");
    }

    let Self { mut windows, mut event_loop, .. } = self;
    event_loop.run_return(move |event, _event_loop, control: &mut ControlFlow| {
      *control = ControlFlow::Wait;

      match event {
        Event::WindowEvent { event, window_id } => {
          if event == WindowEvent::CloseRequested {
            windows.remove(&window_id);
          } else if event == WindowEvent::Destroyed {
            if windows.is_empty() {
              *control = ControlFlow::Exit;
            }
          } else if let Some(wnd) = windows.get_mut(&window_id) {
            wnd.processes_native_event(WrappedWindowEvent::from(event).into());
          }
        }
        Event::MainEventsCleared => windows.iter_mut().for_each(|(_, wnd)| {
          if wnd.need_draw() {
            wnd.request_redraw();
          }
        }),
        Event::RedrawRequested(id) => {
          if let Some(wnd) = windows.get_mut(&id) {
            wnd.draw_frame();
          }
        }
        Event::RedrawEventsCleared => {
          if windows.iter_mut().any(|(_, wnd)| wnd.need_draw()) {
            *control = ControlFlow::Poll;
          }
        }
        _ => (),
      }
    });
  }

  fn add_window(&mut self, wnd: Window) -> Box<dyn RibirWindowId> {
    let id = wnd.raw_window.id();
    self
      .windows
      .insert(WrappedWindowId::from(id.clone()).into(), wnd);

    id
  }
}

impl Default for WinitApplication {
  fn default() -> Self {
    Self {
      windows: Default::default(),
      event_loop: EventLoop::new(),
      ctx: <_>::default(),
    }
  }
}

// impl Application for WinitApplication {}
