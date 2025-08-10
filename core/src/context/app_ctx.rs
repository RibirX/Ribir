use std::{
  cell::RefCell,
  future::Future,
  ops::DerefMut,
  sync::LazyLock,
  task::{Context, RawWaker, RawWakerVTable, Waker},
};

use log::warn;
use pin_project_lite::pin_project;
use ribir_algo::Sc;
use ribir_painter::{TypographyStore, font_db::FontDB};
use rxrust::prelude::{AsyncExecutor, NEW_TIMER_FN};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

use crate::{
  builtin_widgets::Theme,
  clipboard::{Clipboard, MockClipboard},
  event_loop::{EventLoop, FrameworkEvent},
  local_sender::LocalSender,
  scheduler::{RibirScheduler, RuntimeWaker},
  state::{ModifyEffect, ModifyInfo, PartialPath, StateWriter, Stateful, WriterInfo},
  widget::GenWidget,
  window::{BoxShell, UiEvent, Window, WindowAttributes, WindowFlags, WindowId},
};

#[derive(Clone)]
pub struct RibirSchedulerRunner {}

impl<T> AsyncExecutor<T> for RibirSchedulerRunner
where
  T: Future<Output = ()> + 'static,
{
  fn spawn(&self, f: T) { AppCtx::spawn_local(f); }
}

/// Global context shared throughout the application.
///
/// # Thread Safety
///
/// `AppCtx` is not thread-safe and must only be used in the initial thread
/// where it was created. Any attempt to use it across threads may result
/// in panic.
///
/// # Initialization Requirements
///
/// - The context must be initialized by an application instance before use
/// - Contains runtime-dependent components including:
///   - Future wakers
///   - Clipboard handlers
///   - Other platform-specific subsystems
///
/// # Caveats
///
/// - ⚠️ Do not use before application startup completes
/// - ⚠️ Using uninitialized context may lead to undefined behavior
pub struct AppCtx {
  app_theme: Stateful<Theme>,
  windows: RefCell<ahash::HashMap<WindowId, Sc<Window>>>,
  font_db: Sc<RefCell<FontDB>>,
  typography_store: RefCell<TypographyStore>,
  clipboard: RefCell<Box<dyn Clipboard>>,
  event_sender: RefCell<Option<UnboundedSender<FrameworkEvent>>>,
  shell: RefCell<Option<BoxShell>>,
  change_dataset: ChangeDataset,
}

#[derive(Default)]
struct ChangeDataset(RefCell<ChangeDatasetInner>);

#[derive(Default)]
struct ChangeDatasetInner {
  dirty_info: Vec<(PartialPath, Sc<WriterInfo>)>,
  in_emit: bool,
}

impl ChangeDataset {
  fn emit_change(&self) -> bool {
    let mut changed = false;
    if self.0.borrow().in_emit {
      return changed;
    }

    self.0.borrow_mut().in_emit = true;
    loop {
      let writers = std::mem::take(&mut self.0.borrow_mut().dirty_info);
      if writers.is_empty() {
        break;
      }
      changed = true;
      for (path, info) in writers {
        let effect = info
          .batched_modifies
          .replace(ModifyEffect::empty());
        info.notifier.next(ModifyInfo { effect, path });
      }
    }
    self.0.borrow_mut().in_emit = false;
    changed
  }

  fn add_changed(&self, dirty_info: (PartialPath, Sc<WriterInfo>)) {
    self.0.borrow_mut().dirty_info.push(dirty_info);
    if !self.0.borrow().in_emit
      && self.0.borrow().dirty_info.len() == 1
      && !AppCtx::send_event(FrameworkEvent::DataChanged)
    {
      AppCtx::spawn_local(async move {
        AppCtx::shared().change_dataset.emit_change();
      });
    }
  }
}

static APP_CTX: LazyLock<LocalSender<AppCtx>> = LazyLock::new(|| {
  let _ = NEW_TIMER_FN.set(RibirScheduler::timer);
  LocalSender::new(AppCtx::default())
});

impl AppCtx {
  /// Initialize the application context.
  /// should be called only once and before any other access to the context.
  pub(crate) fn init(shell: BoxShell) -> EventLoop {
    assert!(APP_CTX.event_sender.borrow().is_none());
    let _ = NEW_TIMER_FN.set(RibirScheduler::timer);
    let (sender, receiver) = unbounded_channel();
    *APP_CTX.event_sender.borrow_mut() = Some(sender);
    *APP_CTX.shell.borrow_mut() = Some(shell);
    EventLoop::new(receiver)
  }

