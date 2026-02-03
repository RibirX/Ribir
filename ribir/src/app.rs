use std::{
  cell::RefCell,
  collections::HashMap,
  convert::Infallible,
  future::Future,
  sync::{Arc, LazyLock},
};

use app_event_handler::AppHandler;
use ribir_core::{
  local_sender::LocalSender,
  prelude::*,
  window::{BoxShell, BoxShellWindow, UiEvent, WindowAttributes, WindowFlags, WindowId},
};
use tokio::sync::mpsc::{UnboundedSender, unbounded_channel};
use winit::event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy};

use crate::{
  register_platform_app_events_handlers,
  winit_shell_wnd::{RibirShell, ShellCmd, ShellWndHandle, WinitShellWnd, new_id},
};

mod app_event_handler;
mod ui_executor;

pub struct App {
  event_loop: RefCell<Option<EventLoopState>>,
  //Cloning an EventLoopProxy has side effects—it activates the event loop—so wrap it in an Arc to
  // avoid affecting the timing of the event loop(EventLoopProxy::new will call CFRunLoopWakeUp
  // on mac).
  event_loop_proxy: Arc<EventLoopProxy<RibirAppEvent>>,
  windows: RefCell<HashMap<WindowId, Rc<RefCell<WinitShellWnd>>>>,
  active_wnd: std::cell::Cell<Option<WindowId>>,
  events_stream: LocalSubjectMutRef<'static, AppEvent, Infallible>,

  _app_handler: RefCell<Option<UnboundedSender<UiEvent>>>,
  ui_executor: ui_executor::UiExecutor,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HotkeyEvent {
  pub key_code: Option<KeyCode>,
  pub modifiers: Option<ModifiersState>,
}

pub enum AppEvent {
  /// The event is sent when any future is waked to poll.
  FuturesWake,
  /// The event is sent when the application is be required to open a url. For
  /// example, it's launched from browser with a url.
  OpenUrl(String),
  /// The event is get global hotkey, it will receive the hotkey event.
  Hotkey(HotkeyEvent),
  /// The event is sent when the application window focus changed.
  WndFocusChanged(WindowId, bool),
  /// The custom event, you can send any data with this event.
  Custom(Box<dyn Any + Send>),
}

pub enum RibirAppEvent {
  App(AppEvent),
  Cmd(ShellCmd),
}

/// A sender to send event to the application event loop from which the
/// `EventSender` was created.
#[derive(Clone)]
pub struct EventSender(Arc<EventLoopProxy<RibirAppEvent>>);

#[derive(Clone)]
pub(crate) struct CmdSender(Arc<EventLoopProxy<RibirAppEvent>>);

/// Represents the lifecycle states of an application's event loop
enum EventLoopState {
  /// Initialized but not yet running event loop.
  /// Contains the unstarted event loop configuration.
  NotStarted(Box<EventLoop<RibirAppEvent>>),

  /// Active running event loop with guaranteed valid access.
  /// The reference is maintained and validated by the `AppEventHandler`.
  Running(&'static ActiveEventLoop),
}

impl App {
  pub fn events_stream() -> LocalSubjectMutRef<'static, AppEvent, Infallible> {
    App::shared().events_stream.clone()
  }

  pub(crate) fn shell_window(id: WindowId) -> Option<Rc<RefCell<WinitShellWnd>>> {
    App::shared().windows.borrow().get(&id).cloned()
  }

  pub(crate) fn remove_shell_window(id: WindowId) {
    Self::shared().windows.borrow_mut().remove(&id);
  }

  pub(crate) fn send_event(event: UiEvent) {
    let _ = Self::shared()
      ._app_handler
      .borrow()
      .as_ref()
      .unwrap()
      .send(event);
  }
}

/// A guard returned by `App::run` that enables application configuration
/// and window creation before startup.
///
/// It dereferences to `WindowAttributes` for window attribute configuration.
///
/// Upon being dropped, it creates a new window with the `root` widget and
/// then calls `App::exec`.
pub struct AppRunGuard {
  root: Option<Box<dyn FnOnce() -> GenWidget + 'static + Send>>,
  theme: Option<Box<dyn FnOnce() -> Theme + Send + 'static>>,
  wnd_attrs: Option<WindowAttributes>,
}

impl App {
  /// Start an application with the `root` widget, this will use the default
  /// theme to create an application and use the `root` widget to create a
  /// window, then run the application.
  #[track_caller]
  pub fn run<K: ?Sized>(root: impl RInto<GenWidget, K> + Send + 'static) -> AppRunGuard {
    // Keep the application instance is created, when user call
    let _app = App::shared();
    AppRunGuard::new(move || root.r_into())
  }

