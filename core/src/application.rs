use crate::{prelude::*, widget::window::*};
use std::collections::HashMap;
pub use winit::window::WindowId;
use winit::{
  event::Event,
  event_loop::{ControlFlow, EventLoop},
};

pub struct Application {
  windows: HashMap<WindowId, Window>,
  event_loop: EventLoop<()>,
}

impl Application {
  #[inline]
  pub fn new() -> Application { <_>::default() }

  pub fn run(mut self, w: BoxedWidget) {
    let wnd_id = self.new_window(w);
    if let Some(wnd) = self.windows.get_mut(&wnd_id) {
      wnd.render_ready();
      wnd.draw_frame();
    }
    let Self { event_loop, mut windows, .. } = self;

    event_loop.run(move |event, _event_loop, control_flow| {
      *control_flow = ControlFlow::Wait;

      match event {
        Event::WindowEvent { event, window_id } => {
          if let Some(wnd) = windows.get_mut(&window_id) {
            wnd.processes_native_event(event);
          }
        }
        Event::MainEventsCleared => windows.iter_mut().for_each(|(_, wnd)| {
          if wnd.render_ready() {
            wnd.request_redraw();
          }
        }),
        Event::RedrawRequested(id) => {
          if let Some(wnd) = windows.get_mut(&id) {
            wnd.draw_frame();
          }
        }
        _ => (),
      }
    });
  }

  pub(crate) fn new_window(&mut self, w: BoxedWidget) -> WindowId {
    let window = Window::from_event_loop(w, &self.event_loop);
    let id = window.raw_window.borrow().id();
    self.windows.insert(id, window);
    id
  }
}

impl<'a> Default for Application {
  fn default() -> Self {
    Self {
      windows: Default::default(),
      event_loop: EventLoop::new(),
    }
  }
}