  /// Run the application logic in a separate thread. The AppCtx can be used in
  /// this same thread.
  pub fn run<F: Future + 'static + Send>(
    ui_events: UnboundedReceiver<UiEvent>, shell: BoxShell, init: F,
  ) {
    #[cfg(not(target_arch = "wasm32"))]
    std::thread::spawn(move || {
      use crate::scheduler::RibirScheduler;
      let event_loop = AppCtx::init(shell);
      RibirScheduler::spawn_local(async {
        init.await;
        event_loop.run(ui_events).await
      });
      RibirScheduler::run();
    });
    #[cfg(target_arch = "wasm32")]
    {
      let event_loop = AppCtx::init(shell);
      AppCtx::spawn_local(async move {
        init.await;
        event_loop.run(ui_events).await
      });
    }
  }

  /// Obtain the global application context. Please note that it is not
  /// thread-safe and should only be accessed in the initial thread that
  /// utilizes it.
  #[track_caller]
  pub fn shared() -> &'static Self { &APP_CTX }

  pub fn exit() {
    AppCtx::spawn_local(async move {
      AppCtx::shared().event_sender.borrow_mut().take();
    });
  }

  /// Get the theme of the application.
  #[track_caller]
  pub fn app_theme() -> &'static Stateful<Theme> { &Self::shared().app_theme }

  pub fn scheduler() -> RibirSchedulerRunner { RibirSchedulerRunner {} }

  pub async fn new_window(
    content: GenWidget, flags: WindowFlags, attrs: WindowAttributes,
  ) -> Sc<Window> {
    let fut = Self::shared()
      .shell
      .borrow()
      .as_ref()
      .unwrap()
      .new_shell_window(attrs);
    let shell_wnd = fut.await;
    let wnd = Window::new(shell_wnd, flags);
    let id = wnd.id();

    Self::shared()
      .windows
      .borrow_mut()
      .insert(id, wnd.clone());

    wnd.init(content);

    wnd
  }

  /// Get the window by the window id. Return an count reference of the window.
  ///
  /// If you want store the `Window`, you'd better store the `WindowId` instead.
  /// Because `Window` owns so many resources, and it's easy to cause a circular
  /// reference if you store it in another struct with count reference that
  /// belongs to `Window`.
  #[track_caller]
  #[inline]
  pub fn get_window(id: WindowId) -> Option<Sc<Window>> {
    Self::shared().windows.borrow().get(&id).cloned()
  }

  /// Get the window by the window id. Same as `get_window` but will panic if
  /// the window not found.
  #[track_caller]
  #[inline]
  pub fn get_window_assert(id: WindowId) -> Sc<Window> {
    Self::get_window(id).expect("Window not found!")
  }

  /// Return the windows collection of the application.
  pub fn windows() -> &'static RefCell<ahash::HashMap<WindowId, Sc<Window>>> {
    &Self::shared().windows
  }

  /// Returns the number of windows.
  #[track_caller]
  #[inline]
  pub fn wnd_cnt() -> usize { Self::shared().windows.borrow().len() }

  /// Returns true if there is any window in the application.
  #[track_caller]
  #[inline]
  pub fn has_wnd() -> bool { !Self::shared().windows.borrow().is_empty() }

  /// Remove the window by the window id.
  #[track_caller]
  pub fn remove_wnd(id: WindowId) { Self::shared().windows.borrow_mut().remove(&id); }

  /// Get the clipboard of the application.
  #[track_caller]
  pub fn clipboard() -> &'static RefCell<Box<dyn Clipboard>> { &Self::shared().clipboard }

  /// Get the typography store of the application.
  #[track_caller]
  pub fn typography_store() -> &'static RefCell<TypographyStore> {
    &Self::shared().typography_store
  }

  /// Get the font database of the application.
  #[track_caller]
  pub fn font_db() -> &'static Sc<RefCell<FontDB>> { &Self::shared().font_db }

  /// Set the theme of the application
  ///
  /// # Safety
  /// This should be only called before application startup. The behavior is
  /// undefined if you call it in a running application.
  #[track_caller]
  pub fn set_app_theme(theme: Theme) { *Self::shared().app_theme.write() = theme; }

  /// Set the shared clipboard of the application, this should be called before
  /// application startup.
  ///
  /// # Safety
  /// This should be only called before application startup. The behavior is
  /// undefined if you call it in a running application.
  #[track_caller]
  pub fn set_clipboard(clipboard: Box<dyn Clipboard>) {
    *Self::shared().clipboard.borrow_mut() = clipboard;
  }

  #[track_caller]
  pub(crate) fn end_frame() {
    // todo: frame cache is not a good algorithm? because not every text will
    // relayout in every frame.
    Self::shared()
      .typography_store
      .borrow_mut()
      .end_frame();
  }

  pub(crate) fn data_changed(path: PartialPath, writer: Sc<WriterInfo>) {
    AppCtx::shared()
      .change_dataset
      .add_changed((path, writer));
  }

  pub(crate) fn emit_change() -> bool { AppCtx::shared().change_dataset.emit_change() }

  pub(crate) fn send_event(event: FrameworkEvent) -> bool {
    if let Some(event_sender) = AppCtx::shared().event_sender.borrow().as_ref() {
      event_sender.send(event).is_ok()
    } else {
      warn!("Event sender not found, must call inner AppCtx::run().");
      false
    }
  }
}

