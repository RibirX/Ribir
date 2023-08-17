use crate::clipboard::Clipboard;
use crate::register_platform_app_events_handlers;
use crate::winit_shell_wnd::{new_id, WinitShellWnd};
use ribir_core::{prelude::*, timer::Timer, window::WindowId};
use rxrust::scheduler::NEW_TIMER_FN;
use std::rc::Rc;
use std::{convert::Infallible, sync::Once};
use winit::{
  event::{Event, StartCause, WindowEvent},
  event_loop::{ControlFlow, EventLoop, EventLoopBuilder, EventLoopProxy},
  platform::run_return::EventLoopExtRunReturn,
};

pub struct App {
  event_loop: EventLoop<AppEvent>,
  active_wnd: Option<WindowId>,
  events_stream: MutRefItemSubject<'static, AppEvent, Infallible>,
}

pub enum AppEvent {
  /// The event is sent when any future is waked to poll.
  FuturesWake,
  /// The event is sent when the application is be required to open a url. For
  /// example, it's launched from browser with a url.
  OpenUrl(String),
  /// The custom event, you can send any data with this event.
  Custom(Box<dyn Any + Send>),
}

/// A sender to send event to the application event loop from which the
/// `EventSender` was created.
#[derive(Clone)]
pub struct EventSender(EventLoopProxy<AppEvent>);

#[derive(Clone)]
pub struct EventWaker(EventLoopProxy<AppEvent>);

impl App {
  /// Start an application with the `root` widget, this will use the default
  /// theme to create an application and use the `root` widget to create a
  /// window, then run the application.
  #[track_caller]
  pub fn run(root: Widget) {
    Self::new_window(root, None);
    App::exec()
  }

  /// create a new window with the `root` widget and return the window id.
  #[track_caller]
  pub fn new_window(root: Widget, size: Option<Size>) -> WindowId {
    let app = unsafe { App::shared_mut() };
    let shell_wnd = WinitShellWnd::new(size, &app.event_loop);
    let wnd = Window::new(root, Box::new(shell_wnd));
    let id = wnd.id();
    AppCtx::windows().borrow_mut().insert(id, wnd);
    if app.active_wnd.is_none() {
      app.active_wnd = Some(id);
    }
    id
  }

  /// Get a event sender of the application event loop, you can use this to send
  /// event.
  pub fn event_sender() -> EventSender {
    let proxy = App::shared().event_loop.create_proxy();
    EventSender(proxy)
  }

  pub fn events_stream() -> MutRefItemSubject<'static, AppEvent, Infallible> {
    App::shared().events_stream.clone()
  }

  pub fn active_window() -> Rc<Window> {
    App::shared()
      .active_wnd
      .and_then(AppCtx::get_window)
      .expect("application at least have one window before use.")
  }

  /// set the window with `id` to be the active window, and the active window.
  #[track_caller]
  pub fn set_active_window(id: WindowId) {
    let app = unsafe { App::shared_mut() };
    app.active_wnd = Some(id);
    // todo: set the window to be the top window, but we not really support
    // multi window fully, implement this later.
  }

  /// run the application, this will start the event loop and block the current
  /// thread until the application exit.
  #[track_caller]
  pub fn exec() {
    Self::active_window().draw_frame();

    let event_loop = &mut unsafe { App::shared_mut() }.event_loop;

    event_loop.run_return(move |event, _event_loop, control: &mut ControlFlow| {
      *control = ControlFlow::Wait;
      match event {
        Event::WindowEvent { event, window_id } => {
          let wnd_id = new_id(window_id);
          let Some(wnd) =  AppCtx::get_window(wnd_id) else { return; };
          match event {
            WindowEvent::CloseRequested => {
              AppCtx::remove_wnd(wnd_id);
              if !AppCtx::has_wnd() {
                *control = ControlFlow::Exit;
              }
            }
            WindowEvent::Resized(size) => {
              let scale = wnd.device_pixel_ratio();
              let size = size.to_logical(scale as f64);
              let size = Size::new(size.width, size.height);
              let mut shell_wnd = wnd.shell_wnd().borrow_mut();
              let shell_wnd = shell_wnd
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
        Event::MainEventsCleared => {
          AppCtx::run_until_stalled();
          AppCtx::windows().borrow().values().for_each(|wnd| {
            if wnd.need_draw() {
              let wnd = wnd.shell_wnd().borrow();
              let shell = wnd.as_any().downcast_ref::<WinitShellWnd>().unwrap();
              shell.winit_wnd.request_redraw();
            }
          })
        }
        Event::RedrawRequested(id) => {
          if let Some(wnd) = AppCtx::get_window(new_id(id)) {
            wnd.draw_frame()
          }
        }
        Event::RedrawEventsCleared => {
          let need_draw = AppCtx::windows()
            .borrow()
            .values()
            .any(|wnd| wnd.need_draw());
          if need_draw {
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
        Event::UserEvent(mut event) => {
          if let AppEvent::FuturesWake = event {
            AppCtx::run_until_stalled()
          }
          let app = unsafe { App::shared_mut() };
          app.events_stream.next(&mut event);
        }
        _ => (),
      }
    });
  }

  #[track_caller]
  fn shared() -> &'static App { unsafe { Self::shared_mut() } }

  #[track_caller]
  unsafe fn shared_mut() -> &'static mut App {
    static mut INIT_ONCE: Once = Once::new();
    static mut APP: Option<App> = None;
    INIT_ONCE.call_once(|| {
      let event_loop = EventLoopBuilder::with_user_event().build();
      let waker = EventWaker(event_loop.create_proxy());
      let clipboard = Clipboard::new().unwrap();
      unsafe {
        AppCtx::set_clipboard(Box::new(clipboard));
        AppCtx::set_runtime_waker(Box::new(waker));
      }
      let _ = NEW_TIMER_FN.set(Timer::new_timer_future);
      register_platform_app_events_handlers();
      APP = Some(App {
        event_loop,
        events_stream: <_>::default(),
        active_wnd: None,
      })
    });
    AppCtx::thread_check();

    APP.as_mut().unwrap()
  }
}

impl EventSender {
  pub fn send(&self, e: AppEvent) {
    if let Err(err) = self.0.send_event(e) {
      log::error!("{}", err.to_string())
    }
  }
}

impl RuntimeWaker for EventWaker {
  fn clone_box(&self) -> Box<dyn RuntimeWaker + Send> { Box::new(self.clone()) }
  fn wake(&self) { let _ = self.0.send_event(AppEvent::FuturesWake); }
}

/// EventWaker only send `RibirEvent::FuturesWake`.
unsafe impl Send for EventWaker {}
