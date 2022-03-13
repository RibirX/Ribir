use crate::{animation::TickerProvider, prelude::*};
use std::collections::HashMap;
pub use winit::window::WindowId;
use winit::{event::Event, event_loop::EventLoop};

pub struct Application {
  windows: HashMap<WindowId, Window>,
  event_loop: EventLoop<()>,
}

impl Application {
  #[inline]
  pub fn new() -> Application { <_>::default() }

  pub fn run(mut self, w: BoxedWidget, animation_mgr: Option<Box<dyn TickerProvider>>) {
    let wnd_id = self.new_window(w, animation_mgr);
    let Self { event_loop, mut windows, .. } = self;

    if let Some(wnd) = windows.get_mut(&wnd_id) {
      wnd.render_ready();
      wnd.draw_frame();
    }

    event_loop.run(move |event, _event_loop, _| match event {
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
          if wnd.trigger_animation_ticker() {
            wnd.request_redraw();
          }
        }
      }
      _ => (),
    });
  }

  pub(crate) fn new_window(
    &mut self,
    w: BoxedWidget,
    animation_mgr: Option<Box<dyn TickerProvider>>,
  ) -> WindowId {
    let window = Window::from_event_loop(w, &self.event_loop, animation_mgr);

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
    }
  }
}
