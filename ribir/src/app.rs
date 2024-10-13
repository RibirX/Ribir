use std::{cell::RefCell, convert::Infallible, sync::LazyLock};

use ribir_core::{local_sender::LocalSender, prelude::*, timer::Timer, window::WindowId};
use winit::{
  event::{ElementState, Event, Ime, KeyEvent, StartCause, WindowEvent},
  event_loop::{ControlFlow, EventLoop, EventLoopBuilder, EventLoopProxy, EventLoopWindowTarget},
};

use crate::{
  register_platform_app_events_handlers,
  winit_shell_wnd::{WinitShellWnd, new_id},
};

pub struct App {
  event_loop_proxy: EventLoopProxy<AppEvent>,
  /// The event loop of the application, it's only available on native platform
  /// after `App::exec` called
  event_loop: RefCell<Option<EventLoop<AppEvent>>>,
  #[cfg(not(target_family = "wasm"))]
  active_wnd: std::cell::Cell<Option<WindowId>>,
  events_stream: MutRefItemSubject<'static, AppEvent, Infallible>,
}

/// Attributes for creating a new window.
pub struct WindowAttributes {
  pub resizable: bool,
  pub maximized: bool,
  pub visible: bool,
  pub decorations: bool,
  pub title: String,
  pub size: Option<Size>,
  pub min_size: Option<Size>,
  pub max_size: Option<Size>,
  pub position: Option<Point>,
  pub icon: Option<Resource<PixelImage>>,
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

/// A sender to send event to the application event loop from which the
/// `EventSender` was created.
#[derive(Clone)]
pub struct EventSender(EventLoopProxy<AppEvent>);

#[derive(Clone)]
pub struct EventWaker(EventLoopProxy<AppEvent>);

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
        wnd.processes_receive_chars(value);
      }
      Ime::Disabled => wnd.exit_pre_edit(),
    }
  }

  fn event_loop_handle(event: Event<AppEvent>, loop_handle: &EventLoopWindowTarget<AppEvent>) {
    match event {
      Event::WindowEvent { event, window_id } => {
        let wnd_id = new_id(window_id);
        let Some(wnd) = AppCtx::get_window(wnd_id) else {
          return;
        };
        match event {
          WindowEvent::CloseRequested => {
            AppCtx::remove_wnd(wnd_id);
            if !AppCtx::has_wnd() {
              loop_handle.exit();
            }
          }
          WindowEvent::RedrawRequested => {
            AppCtx::frame_ticks().clone().next(Instant::now());

            if let Some(wnd) = AppCtx::get_window(wnd_id) {
              // if the window is not visible, don't draw it./
              if wnd.is_visible() != Some(false) {
                // if this frame is really draw, request another redraw. To make sure the draw
                // always end with a empty draw and emit an extra tick cycle message.
                if wnd.draw_frame() {
                  request_redraw(&wnd);
                }
              }
            }
          }
          WindowEvent::Resized(_) => {
            let size = wnd.shell_wnd().borrow().inner_size();
            wnd.shell_wnd().borrow_mut().on_resize(size);
            request_redraw(&wnd)
          }
          WindowEvent::Focused(focused) => {
            let mut event = AppEvent::WndFocusChanged(wnd_id, focused);

            App::shared()
              .events_stream
              .clone()
              .next(&mut event);
          }
          WindowEvent::KeyboardInput { event, .. } if !wnd.is_pre_editing() => {
            let KeyEvent { physical_key, logical_key, text, location, repeat, state, .. } = event;
            wnd.processes_keyboard_event(physical_key, logical_key, repeat, location, state);
            if state == ElementState::Pressed {
              if let Some(txt) = text {
                wnd.processes_receive_chars(txt.to_string());
              }
            }
          }
          WindowEvent::Ime(ime) => App::process_winit_ime_event(&wnd, ime),
          WindowEvent::MouseInput { state, button, device_id, .. } => {
            if state == ElementState::Pressed {
              wnd.force_exit_pre_edit()
            }
            wnd.process_mouse_input(device_id, state, button);
          }
          #[allow(deprecated)]
          event => wnd.processes_native_event(event),
        }
        wnd.emit_events();

        if wnd.need_draw() {
          request_redraw(&wnd)
        }
      }
      Event::AboutToWait => {
        let run_count = AppCtx::run_until_stalled();
        if run_count > 0 {
          for wnd in AppCtx::windows().borrow().values() {
            request_redraw(wnd);
          }
        }
        if run_count > 0 {
          loop_handle.set_control_flow(ControlFlow::Poll);
        } else if let Some(t) = Timer::recently_timeout() {
          let control = ControlFlow::wait_duration(t.duration_since(Instant::now()));
          loop_handle.set_control_flow(control);
        } else {
          loop_handle.set_control_flow(ControlFlow::Wait);
        };
      }
      Event::NewEvents(StartCause::Poll | StartCause::ResumeTimeReached { .. }) => {
        Timer::wake_timeout_futures()
      }
      Event::UserEvent(mut event) => {
        AppCtx::spawn_local(async move {
          App::shared()
            .events_stream
            .clone()
            .next(&mut event);
        })
        .unwrap();
      }
      _ => (),
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
  pub fn run(root: impl Into<GenWidget>) -> AppRunGuard { AppRunGuard::new(root.into()) }

  /// Get a event sender of the application event loop, you can use this to send
  /// event.
  pub fn event_sender() -> EventSender { EventSender(App::shared().event_loop_proxy.clone()) }

  /// Creating a new window using the `root` widget and the specified canvas.
  /// Note: This is exclusive to the web platform.
  #[cfg(target_family = "wasm")]
  pub async fn new_with_canvas(
    root: GenWidget, canvas: web_sys::HtmlCanvasElement, attrs: WindowAttributes,
  ) -> Sc<Window> {
    let event_loop = App::shared().event_loop.borrow();
    let event_loop = event_loop.as_ref().expect(
      " Event loop consumed. You can't create window after `App::exec` called in Web platform.",
    );
    let shell_wnd = WinitShellWnd::new_with_canvas(canvas, &event_loop, attrs).await;
    let wnd = AppCtx::new_window(Box::new(shell_wnd), root);
    wnd
  }

  /// create a new window with the `root` widget
  #[allow(clippy::await_holding_refcell_ref)]
  pub async fn new_window(root: GenWidget, attrs: WindowAttributes) -> Sc<Window> {
    let app = App::shared();
    let event_loop = app.event_loop.borrow();
    let event_loop = event_loop.as_ref().expect(
      " Event loop consumed. You can't create window after `App::exec` called in Web platform.",
    );
    let shell_wnd = WinitShellWnd::new(event_loop, attrs).await;
    let wnd = AppCtx::new_window(Box::new(shell_wnd), root);

    #[cfg(not(target_family = "wasm"))]
    if app.active_wnd.get().is_none() {
      app.active_wnd.set(Some(wnd.id()));
    }
    wnd
  }

  #[cfg(not(target_family = "wasm"))]
  pub fn active_window() -> Sc<Window> {
    App::shared()
      .active_wnd
      .get()
      .and_then(AppCtx::get_window)
      .expect("application at least have one window before use.")
  }

  /// set the window with `id` to be the active window, and the active window.
  #[track_caller]
  #[cfg(not(target_family = "wasm"))]
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
    #[cfg(not(target_family = "wasm"))]
    {
      use winit::platform::run_on_demand::EventLoopExtRunOnDemand;
      let mut event_loop = App::shared().event_loop.borrow_mut();
      let _ = event_loop
        .as_mut()
        .unwrap()
        .run_on_demand(App::event_loop_handle);
    }

    #[cfg(target_family = "wasm")]
    {
      use winit::platform::web::EventLoopExtWebSys;
      let mut event_loop = App::shared().event_loop.borrow_mut();
      let event_loop = event_loop.take().expect(
        "Event loop consumed. You can't exec the application after `App::exec` called in Web \
         platform.",
      );
      event_loop.spawn(App::event_loop_handle);
    }
  }

  #[track_caller]
  fn shared() -> &'static App {
    static APP: LazyLock<LocalSender<App>> = LazyLock::new(|| {
      let event_loop = EventLoopBuilder::with_user_event()
        .build()
        .unwrap();
      let waker = EventWaker(event_loop.create_proxy());

      #[cfg(not(target_family = "wasm"))]
      AppCtx::set_clipboard(Box::new(crate::clipboard::Clipboard::new().unwrap()));
      AppCtx::set_runtime_waker(Box::new(waker));

      register_platform_app_events_handlers();
      let app = App {
        event_loop_proxy: event_loop.create_proxy(),
        event_loop: RefCell::new(Some(event_loop)),
        events_stream: <_>::default(),
        #[cfg(not(target_family = "wasm"))]
        active_wnd: std::cell::Cell::new(None),
      };
      LocalSender::new(app)
    });
    &APP
  }
}