impl AppCtx {
  #[cfg(not(target_arch = "wasm32"))]
  pub fn wait_future<F: Future>(f: F) -> F::Output { RibirScheduler::run_until(f) }

  #[inline]
  pub fn spawn_local<Fut>(future: Fut)
  where
    Fut: Future<Output = ()> + 'static,
  {
    RibirScheduler::spawn_local(future);
  }

  #[cfg(not(target_arch = "wasm32"))]
  pub fn run_until_stalled() {
    let _ = AppCtx::shared(); // check thread
    RibirScheduler::run_until_stalled();
  }

  #[cfg(not(target_arch = "wasm32"))]
  pub fn spawn<Fut>(future: Fut) -> tokio::task::JoinHandle<Fut::Output>
  where
    Fut: Future + 'static + Send,
    Fut::Output: Send,
  {
    RibirScheduler::spawn(future)
  }

  #[inline]
  pub fn spawn_in_ui<Fut>(future: Fut) -> tokio::sync::oneshot::Receiver<Fut::Output>
  where
    Fut: Future + 'static + Send,
    Fut::Output: Send + 'static,
  {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    AppCtx::shared()
      .shell
      .borrow()
      .as_ref()
      .unwrap()
      .run_in_shell(Box::pin(async move {
        let res = future.await;
        let _ = sender.send(res);
      }));
    receiver
  }

  pub(crate) fn shell_mut() -> impl DerefMut<Target = Option<BoxShell>> {
    AppCtx::shared().shell.borrow_mut()
  }
}

pin_project! {
  struct WakerFuture<F> {
    #[pin]
    fut: F,
    waker: Box<dyn RuntimeWaker + Send>,
  }
}

impl<F> WakerFuture<F>
where
  F: Future,
{
  fn local_waker(&self, cx: &std::task::Context<'_>) -> Waker {
    type RawLocalWaker = (std::task::Waker, Box<dyn RuntimeWaker + Send>);
    fn clone(this: *const ()) -> RawWaker {
      let waker = this as *const RawLocalWaker;
      let (w, cb) = unsafe { &*waker };
      let data = Box::new((w.clone(), cb.clone()));
      let raw = Box::leak(data) as *const RawLocalWaker;
      RawWaker::new(raw as *const (), &VTABLE)
    }

    unsafe fn wake(this: *const ()) {
      let waker = this as *mut RawLocalWaker;
      let (w, ribir_waker) = unsafe { &*waker };
      w.wake_by_ref();
      ribir_waker.wake();
      unsafe{drop(this);}
    }

    unsafe fn wake_by_ref(this: *const ()) {
      let waker = this as *mut RawLocalWaker;
      let (w, ribir_waker) = unsafe { &*waker };
      w.wake_by_ref();
      ribir_waker.wake();
    }

    unsafe fn drop(this: *const ()) {
      let waker = this as *mut RawLocalWaker;
      let _ = unsafe { Box::from_raw(waker) };
    }
    static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, wake, wake_by_ref, drop);

    let old_waker = cx.waker().clone();
    let data = Box::new((old_waker, self.waker.clone()));
    let raw = RawWaker::new(Box::leak(data) as *const RawLocalWaker as *const (), &VTABLE);
    unsafe { Waker::from_raw(raw) }
  }
}

impl<F> Future for WakerFuture<F>
where
  F: Future,
{
  type Output = F::Output;
  fn poll(
    self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>,
  ) -> std::task::Poll<Self::Output> {
    let waker = self.local_waker(cx);
    let mut cx = Context::from_waker(&waker);
    let this = self.project();
    this.fut.poll(&mut cx)
  }
}

#[derive(Clone, Copy, Default)]
pub struct MockWaker;
impl RuntimeWaker for MockWaker {
  fn wake(&self) {}
  fn clone_box(&self) -> Box<dyn RuntimeWaker + Send> { Box::new(MockWaker) }
}

impl Default for AppCtx {
  fn default() -> Self {
    let app_theme = Stateful::new(Theme::default());

    let mut font_db = FontDB::default();
    font_db.load_system_fonts();

    let font_db = Sc::new(RefCell::new(font_db));
    let typography_store = RefCell::new(TypographyStore::new(font_db.clone()));

    AppCtx {
      font_db,
      app_theme,
      typography_store,
      clipboard: RefCell::new(Box::new(MockClipboard {})),

      windows: RefCell::new(ahash::HashMap::default()),
      change_dataset: ChangeDataset::default(),
      event_sender: RefCell::new(None),
      shell: RefCell::new(None),
    }
  }
}

