use crate::prelude::*;
use std::{collections::HashMap, rc::Rc};
pub use winit::window::WindowId;
use winit::{
  event::{Event, WindowEvent},
  event_loop::{ControlFlow, EventLoop},
  platform::run_return::EventLoopExtRunReturn,
};

pub struct Application {
  windows: HashMap<WindowId, Window>,
  ctx: AppContext,
  event_loop: EventLoop<()>,
}

impl Application {
  #[inline]
  pub fn new() -> Application { <_>::default() }

  // todo: theme can provide fonts to load.
  #[inline]
  pub fn with_theme(mut self, theme: Theme) -> Application {
    self.ctx.app_theme = Rc::new(theme);
    self
  }

  #[inline]
  pub fn context(&self) -> &AppContext { &self.ctx }

  pub fn exec(mut self, w: Widget) {
    let wnd_id = self.new_window(w);
    if let Some(wnd) = self.windows.get_mut(&wnd_id) {
      wnd.draw_frame();
    }

    let Self { mut windows, mut event_loop, .. } = self;
    event_loop.run_return(
      move |event, _event_loop, control: &mut ControlFlow| match event {
        Event::WindowEvent { event, window_id } => {
          if event == WindowEvent::CloseRequested {
            windows.remove(&window_id);
          } else if event == WindowEvent::Destroyed {
            if windows.len() == 0 {
              *control = ControlFlow::Exit;
            }
          } else if let Some(wnd) = windows.get_mut(&window_id) {
            wnd.processes_native_event(event);
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
        _ => (),
      },
    );
  }

  pub fn run(w: Widget) {
    let app = Application::default();
    app.exec(w)
  }

  pub(crate) fn new_window(&mut self, w: Widget) -> WindowId {
    let window = Window::from_event_loop(w, &self.event_loop, self.ctx.clone());

    let id = window.raw_window.id();
    self.windows.insert(id, window);

    id
  }
}

impl<'a> Default for Application {
  fn default() -> Self {
    Self {
      windows: Default::default(),
      event_loop: EventLoop::new(),
      ctx: <_>::default(),
    }
  }
}