impl AppRunGuard {
  fn new(root: GenWidget) -> Self {
    static ONCE: std::sync::Once = std::sync::Once::new();
    assert!(!ONCE.is_completed(), "App::run can only be called once.");
    ONCE.call_once(|| {});

    Self { root: Some(root), wnd_attrs: Some(Default::default()), theme_initd: false }
  }

  /// Set the application theme, this will apply to whole application.
  pub fn with_app_theme(&mut self, theme: Theme) -> &mut Self {
    AppCtx::set_app_theme(theme);
    self.theme_initd = true;
    self
  }

  /// Sets the initial title of the window in the title bar.
  ///
  /// The default is `"Ribir App"`.
  pub fn with_title(&mut self, title: impl Into<String>) -> &mut Self {
    self.wnd_attr().title = title.into();
    self
  }

  /// Sets whether the window should be resizable or not. The default is `true`.
  pub fn with_resizable(&mut self, resizable: bool) -> &mut Self {
    self.wnd_attr().resizable = resizable;
    self
  }

  /// Sets the initial size of the window client area, window excluding the
  /// title bar and borders.
  pub fn with_size(&mut self, size: Size) -> &mut Self {
    self.wnd_attr().size = Some(size);
    self
  }

  /// Sets the minimum size of the window client area
  pub fn with_min_size(&mut self, size: Size) -> &mut Self {
    self.wnd_attr().min_size = Some(size);
    self
  }