#[cfg(all(feature = "test-utils", not(target_arch = "wasm32")))]
pub mod test_utils {
  use std::{
    cell::RefCell,
    sync::{Mutex, MutexGuard},
  };

  use ribir_algo::Sc;
  use tokio::{
    runtime::EnterGuard,
    sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel},
  };

  use crate::{
    context::{AppCtx, app_ctx::APP_CTX},
    event_loop::{EventLoop, FrameworkEvent},
    scheduler::RibirScheduler,
    test_helper::{TestShell, TestWindow},
    window::{UiEvent, Window},
  };

  pub struct AppCtxScopeGuard {
    _guard: (Option<TestRuntimeGuard>, MutexGuard<'static, ()>),
  }

  impl Drop for AppCtxScopeGuard {
    fn drop(&mut self) { self._guard.0.take(); }
  }

  impl AppCtx {
    /// Start a new scope to mock a new application startup for `AppCtx`, this
    /// will force reset the application context and return a lock guard. The
    /// lock guard prevents two scope have intersecting lifetime.
    ///
    /// In normal case, you should not directly call this method in your
    /// production code, use only if you have same requirement and know how
    /// `new_lock_scope` works.
    ///
    /// It's useful for unit test and server side rendering. Because many tests
    /// are runnings parallels in one process, we call this method before each
    /// test that uses `AppCtx` to ensure every test has a clean `AppCtx`. For
    /// server-side it's can help to reuse the process resource.
    ///
    /// # Safety
    /// If your application want create multi `AppCtx` instances, hold a scope
    /// for every instance. Otherwise, the behavior is undefined.
    #[track_caller]
    pub fn new_lock_scope() -> AppCtxScopeGuard {
      static LOCK: Mutex<()> = Mutex::new(());

      let locker = LOCK.lock().unwrap_or_else(|e| {
        println!("lock error: {e}");
        // Only clear for test, so we have a clear error message.
        #[cfg(test)]
        LOCK.clear_poison();

        e.into_inner()
      });

      APP_CTX.reset();
      let guard = AppCtx::reset_test_env();

      AppCtxScopeGuard { _guard: (Some(guard), locker) }
    }
  }

  thread_local! {
    static RUNTIME_EVENTS: RefCell<Option<UnboundedReceiver<FrameworkEvent>>> =
      const { RefCell::new(None) };
  }

  pub struct TestRuntimeGuard {
    _sender: UnboundedSender<UiEvent>,
    old_sender: Option<UnboundedSender<FrameworkEvent>>,
    _runtime_guard: EnterGuard<'static>,
  }

  impl Drop for TestRuntimeGuard {
    fn drop(&mut self) {
      AppCtx::run_until_stalled();
      for wnd_id in AppCtx::windows().borrow().keys() {
        let _ = self
          ._sender
          .send(UiEvent::CloseRequest { wnd_id: *wnd_id });
      }
      AppCtx::run_until_stalled();
      AppCtx::shared().event_sender.borrow_mut().take();
      RibirScheduler::run();
      std::mem::swap(&mut *AppCtx::shared().event_sender.borrow_mut(), &mut self.old_sender);
    }
  }

  impl AppCtx {
    pub fn new_test_frame(wnd: &TestWindow) {
      wnd.run_frame_tasks();
      RibirScheduler::run_until_stalled();
      AppCtx::send_event(FrameworkEvent::NewFrame { wnd_id: wnd.id(), force_redraw: false });
      wnd.run_frame_tasks();
      RibirScheduler::run_until_stalled();
    }

    pub fn reset_test_env() -> TestRuntimeGuard {
      let (sender, receiver) = unbounded_channel();
      let (ui_sender, ui_receiver) = unbounded_channel();
      let old_sender = Option::replace(&mut *AppCtx::shared().event_sender.borrow_mut(), sender);
      *AppCtx::shared().shell.borrow_mut() = Some(Box::new(TestShell {}));
      RibirScheduler::spawn_local(async move {
        let event_loop = EventLoop::new(receiver);
        event_loop.run(ui_receiver).await;
      });
      let _runtime_guard = RibirScheduler::enter();
      TestRuntimeGuard { _sender: ui_sender, old_sender, _runtime_guard }
    }

    pub fn insert_window(wnd: Sc<Window>) {
      AppCtx::windows()
        .borrow_mut()
        .insert(wnd.id(), wnd);
    }
  }
}