  /// Start an application with the `root` widget, this will use the default
  /// theme to create an application and use the `root` widget to create a
  /// window, then run the application.
  /// Note:
  ///  1. different from `run`, when the app need to recreate the hold widget
  ///     again, it will use the same data build from `data_builder`.
  ///  2. as the application's logic will be run in a separated thread, so the
  ///     data need to be `Send` and lazy init.
  #[track_caller]
  pub fn run_with_data<K, Data: 'static, W: IntoWidget<'static, K>>(
    data_builder: impl FnOnce() -> Data + Send + 'static,
    widget_builder: impl Fn(&'static Data) -> W + Send + 'static,
  ) -> AppRunGuard {
    // Keep the application instance is created, when user call
    let _app = App::shared();

    AppRunGuard::new(move || {
      let data = data_builder();
      (move || {
        let ptr = &data as *const Data;
        widget_builder(unsafe { &*ptr }).into_widget()
      })
      .r_into()
    })
  }

  /// Get a event sender of the application event loop, you can use this to send
  /// event.
  pub fn event_sender() -> EventSender { EventSender(App::shared().event_loop_proxy.clone()) }

  pub(crate) fn cmd_sender() -> CmdSender { CmdSender(App::shared().event_loop_proxy.clone()) }

  /// Creates a new window containing the specified root widget.
  ///
  /// # Platform-specific Behavior: Web
  ///
  /// - Looks for the first DOM element with CSS class `ribir_container`:
  ///   - If found: Creates a canvas and appends it to this container
  ///   - After use, removes the `ribir_container` class from the element
  ///   - Subsequent windows will look for the next container with this class
  /// - If no container found, creates and appends the canvas to the body.
  pub async fn new_window(attrs: WindowAttributes) -> BoxShellWindow {
    let shell_wnd = WinitShellWnd::new(attrs).await;

    let proxy = ShellWndHandle {
      winit_wnd: shell_wnd.winit_wnd.clone(),
      sender: App::cmd_sender(),
      cursor: CursorIcon::Default,
    };

    let wid: WindowId = shell_wnd.id();

    let app = App::shared();
    app
      .windows
      .borrow_mut()
      .insert(wid, Rc::new(RefCell::new(shell_wnd)));
    if app.active_wnd.get().is_none() {
      app.active_wnd.set(Some(wid));
    }
    Box::new(proxy)
  }

  /// Creates a new shell window from an existing winit window.
  /// This is used for two-phase window creation where the winit window is
  /// created synchronously (in event loop callback) and backend is initialized
  /// asynchronously.
  pub(crate) async fn new_window_from_winit(
    winit_wnd: std::sync::Arc<winit::window::Window>,
  ) -> BoxShellWindow {
    let shell_wnd = WinitShellWnd::from_winit_window(winit_wnd).await;

    let proxy = ShellWndHandle {
      winit_wnd: shell_wnd.winit_wnd.clone(),
      sender: App::cmd_sender(),
      cursor: CursorIcon::Default,
    };

    let wid: WindowId = shell_wnd.id();

    let app = App::shared();
    app
      .windows
      .borrow_mut()
      .insert(wid, Rc::new(RefCell::new(shell_wnd)));
    if app.active_wnd.get().is_none() {
      app.active_wnd.set(Some(wid));
    }
    Box::new(proxy)
  }

  pub fn active_window() -> Rc<Window> {
    App::shared()
      .active_wnd
      .get()
      .and_then(AppCtx::get_window)
      .expect("application at least have one window before use.")
  }

  /// set the window with `id` to be the active window, and the active window.
  #[track_caller]
  pub fn set_active_window(id: WindowId) {
    App::shared().active_wnd.set(Some(id));

    // todo: set the window to be the top window, but we not really support
    // multi window fully, implement this later.

    if let Some(wnd) = AppCtx::get_window(id) {
      let mut shell = wnd.shell_wnd().borrow_mut();
      if shell.is_minimized() {
        shell.set_minimized(false);
      }
      shell.focus_window();
    };
  }

  pub(crate) fn spawn_local<Fut>(future: Fut)
  where
    Fut: Future<Output = ()> + 'static,
  {
    Self::shared().ui_executor.spawn_local(future);
  }

  /// run the application, this will start the event loop and block the current
  /// thread until the application exit.
  #[track_caller]
  pub fn exec(app: impl FnOnce() + Send + 'static) {
    let (sender, recv) = unbounded_channel();
    *Self::shared()._app_handler.borrow_mut() = Some(sender);
    let shell: BoxShell = Box::new(RibirShell { cmd_sender: App::cmd_sender() });

    AppCtx::run(recv, shell, async move {
      #[cfg(not(target_arch = "wasm32"))]
      AppCtx::set_clipboard(Box::new(crate::clipboard::Clipboard::new().unwrap()));

      app();
      register_platform_app_events_handlers()
    });

    let event_loop = App::take_event_loop();

    #[cfg(not(target_arch = "wasm32"))]
    let _ = event_loop.run_app(&mut AppHandler::default());

    #[cfg(target_arch = "wasm32")]
    winit::platform::web::EventLoopExtWebSys::spawn_app(event_loop, AppHandler::default());
  }

  #[track_caller]
  pub(crate) fn shared() -> &'static App {
    static APP: LazyLock<LocalSender<App>> = LazyLock::new(|| {
      let event_loop = EventLoop::with_user_event().build().unwrap();

      let event_loop = Box::new(event_loop);

      let event_loop_proxy = event_loop.create_proxy();

      let app: App = App {
        event_loop_proxy: Arc::new(event_loop.create_proxy()),
        event_loop: RefCell::new(Some(EventLoopState::NotStarted(event_loop))),
        events_stream: Local::subject_mut_ref(),
        active_wnd: std::cell::Cell::new(None),
        windows: <_>::default(),
        _app_handler: <_>::default(),
        ui_executor: ui_executor::UiExecutor::new(event_loop_proxy),
      };
      LocalSender::new(app)
    });

    &APP
  }