  /// Sets the maximum size of the window client area
  pub fn with_max_size(&mut self, size: Size) -> &mut Self {
    self.wnd_attr().max_size = Some(size);
    self
  }

  /// Sets the initial position of the window in screen coordinates.
  pub fn with_position(&mut self, position: Point) -> &mut Self {
    self.wnd_attr().position = Some(position);
    self
  }

  /// Sets whether the window should be maximized when it is first shown.
  pub fn with_maximized(&mut self, maximized: bool) -> &mut Self {
    self.wnd_attr().maximized = maximized;
    self
  }

  /// Sets whether the window should be visible when it is first shown.
  pub fn with_visible(&mut self, visible: bool) -> &mut Self {
    self.wnd_attr().visible = visible;
    self
  }

  /// Sets whether the window should have a border, a title bar, etc.
  pub fn with_decorations(&mut self, decorations: bool) -> &mut Self {
    self.wnd_attr().decorations = decorations;
    self
  }

  /// Sets the icon of the window.
  pub fn with_icon(&mut self, icon: Resource<PixelImage>) -> &mut Self {
    self.wnd_attr().icon = Some(icon);
    self
  }

  fn wnd_attr(&mut self) -> &mut WindowAttributes {
    // Should be safe to unwrap because `wnd_attrs` is always `Some` before
    // drop.
    unsafe { self.wnd_attrs.as_mut().unwrap_unchecked() }
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
    let wnd = App::new_window(root, attr);

    #[cfg(target_family = "wasm")]
    wasm_bindgen_futures::spawn_local(async move {
      let _ = wnd.await;
      App::exec();
    });
    #[cfg(not(target_family = "wasm"))]
    {
      AppCtx::wait_future(wnd);
      App::exec();
    }
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

pub(crate) fn request_redraw(wnd: &Window) {
  let wnd = wnd.shell_wnd().borrow();
  let shell = wnd
    .as_any()
    .downcast_ref::<WinitShellWnd>()
    .unwrap();
  shell.winit_wnd.request_redraw();
}

impl WindowAttributes {
  /// Sets the initial title of the window in the title bar.
  ///
  /// The default is `"Ribir App"`.
  pub fn with_title(&mut self, title: impl Into<String>) -> &mut Self {
    self.title = title.into();
    self
  }

  /// Sets whether the window should be resizable or not. The default is `true`.
  pub fn with_resizable(&mut self, resizable: bool) -> &mut Self {
    self.resizable = resizable;
    self
  }

  /// Sets the initial size of the window client area, window excluding the
  /// title bar and borders.
  pub fn with_size(&mut self, size: Size) -> &mut Self {
    self.size = Some(size);
    self
  }

  /// Sets the minimum size of the window client area
  pub fn with_min_size(&mut self, size: Size) -> &mut Self {
    self.min_size = Some(size);
    self
  }

  /// Sets the maximum size of the window client area
  pub fn with_max_size(&mut self, size: Size) -> &mut Self {
    self.max_size = Some(size);
    self
  }

  /// Sets the initial position of the window in screen coordinates.
  pub fn with_position(&mut self, position: Point) -> &mut Self {
    self.position = Some(position);
    self
  }

  /// Sets whether the window should be maximized when it is first shown.
  pub fn with_maximized(&mut self, maximized: bool) -> &mut Self {
    self.maximized = maximized;
    self
  }

  /// Sets whether the window should be visible when it is first shown.
  pub fn with_visible(&mut self, visible: bool) -> &mut Self {
    self.visible = visible;
    self
  }

  /// Sets whether the window should have a border, a title bar, etc.
  pub fn with_decorations(&mut self, decorations: bool) -> &mut Self {
    self.decorations = decorations;
    self
  }

  /// Sets the icon of the window.
  pub fn with_icon(&mut self, icon: Resource<PixelImage>) -> &mut Self {
    self.icon = Some(icon);
    self
  }
}

impl Default for WindowAttributes {
  fn default() -> Self {
    Self {
      resizable: true,
      size: None,
      min_size: None,
      max_size: None,
      position: None,
      title: "Ribir App".to_string(),
      maximized: false,
      visible: true,
      decorations: true,
      icon: None,
    }
  }
}

#[cfg(test)]
mod tests {

  use ribir_core::{
    prelude::*,
    test_helper::{MockBox, TestWindow},
  };
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
    assert_eq!(&*log.read(), &[
      "on_ime_pre_edit_begin",
      "on_ime_pre_edit_update hello",
      "on_ime_pre_edit_end"
    ]);

    log.write().clear();
    App::process_winit_ime_event(&wnd, Ime::Preedit("hello".to_string(), None));
    App::process_winit_ime_event(&wnd, Ime::Commit("hello".to_string()));
    wnd.draw_frame();
    assert_eq!(&*log.read(), &[
      "on_ime_pre_edit_begin",
      "on_ime_pre_edit_update hello",
      "on_ime_pre_edit_end",
      "on_chars hello",
    ]);

    log.write().clear();
    App::process_winit_ime_event(&wnd, Ime::Preedit("hello".to_string(), None));
    wnd.force_exit_pre_edit();
    let device_id = unsafe { winit::event::DeviceId::dummy() };
    wnd.process_mouse_input(
      device_id,
      winit::event::ElementState::Pressed,
      winit::event::MouseButton::Left,
    );
    wnd.process_mouse_input(
      device_id,
      winit::event::ElementState::Released,
      winit::event::MouseButton::Left,
    );

    wnd.draw_frame();
    assert_eq!(&*log.read(), &[
      "on_ime_pre_edit_begin",
      "on_ime_pre_edit_update hello",
      "on_ime_pre_edit_end",
      "on_chars hello",
      "on_tap",
    ]);
  }
}
