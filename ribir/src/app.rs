use std::{cell::RefCell, convert::Infallible, sync::LazyLock};

use app_event_handler::AppHandler;
use ribir_core::{
  local_sender::LocalSender,
  prelude::{image::ColorFormat, *},
  timer::Timer,
  window::WindowId,
};
use winit::{
  event::{ElementState, Ime, KeyEvent},
  event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy},
};

use crate::{
  register_platform_app_events_handlers,
  winit_shell_wnd::{WinitShellWnd, new_id},
};

mod app_event_handler;

pub struct App {
  event_loop: RefCell<Option<EventLoopState>>,
  event_loop_proxy: EventLoopProxy<AppEvent>,
  active_wnd: std::cell::Cell<Option<WindowId>>,
  events_stream: MutRefItemSubject<'static, AppEvent, Infallible>,
}

/// The attributes use to create a window.
pub struct WindowAttributes(winit::window::WindowAttributes);

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

/// A sender to send event to the application event loop from which the
/// `EventSender` was created.
#[derive(Clone)]
pub struct EventSender(EventLoopProxy<AppEvent>);

#[derive(Clone)]
pub struct EventWaker(EventLoopProxy<AppEvent>);

/// Represents the lifecycle states of an application's event loop
enum EventLoopState {
  /// Initialized but not yet running event loop.
  /// Contains the unstarted event loop configuration.
  NotStarted(Box<EventLoop<AppEvent>>),

  /// Active running event loop with guaranteed valid access.
  /// The reference is maintained and validated by the `AppEventHandler`.
  Running(&'static ActiveEventLoop),
}

impl App {
  pub fn events_stream() -> MutRefItemSubject<'static, AppEvent, Infallible> {
    App::shared().events_stream.clone()
  }

  fn process_winit_ime_event(wnd: &Window, ime: Ime) {
    match ime {
      Ime::Enabled => {}
      Ime::Preedit(txt, cursor) => {
        if txt.is_empty() {
          wnd.exit_pre_edit();
        } else {
          wnd.update_pre_edit(&txt, &cursor);
        }
      }
      Ime::Commit(value) => {
        wnd.exit_pre_edit();
        wnd.processes_receive_chars(value.into());
      }
      Ime::Disabled => wnd.exit_pre_edit(),
    }
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
  root: Option<GenWidget>,
  wnd_attrs: Option<WindowAttributes>,
  theme_initd: bool,
}

impl App {
  /// Start an application with the `root` widget, this will use the default
  /// theme to create an application and use the `root` widget to create a
  /// window, then run the application.
  #[track_caller]
  pub fn run<K: ?Sized>(root: impl RInto<GenWidget, K>) -> AppRunGuard {
    // Keep the application instance is created, when user call
    let _app = App::shared();
    AppRunGuard::new(root.r_into())
  }

  /// Get a event sender of the application event loop, you can use this to send
  /// event.
  pub fn event_sender() -> EventSender { EventSender(App::shared().event_loop_proxy.clone()) }

  /// Creates a new window containing the specified root widget.
  ///
  /// # Platform-specific Behavior: Web
  ///
  /// - Looks for the first DOM element with CSS class `ribir_container`:
  ///   - If found: Creates a canvas and appends it to this container
  ///   - After use, removes the `ribir_container` class from the element
  ///   - Subsequent windows will look for the next container with this class
  /// - If no container found, creates and appends the canvas to the body.
  pub async fn new_window(root: GenWidget, attrs: WindowAttributes) -> Sc<Window> {
    let shell_wnd = WinitShellWnd::new(attrs.0).await;
    let wnd = AppCtx::new_window(Box::new(shell_wnd), root);

    let app = App::shared();
    if app.active_wnd.get().is_none() {
      app.active_wnd.set(Some(wnd.id()));
    }
    wnd
  }

  pub fn active_window() -> Sc<Window> {
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

  /// run the application, this will start the event loop and block the current
  /// thread until the application exit.
  #[track_caller]
  pub fn exec() {
    let event_loop = App::take_event_loop();

    #[cfg(not(target_family = "wasm"))]
    let _ = event_loop.run_app(&mut AppHandler::default());

    #[cfg(target_family = "wasm")]
    winit::platform::web::EventLoopExtWebSys::spawn_app(event_loop, AppHandler::default());
  }

  #[track_caller]
  fn shared() -> &'static App {
    static APP: LazyLock<LocalSender<App>> = LazyLock::new(|| {
      let event_loop = EventLoop::with_user_event().build().unwrap();
      let waker = EventWaker(event_loop.create_proxy());

      #[cfg(not(target_family = "wasm"))]
      AppCtx::set_clipboard(Box::new(crate::clipboard::Clipboard::new().unwrap()));
      AppCtx::set_runtime_waker(Box::new(waker));

      register_platform_app_events_handlers();
      let event_loop = Box::new(event_loop);
      let app = App {
        event_loop_proxy: event_loop.create_proxy(),
        event_loop: RefCell::new(Some(EventLoopState::NotStarted(event_loop))),
        events_stream: <_>::default(),
        active_wnd: std::cell::Cell::new(None),
      };
      LocalSender::new(app)
    });
    &APP
  }

