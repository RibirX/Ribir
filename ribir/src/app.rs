use crate::clipboard::Clipboard;
use crate::winit_shell_wnd::{new_id, WinitShellWnd};
use ribir_core::timer::Timer;
use ribir_core::{prelude::*, window::WindowId};
use rxrust::scheduler::NEW_TIMER_FN;
use std::collections::HashMap;
use std::sync::Once;
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
  /// Start an application with the `root` widget, this will use the default
  /// theme to create an application and use the `root` widget to create a
  /// window, then run the application.
  #[track_caller]
  pub fn run(root: Widget) {
    Self::new_window(root, None, |_| {});
    App::exec()
  }

  /// create a new window with the `root` widget, and you can config the new
  /// window in the callback, then return the window id.
  #[track_caller]
  pub fn new_window(root: Widget, size: Option<Size>, cb: impl FnOnce(&mut Window)) -> WindowId {
    let app = unsafe { App::shared() };

    let shell_wnd = WinitShellWnd::new(size, &app.event_loop);
    let wnd = Window::new(root, Box::new(shell_wnd));
    let id = wnd.id();
    if app.active_wnd.is_none() {
      app.active_wnd = Some(id);
    }
    app.windows.insert(id, wnd);
    cb(app.windows.get_mut(&id).unwrap());

    id
  }

  /// set the window with `id` to be the active window, and the active window.
  #[track_caller]
  pub fn set_active_window(id: WindowId) {
    let app = unsafe { App::shared() };
    app.active_wnd = Some(id);
    // todo: set the window to be the top window, but we not really support
    // multi window fully, implement this later.
  }

  /// run the application, this will start the event loop and block the current
  /// thread until the application exit.
  #[track_caller]
  pub fn exec() {
    let app = unsafe { App::shared() };
    app
      .active_wnd
      .and_then(|id| app.windows.get_mut(&id))
      .expect("application at least have one window")
      .draw_frame();

    let Self { windows, event_loop, .. } = app;

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

  #[track_caller]
  unsafe fn shared() -> &'static mut App {
    static mut INIT_ONCE: Once = Once::new();
    static mut APP: Option<App> = None;
    INIT_ONCE.call_once(|| {
      let event_loop = EventLoopBuilder::with_user_event().build();
      let waker = EventSender(event_loop.create_proxy());
      let clipboard = Clipboard::new().unwrap();
      unsafe {
        AppCtx::set_clipboard(Box::new(clipboard));
        AppCtx::set_runtime_waker(Box::new(waker));
      }
      let _ = NEW_TIMER_FN.set(Timer::new_timer_future);
      APP = Some(App {
        windows: Default::default(),
        event_loop,
        active_wnd: None,
      })
    });
    AppCtx::thread_check();

    APP.as_mut().unwrap()
  }
}

impl RuntimeWaker for EventSender {
  fn clone_box(&self) -> Box<dyn RuntimeWaker + Send> { Box::new(self.clone()) }
  fn wake(&self) { let _ = self.0.send_event(RibirEvent::LocalFuturesReady); }
}
