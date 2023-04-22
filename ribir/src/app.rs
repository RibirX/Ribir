use crate::{
  prelude::*,
  timer::{new_timer, recently_timeout, wake_timeout_futures},
};
use rxrust::scheduler::NEW_TIMER_FN;
use ribir_core::{
  prelude::*,
  window::{ShellWindow, WindowId},
};
use ribir_widgets::prelude::*;
use std::{collections::HashMap, rc::Rc};
use winit::{
  event::{Event, StartCause, WindowEvent},
  event_loop::{ControlFlow, EventLoop},
  platform::run_return::EventLoopExtRunReturn,
};
use crate::winit_shell_wnd::new_id;

pub struct App {
  windows: HashMap<WindowId, Window>,
  ctx: AppContext,
  event_loop: EventLoop<()>,
  active_wnd: Option<WindowId>,
}

impl App {
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

  pub fn run(root: Widget) { Self::run_by::<crate::winit_shell_wnd::WinitShellWnd>(root) }

  pub fn new_window(&mut self, root: Widget, size: Option<Size>) -> &mut Window {
    self.new_window_by::<crate::winit_shell_wnd::WinitShellWnd>(root, size)
  }

  fn run_by<S: ShellWindow + 'static>(root: Widget) {
    let mut app = App::new(material::purple::light());
    let id = app.new_window(root, None).set_title("ribir app").id();
    app.exec(id);
  }

  pub fn new_window_by<S: ShellWindow + 'static>(
    &mut self,
    root: Widget,
    size: Option<Size>,
  ) -> &mut Window {
    let wnd = Window::new::<S>(root, size, self.context().clone());
    let id = wnd.id();
    assert!(self.windows.get(&id).is_none());
    self.windows.entry(id).or_insert(wnd)
  }

  #[inline]
  pub fn context(&self) -> &AppContext { &self.ctx }

  pub fn event_loop(&self) -> &EventLoop<()> { &self.event_loop }

  pub fn exec(&mut self, id: WindowId) {
    self.active_wnd = Some(id);
    self
      .windows
      .get_mut(&id)
      .expect("application at least have one window")
      .draw_frame();

    let Self { windows, event_loop, .. } = self;

    event_loop.run_return(move |event, _event_loop, control: &mut ControlFlow| {
      *control = ControlFlow::Wait;

      match event {
        Event::WindowEvent { event, window_id } => {
          if event == WindowEvent::CloseRequested {
            windows.remove(&new_id(window_id));
          } else if event == WindowEvent::Destroyed {
            if windows.is_empty() {
              *control = ControlFlow::Exit;
            }
          } else if let Some(wnd) = windows.get_mut(&new_id(window_id)) {
            wnd.processes_native_event(event);
          }
        }
        Event::MainEventsCleared => windows.iter_mut().for_each(|(_, wnd)| {
          wnd.run_futures();
          if wnd.need_draw() {
            let wnd = wnd
              .shell_wnd()
              .as_any()
              .downcast_ref::<winit::window::Window>()
              .unwrap();
            wnd.request_redraw();
          }
        }),
        Event::RedrawRequested(id) => {
          if let Some(wnd) = windows.get_mut(&new_id(id)) {
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
}

impl Default for App {
  fn default() -> Self {
    Self {
      windows: Default::default(),
      event_loop: EventLoop::new(),
      ctx: <_>::default(),
      active_wnd: None,
    }
  }
}