  #[track_caller]
  fn pump_ui_tasks() { App::shared().ui_executor.pump(); }

  fn take_event_loop() -> EventLoop<RibirAppEvent> {
    let app = App::shared();
    let mut event_loop = app.event_loop.borrow_mut();

    let event_loop = event_loop
      .take()
      .expect("Event loop already consumed.");
    match event_loop {
      EventLoopState::NotStarted(event_loop) => *event_loop,
      EventLoopState::Running(_) => panic!("Event loop already running."),
    }
  }

  /// Retrieves the active event loop instance for the current execution
  /// context.
  ///
  /// This provides access to platform-specific system resources including:
  /// - Window creation and management
  /// - System theme information
  /// - Monitor enumeration
  /// - Event processing control
  ///
  /// # Important Safety Notes
  ///
  /// 1. **Lifetime Management**: The returned event loop reference is transient
  ///    and should never be stored in long-lived structures. The underlying
  ///    instance may be invalidated or refreshed during event loop iterations.
  ///
  /// 2. **Thread Affinity**: This interface is main-thread constrained. Access
  ///    must occur exclusively through this accessor function to ensure thread
  ///    safety and prevent invalid state references.
  ///
  /// 3. **Reentrancy**: Avoid nested calls during event processing as this may
  ///    lead to undefined behavior in platform-specific implementations.
  pub(crate) fn active_event_loop() -> &'static ActiveEventLoop {
    let event_loop = App::shared().event_loop.borrow();
    let state = event_loop
      .as_ref()
      .expect("Event loop must be initialized before access");

    match state {
      EventLoopState::Running(event_loop) => event_loop,
      EventLoopState::NotStarted(_) => panic!("Event loop accessed in invalid state."),
    }
  }
}

impl AppRunGuard {
  fn new<W: FnOnce() -> GenWidget + Send + 'static>(root: W) -> Self {
    static ONCE: std::sync::Once = std::sync::Once::new();
    assert!(!ONCE.is_completed(), "App::run can only be called once.");
    ONCE.call_once(|| {});

