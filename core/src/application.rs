use crate::{
  prelude::*,
  timer::{new_timer, recently_timeout, wake_timeout_futures},
};
use rxrust::scheduler::NEW_TIMER_FN;
use std::{collections::HashMap, rc::Rc};
pub use winit::window::WindowId;
use winit::{
  event::{Event, StartCause, WindowEvent},
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
  pub fn new(theme: FullTheme) -> Self {
    let app_theme = Rc::new(Theme::Full(theme));
    let ctx = AppContext {
      app_theme: app_theme.clone(),
      ..Default::default()
    };
    ctx.load_font_from_theme(app_theme);

    let _ = NEW_TIMER_FN.set(new_timer);
    Self { ctx, ..Default::default() }
  }

  #[inline]
  pub fn with_theme(mut self, theme: FullTheme) -> Application {
    let app_theme = Rc::new(Theme::Full(theme));
    self.ctx.app_theme = app_theme.clone();
    self.ctx.load_font_from_theme(app_theme);
    self
  }

  #[inline]
  pub fn context(&self) -> &AppContext { &self.ctx }

  pub fn event_loop(&self) -> &EventLoop<()> { &self.event_loop }

  pub fn exec(mut self, wnd_id: WindowId) {
    if let Some(wnd) = self.windows.get_mut(&wnd_id) {
      wnd.draw_frame();
    } else {
      panic!("application at least have one window");
    }

    let Self { mut windows, mut event_loop, .. } = self;
    event_loop.run_return(
      move |event, _event_loop, control: &mut ControlFlow| match event {
        Event::WindowEvent { event, window_id } => {
          if event == WindowEvent::CloseRequested {
            windows.remove(&window_id);
          } else if event == WindowEvent::Destroyed {
            if windows.is_empty() {
              *control = ControlFlow::Exit;
            }
          } else if let Some(wnd) = windows.get_mut(&window_id) {
            wnd.processes_native_event(event);
          }
        }
        Event::MainEventsCleared => {
          windows.iter_mut().for_each(|(_, wnd)| {
            wnd.run_futures();
            if wnd.need_draw() {
              wnd.request_redraw();
            }
          });
        }
        Event::RedrawRequested(id) => {
          if let Some(wnd) = windows.get_mut(&id) {
            wnd.draw_frame();
          }
        }
        Event::RedrawEventsCleared => {
          if windows.iter_mut().any(|(_, wnd)| wnd.need_draw()) {
            *control = ControlFlow::Poll;
          } else if let Some(t) = recently_timeout() {
            *control = ControlFlow::WaitUntil(t);
          } else {
            *control = ControlFlow::Wait;
          };
        }
        Event::NewEvents(cause) => match cause {
          StartCause::Poll | StartCause::ResumeTimeReached { start: _, requested_resume: _ } => {
            wake_timeout_futures();
          }
          _ => (),
        },
        _ => (),
      },
    );
  }

  pub fn add_window(&mut self, wnd: Window) -> WindowId {
    let id = wnd.raw_window.id();
    self.windows.insert(id, wnd);

    id
  }
}

impl Default for Application {
  fn default() -> Self {
    Self {
      windows: Default::default(),
      event_loop: EventLoop::new(),
      ctx: <_>::default(),
    }
  }
}
