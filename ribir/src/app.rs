use crate::clipboard::Clipboard;
use crate::winit_shell_wnd::{new_id, WinitShellWnd};
use ribir_core::timer::Timer;
use ribir_core::{prelude::*, window::WindowId};
use rxrust::scheduler::NEW_TIMER_FN;
use std::collections::HashMap;
use winit::event_loop::{EventLoopBuilder, EventLoopProxy};
use winit::{
  event::{Event, StartCause, WindowEvent},
  event_loop::{ControlFlow, EventLoop},
  platform::run_return::EventLoopExtRunReturn,
};

pub struct App {
  windows: HashMap<WindowId, Window>,
  event_loop: EventLoop<RibirEvent>,
  active_wnd: Option<WindowId>,
}

pub enum RibirEvent {
  LocalFuturesReady,
}

#[derive(Clone)]
pub struct EventSender(EventLoopProxy<RibirEvent>);

impl App {
  /// Create an application with a theme, caller should init your widgets
  /// library with the theme before create the application.
  pub fn new(theme: FullTheme) -> Self {
    let event_loop = EventLoopBuilder::with_user_event().build();
    let waker = EventSender(event_loop.create_proxy());
    let clipboard = Clipboard::new().unwrap();
    unsafe {
      AppCtx::set_app_theme(theme);
      AppCtx::set_clipboard(Box::new(clipboard));
      AppCtx::set_runtime_waker(Box::new(waker));
    }
    let _ = NEW_TIMER_FN.set(Timer::new_timer_future);
    Self {
      windows: Default::default(),
      event_loop,
      active_wnd: None,
    }
  }

  /// Start an application with the `root` widget, this will use the default
  /// theme to create an application and use the `root` widget to create a
  /// window, then run the application.
  pub fn run(root: Widget) {
    let mut app = App::default();
    app.new_window(root, None);
    app.exec()
  }

  pub fn new_window(&mut self, root: Widget, size: Option<Size>) -> &mut Window {
    let shell_wnd = WinitShellWnd::new(size, &self.event_loop);
    let wnd = Window::new(root, Box::new(shell_wnd));
    let id = wnd.id();
    assert!(self.windows.get(&id).is_none());
    if self.active_wnd.is_none() {
      self.active_wnd = Some(id);
    }
    self.windows.entry(id).or_insert(wnd)
  }

  pub fn event_loop(&self) -> &EventLoop<RibirEvent> { &self.event_loop }

  pub fn set_active_window(&mut self, id: WindowId) { self.active_wnd = Some(id); }

  pub fn exec(&mut self) {
    self
      .active_wnd
      .and_then(|id| self.windows.get_mut(&id))
      .expect("application at least have one window")
      .draw_frame();

    let Self { windows, event_loop, .. } = self;

    event_loop.run_return(move |event, _event_loop, control: &mut ControlFlow| {
      *control = ControlFlow::Wait;

      match event {
        Event::WindowEvent { event, window_id } => {
          if let Some(wnd) = windows.get_mut(&new_id(window_id)) {
            match event {
              WindowEvent::CloseRequested => {
                windows.remove(&new_id(window_id));
                if windows.is_empty() {
                  *control = ControlFlow::Exit;
                }
              }
              WindowEvent::Resized(size) => {
                let scale = wnd.device_pixel_ratio();
                let size = size.to_logical(scale as f64);
                let size = Size::new(size.width, size.height);
                let shell_wnd = wnd
                  .shell_wnd_mut()
                  .as_any_mut()
                  .downcast_mut::<WinitShellWnd>()
                  .unwrap();
                shell_wnd.on_resize(size);
                wnd.on_wnd_resize_event(size);
              }
              event => {
                #[allow(deprecated)]
                wnd.processes_native_event(event);
              }
            }
          }
        }
        Event::MainEventsCleared => {
          AppCtx::run_until_stalled();
          windows.iter_mut().for_each(|(_, wnd)| {
            if wnd.need_draw() {
              let wnd = wnd
                .shell_wnd()
                .as_any()
                .downcast_ref::<WinitShellWnd>()
                .unwrap();
              wnd.winit_wnd.request_redraw();
            }
          })
        }
        Event::RedrawRequested(id) => {
          if let Some(wnd) = windows.get_mut(&new_id(id)) {
            wnd.draw_frame();
          }
        }
        Event::RedrawEventsCleared => {
          if windows.iter_mut().any(|(_, wnd)| wnd.need_draw()) {
            *control = ControlFlow::Poll;
          } else if let Some(t) = Timer::recently_timeout() {
            *control = ControlFlow::WaitUntil(t);
          } else {
            *control = ControlFlow::Wait;
          };
        }
        Event::NewEvents(cause) => match cause {
          StartCause::Poll | StartCause::ResumeTimeReached { start: _, requested_resume: _ } => {
            Timer::wake_timeout_futures();
          }
          _ => (),
        },
        Event::UserEvent(event) => match event {
          RibirEvent::LocalFuturesReady => AppCtx::run_until_stalled(),
        },
        _ => (),
      }
    });
  }
}

impl Default for App {
  fn default() -> Self {
    let theme = FullTheme::default();
    App::new(theme)
  }
}

impl RuntimeWaker for EventSender {
  fn clone_box(&self) -> Box<dyn RuntimeWaker + Send> { Box::new(self.clone()) }
  fn wake(&self) { let _ = self.0.send_event(RibirEvent::LocalFuturesReady); }
}