  fn take_event_loop() -> EventLoop<AppEvent> {
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
  fn new(root: GenWidget) -> Self {
    static ONCE: std::sync::Once = std::sync::Once::new();
    assert!(!ONCE.is_completed(), "App::run can only be called once.");
    ONCE.call_once(|| {});

    Self { root: Some(root), wnd_attrs: Some(WindowAttributes::default()), theme_initd: false }
  }

  /// Set the application theme, this will apply to whole application.
  pub fn with_app_theme(&mut self, theme: Theme) -> &mut Self {
    AppCtx::set_app_theme(theme);
    self.theme_initd = true;
    self
  }
}

impl Drop for AppRunGuard {
  fn drop(&mut self) {
    AppCtx::run_until_stalled();
    #[cfg(feature = "ribir_material")]
    if !self.theme_initd {
      AppCtx::set_app_theme(ribir_material::purple::light());
    }

    let root = self.root.take().unwrap();
    let attr = self.wnd_attrs.take().unwrap();
    AppCtx::spawn_local(async move {
      let _ = App::new_window(root, attr).await;
    })
    .unwrap();
    App::exec();
  }
}

impl EventSender {
  pub fn send(&self, e: AppEvent) {
    if let Err(err) = self.0.send_event(e) {
      log::error!("{}", err)
    }
  }
}

impl RuntimeWaker for EventWaker {
  fn clone_box(&self) -> Box<dyn RuntimeWaker + Send> { Box::new(self.clone()) }
  fn wake(&self) { let _ = self.0.send_event(AppEvent::FuturesWake); }
}

/// EventWaker only send `RibirEvent::FuturesWake`.
unsafe impl Send for EventWaker {}

pub(crate) fn request_redraw(wnd: &Window) {
  let wnd = wnd.shell_wnd().borrow();
  let shell = wnd
    .as_any()
    .downcast_ref::<WinitShellWnd>()
    .unwrap();
  shell.winit_wnd.request_redraw();
}

fn into_winit_size(size: Size) -> winit::dpi::Size {
  winit::dpi::LogicalSize::new(size.width, size.height).into()
}

impl WindowAttributes {
  /// Initial title of the window in the title bar.
  ///
  /// Default: `"Ribir App"`
  pub fn with_title(&mut self, title: impl Into<String>) -> &mut Self {
    self.0.title = title.into();
    self
  }

  /// Whether the window should be resizable.
  ///
  /// Default: `true`
  pub fn with_resizable(&mut self, resizable: bool) -> &mut Self {
    self.0.resizable = resizable;
    self
  }

  /// Initial size of the window client area (excluding decorations).
  pub fn with_size(&mut self, size: Size) -> &mut Self {
    self.0.inner_size = Some(into_winit_size(size));
    self
  }

  /// Minimum size of the window client area
  pub fn with_min_size(&mut self, size: Size) -> &mut Self {
    self.0.min_inner_size = Some(into_winit_size(size));
    self
  }

  /// Maximum size of the window client area
  pub fn with_max_size(&mut self, size: Size) -> &mut Self {
    self.0.max_inner_size = Some(into_winit_size(size));
    self
  }

  /// Initial position of the window in screen coordinates.
  pub fn position(mut self, position: Point) -> Self {
    self.0.position = Some(winit::dpi::LogicalPosition::new(position.x, position.y).into());
    self
  }

  /// Whether the window should start maximized.
  ///
  /// Default: `false`
  pub fn with_maximized(&mut self, maximized: bool) -> &mut Self {
    self.0.maximized = maximized;
    self
  }

  /// Initial window visibility.
  ///
  /// Default: `true`
  pub fn with_visible(&mut self, visible: bool) -> &mut Self {
    self.0.visible = visible;
    self
  }

  /// Whether the window should show decorations.
  ///
  /// Default: `true`
  pub fn with_decorations(&mut self, decorations: bool) -> &mut Self {
    self.0.decorations = decorations;
    self
  }

  /// Window icon in RGBA8 format.
  pub fn with_icon(&mut self, icon: &PixelImage) -> &mut Self {
    debug_assert!(icon.color_format() == ColorFormat::Rgba8, "Icon must be in RGBA8 format");

    self.0.window_icon =
      winit::window::Icon::from_rgba(icon.pixel_bytes().to_vec(), icon.width(), icon.height()).ok();

    self
  }
}

impl Default for WindowAttributes {
  fn default() -> Self { Self(winit::window::WindowAttributes::default().with_title("Ribir App")) }
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

  use super::App;

  #[test]
  fn ime_pre_edit() {
    let log = Stateful::new(vec![]);
    let log2 = log.clone_writer();

    let mut wnd = TestWindow::new_with_size(
      fn_widget! {
        @MockBox {
          size: INFINITY_SIZE,
          auto_focus: true,
          on_ime_pre_edit: move |e| {
            match &e.pre_edit {
              ImePreEdit::Begin => $log2.write().push("on_ime_pre_edit_begin".to_string()),
              ImePreEdit::PreEdit { value, .. } => $log2.write().push(format!("on_ime_pre_edit_update {value}")),
              ImePreEdit::End => $log2.write().push("on_ime_pre_edit_end".to_string()),
            }
          },
          on_chars: move|e| $log2.write().push(format!("on_chars {}", e.chars)),
          on_tap: move |_| $log2.write().push("on_tap".to_string()),
        }
      },
      Size::new(200., 200.),
    );

    wnd.draw_frame();

    App::process_winit_ime_event(&wnd, Ime::Enabled);
    App::process_winit_ime_event(&wnd, Ime::Preedit("hello".to_string(), None));
    App::process_winit_ime_event(&wnd, Ime::Disabled);
    wnd.draw_frame();
    assert_eq!(
      &*log.read(),
      &["on_ime_pre_edit_begin", "on_ime_pre_edit_update hello", "on_ime_pre_edit_end"]
    );

    log.write().clear();
    App::process_winit_ime_event(&wnd, Ime::Preedit("hello".to_string(), None));
    App::process_winit_ime_event(&wnd, Ime::Commit("hello".to_string()));
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
    App::process_winit_ime_event(&wnd, Ime::Preedit("hello".to_string(), None));
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