    let theme: Option<Box<dyn FnOnce() -> Theme + Send + 'static>> = {
      cfg_if::cfg_if! {
        if #[cfg(feature = "ribir_material")] {
          Some(Box::new(ribir_material::purple::light))
        } else {
          None
        }
      }
    };

    Self { root: Some(Box::new(root)), wnd_attrs: Some(WindowAttributes::default()), theme }
  }

  /// Set the application theme, this will apply to whole application.
  pub fn with_app_theme(&mut self, theme: impl FnOnce() -> Theme + Send + 'static) -> &mut Self {
    self.theme = Some(Box::new(theme));
    self
  }
}

impl Drop for AppRunGuard {
  fn drop(&mut self) {
    let root = self.root.take().unwrap();
    let attr = self.wnd_attrs.take().unwrap();
    let theme = self.theme.take();

    App::exec(move || {
      if let Some(theme) = theme {
        AppCtx::set_app_theme(theme());
      }

      AppCtx::spawn_local(async move {
        AppCtx::new_window(root(), WindowFlags::DEFAULT, attr).await;
      });
    });
  }
}

impl EventSender {
  pub fn send(&self, e: AppEvent) {
    if let Err(err) = self.0.send_event(RibirAppEvent::App(e)) {
      log::error!("{}", err)
    }
  }
}

impl CmdSender {
  pub fn send(&self, cmd: ShellCmd) {
    if let Err(err) = self.0.send_event(RibirAppEvent::Cmd(cmd)) {
      log::error!("{}", err)
    }
  }
}

impl std::ops::Deref for AppRunGuard {
  type Target = WindowAttributes;

  fn deref(&self) -> &Self::Target { unsafe { self.wnd_attrs.as_ref().unwrap_unchecked() } }
}

impl std::ops::DerefMut for AppRunGuard {
  fn deref_mut(&mut self) -> &mut Self::Target {
    unsafe { self.wnd_attrs.as_mut().unwrap_unchecked() }
  }
}

#[cfg(test)]
mod tests {

  use ribir_core::{prelude::*, test_helper::*};
  use winit::event::Ime;

  #[test]
  fn ime_pre_edit() {
    reset_test_env!();
    let log = Stateful::new(vec![]);
    let log2 = log.clone_writer();

    let wnd = TestWindow::new_with_size(
      fn_widget! {
        @MockBox {
          size: INFINITY_SIZE,
          auto_focus: true,
          on_ime_pre_edit: move |e| {
            match &e.pre_edit {
              ImePreEdit::Begin => $write(log2).push("on_ime_pre_edit_begin".to_string()),
              ImePreEdit::PreEdit { value, .. } => $write(log2).push(format!("on_ime_pre_edit_update {value}")),
              ImePreEdit::End => $write(log2).push("on_ime_pre_edit_end".to_string()),
            }
          },
          on_chars: move|e| $write(log2).push(format!("on_chars {}", e.chars)),
          on_tap: move |_| $write(log2).push("on_tap".to_string()),
        }
      },
      Size::new(200., 200.),
    );

    wnd.draw_frame();

    wnd.process_ime(Ime::Enabled);
    wnd.process_ime(Ime::Preedit("hello".to_string(), None));
    wnd.process_ime(Ime::Disabled);
    wnd.draw_frame();
    assert_eq!(
      &*log.read(),
      &["on_ime_pre_edit_begin", "on_ime_pre_edit_update hello", "on_ime_pre_edit_end"]
    );

    log.write().clear();

    wnd.process_ime(Ime::Preedit("hello".to_string(), None));
    wnd.process_ime(Ime::Commit("hello".to_string()));
    wnd.draw_frame();
    assert_eq!(
      &*log.read(),
      &[
        "on_ime_pre_edit_begin",
        "on_ime_pre_edit_update hello",
        "on_ime_pre_edit_end",
        "on_chars hello",
      ]
    );

    log.write().clear();
    wnd.process_ime(Ime::Preedit("hello".to_string(), None));
    wnd.force_exit_pre_edit();

    wnd.process_mouse_press(Box::new(DummyDeviceId), MouseButtons::PRIMARY);
    wnd.process_mouse_release(Box::new(DummyDeviceId), MouseButtons::PRIMARY);

    wnd.draw_frame();
    assert_eq!(
      &*log.read(),
      &[
        "on_ime_pre_edit_begin",
        "on_ime_pre_edit_update hello",
        "on_ime_pre_edit_end",
        "on_chars hello",
        "on_tap",
      ]
    );
  }
}
