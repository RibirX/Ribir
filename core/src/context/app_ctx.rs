use std::{cell::RefCell, future::Future, pin::Pin, sync::LazyLock};

use ribir_algo::Rc;
use rxrust::LocalScheduler;
use tracing::warn;

#[cfg(not(target_arch = "wasm32"))]
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;
#[cfg(target_arch = "wasm32")]
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;
use ribir_painter::{TextServices, new_text_services};
use smallvec::SmallVec;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
#[cfg(not(target_arch = "wasm32"))]
use tokio::{runtime::Runtime, task::LocalSet};

use crate::{
  builtin_widgets::Theme,
  clipboard::{Clipboard, MockClipboard},
  event_loop::{CoreMsg, EventLoop},
  local_sender::LocalSender,
  prelude::Duration,
  state::{ModifyEffect, ModifyInfo, PartialId, StateWriter, Stateful, WriterInfo},
  widget::GenWidget,
  window::{BoxShell, UiEvent, Window, WindowAttributes, WindowFlags, WindowId},
};

/// Global context shared throughout the application.
///
/// # Thread Safety
///
/// `AppCtx` is not thread-safe and must only be used in the initial thread
/// where it was created. Any attempt to use it across threads may result
/// in panic.
///
/// - The context must be initialized by an application instance before use.
///
/// # Caveats
///
/// - ⚠️ Do not use before application startup completes
/// - ⚠️ Using uninitialized context may lead to undefined behavior
pub struct AppCtx {
  app_theme: Stateful<Theme>,
  windows: RefCell<ahash::HashMap<WindowId, Rc<Window>>>,
  text_services: Box<TextServices>,
  clipboard: RefCell<Box<dyn Clipboard>>,
  core_event_sender: RefCell<Option<UnboundedSender<CoreMsg>>>,
  shell: RefCell<Option<BoxShell>>,
  change_dataset: ChangeDataset,
  #[cfg(not(target_arch = "wasm32"))]
  pub(crate) local_set: RefCell<LocalSet>,
  #[cfg(all(not(target_arch = "wasm32"), feature = "test-utils"))]
  pub(crate) spawn_count: std::cell::Cell<usize>,
}

#[derive(Default)]
struct ChangeDataset(RefCell<ChangeDatasetInner>);

enum ChangeRecord {
  Modify { path: SmallVec<[PartialId; 1]>, info: Rc<WriterInfo> },
  Complete { notifier: crate::state::Notifier, finalize: Box<dyn FnOnce()> },
}

#[derive(Default)]
struct ChangeDatasetInner {
  records: Vec<ChangeRecord>,
  in_emit: bool,
}

impl ChangeDataset {
  fn has_pending_changes(&self) -> bool { !self.0.borrow().records.is_empty() }

  fn emit_change(&self) -> bool {
    let mut changed = false;
    if self.0.borrow().in_emit {
      return changed;
    }

    self.0.borrow_mut().in_emit = true;
    loop {
      let records = std::mem::take(&mut self.0.borrow_mut().records);
      if records.is_empty() {
        break;
      }
      changed = true;
      for record in records {
        match record {
          ChangeRecord::Modify { path, info } => {
            let effect = info
              .batched_modifies
              .replace(ModifyEffect::empty());
            info.notifier.next(ModifyInfo { effect, path });
          }
          ChangeRecord::Complete { mut notifier, finalize } => {
            notifier.complete();
            finalize();
          }
        }
      }
    }
    self.0.borrow_mut().in_emit = false;
    changed
  }

  fn add_record(&self, record: ChangeRecord) {
    self.0.borrow_mut().records.push(record);
    if !self.0.borrow().in_emit
      && self.0.borrow().records.len() == 1
      && !AppCtx::send_event(CoreMsg::DataChanged)
    {
      AppCtx::spawn_local(async move {
        AppCtx::shared().change_dataset.emit_change();
      });
    }
  }
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) static RUNTIME: LazyLock<Runtime> = LazyLock::new(|| {
  tokio::runtime::Builder::new_current_thread()
    .enable_all()
    .build()
    .expect("Failed building the Runtime")
});

static APP_CTX: LazyLock<LocalSender<AppCtx>> =
  LazyLock::new(|| LocalSender::new(AppCtx::default()));

impl AppCtx {
  /// Initialize the application context.
  /// should be called only once and before any other access to the context.
  pub(crate) fn init(core_event_sender: UnboundedSender<CoreMsg>, shell: BoxShell) {
    assert!(APP_CTX.core_event_sender.borrow().is_none());

    *APP_CTX.core_event_sender.borrow_mut() = Some(core_event_sender);
    *APP_CTX.shell.borrow_mut() = Some(shell);
  }

  /// Run the application logic in a separate thread. The AppCtx can be used in
  /// this same thread.
  pub fn run<F: Future + 'static + Send>(
    core_event_sender: UnboundedSender<CoreMsg>, core_events: UnboundedReceiver<CoreMsg>,
    shell: BoxShell, init: F,
  ) {
    #[cfg(not(target_arch = "wasm32"))]
    std::thread::spawn(move || {
      AppCtx::init(core_event_sender, shell);
      let event_loop = EventLoop::new(core_events);

      // Drive the app main future inside a LocalSet for the whole lifetime of
      // the AppCtx thread. This ensures `spawn_local` remains valid as long as
      // the main loop is running.
      let main_fut = async {
        #[cfg(feature = "debug")]
        {
          crate::debug_tool::start_debug_client();
        }

        init.await;
        event_loop.run().await;
      };
      let local_set = &*AppCtx::shared().local_set.borrow();
      RUNTIME.block_on(local_set.run_until(main_fut));
    });
    #[cfg(target_arch = "wasm32")]
    {
      AppCtx::init(core_event_sender, shell);
      let event_loop = EventLoop::new(core_events);

      AppCtx::spawn_local(async move {
        #[cfg(feature = "debug")]
        {
          crate::debug_tool::start_debug_client();
        }

        init.await;
        event_loop.run().await;
      });
    }
  }

  /// Obtain the global application context. Please note that it is not
  /// thread-safe and should only be accessed in the initial thread that
  /// utilizes it.
  #[track_caller]
  pub fn shared() -> &'static Self { &APP_CTX }

  pub fn exit() {
    let _ = AppCtx::send_event(CoreMsg::Exit);
    AppCtx::spawn_local(async move {
      AppCtx::shared()
        .core_event_sender
        .borrow_mut()
        .take();
    });
  }

  /// Get the theme of the application.
  #[track_caller]
  pub fn app_theme() -> &'static Stateful<Theme> { &Self::shared().app_theme }

  /// Scheduler accessor used by codepaths that pass an explicit scheduler to
  /// rxrust observables (e.g. `observable::{timer,timer_at,interval}`).
  #[inline]
  pub fn scheduler() -> LocalScheduler { LocalScheduler }

  pub async fn new_window(
    content: GenWidget, flags: WindowFlags, attrs: WindowAttributes,
  ) -> Rc<Window> {
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
    // After init the tree is dirty; request the platform to schedule the first
    // frame directly.  winit coalesces multiple request_redraw() calls
    // automatically, so no extra Scheduler-level bookkeeping is needed here.
    wnd
      .shell_wnd()
      .borrow()
      .request_draw(crate::window::RedrawDemand::Normal);

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
  pub fn get_window(id: WindowId) -> Option<Rc<Window>> {
    Self::shared().windows.borrow().get(&id).cloned()
  }

  /// Get the window by the window id. Same as `get_window` but will panic if
  /// the window not found.
  #[track_caller]
  #[inline]
  pub fn get_window_assert(id: WindowId) -> Rc<Window> {
    Self::get_window(id).expect("Window not found!")
  }

  /// Return the windows collection of the application.
  pub fn windows() -> &'static RefCell<ahash::HashMap<WindowId, Rc<Window>>> {
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

  #[track_caller]
  pub fn text_services() -> &'static TextServices { AppCtx::shared().text_services.as_ref() }

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
  pub(crate) fn end_frame() {}

  pub(crate) fn data_changed(path: SmallVec<[PartialId; 1]>, writer: Rc<WriterInfo>) {
    AppCtx::shared()
      .change_dataset
      .add_record(ChangeRecord::Modify { path, info: writer });
  }

  pub(crate) fn data_complete(notifier: crate::state::Notifier, finalize: Box<dyn FnOnce()>) {
    AppCtx::shared()
      .change_dataset
      .add_record(ChangeRecord::Complete { notifier, finalize });
  }

  pub(crate) fn has_pending_changes() -> bool {
    AppCtx::shared()
      .change_dataset
      .has_pending_changes()
  }

  pub(crate) fn emit_change() -> bool { AppCtx::shared().change_dataset.emit_change() }

  pub(crate) fn send_event(event: CoreMsg) -> bool {
    if let Some(event_sender) = AppCtx::shared()
      .core_event_sender
      .borrow()
      .as_ref()
    {
      event_sender.send(event).is_ok()
    } else {
      warn!("Core event sender not found, must call inner AppCtx::run().");
      false
    }
  }

  pub fn send_ui_event(event: UiEvent) -> bool { AppCtx::send_event(event.into()) }
}

impl AppCtx {
  #[cfg(not(target_arch = "wasm32"))]
  #[inline]
  pub fn spawn_local<Fut>(future: Fut)
  where
    Fut: Future<Output = ()> + 'static,
  {
    let local_set = AppCtx::shared().local_set.borrow();

    #[cfg(feature = "test-utils")]
    let future = {
      let count = &AppCtx::shared().spawn_count;
      count.set(count.get() + 1);
      async move {
        future.await;
        let count = &AppCtx::shared().spawn_count;
        count.set(count.get() - 1);
      }
    };

    local_set.spawn_local(future);
  }

  #[cfg(target_arch = "wasm32")]
  #[inline]
  pub fn spawn_local<Fut>(future: Fut)
  where
    Fut: Future<Output = ()> + 'static,
  {
    wasm_bindgen_futures::spawn_local(future);
  }

  #[cfg(not(target_arch = "wasm32"))]
  pub fn spawn<Fut>(future: Fut)
  where
    Fut: Future<Output = ()> + Send + 'static,
  {
    tokio::task::spawn(future);
  }

  #[cfg(target_arch = "wasm32")]
  pub fn spawn<Fut>(future: Fut)
  where
    Fut: Future<Output = ()> + 'static,
  {
    wasm_bindgen_futures::spawn_local(future);
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

  pub(crate) fn shell_mut() -> std::cell::RefMut<'static, Option<BoxShell>> {
    AppCtx::shared().shell.borrow_mut()
  }

  #[cfg(not(target_arch = "wasm32"))]
  pub fn enter() -> tokio::runtime::EnterGuard<'static> { RUNTIME.enter() }

  #[cfg(not(target_arch = "wasm32"))]
  pub fn timer(duration: Duration) -> BoxFuture<'static, ()> {
    Box::pin(tokio::time::sleep(duration))
  }

  #[cfg(target_arch = "wasm32")]
  pub fn timer(duration: Duration) -> BoxFuture<'static, ()> {
    Box::pin(gloo_timers::future::sleep(duration))
  }
}

impl Default for AppCtx {
  fn default() -> Self {
    let app_theme = Stateful::new(Theme::default());
    let text_services = new_text_services();

    AppCtx {
      text_services,
      app_theme,
      clipboard: RefCell::new(Box::new(MockClipboard {})),
      windows: RefCell::new(ahash::HashMap::default()),
      change_dataset: ChangeDataset::default(),
      core_event_sender: RefCell::new(None),
      shell: RefCell::new(None),
      #[cfg(not(target_arch = "wasm32"))]
      local_set: RefCell::new(LocalSet::new()),
      #[cfg(all(not(target_arch = "wasm32"), feature = "test-utils"))]
      spawn_count: std::cell::Cell::new(0),
    }
  }
}

#[cfg(all(feature = "test-utils", not(target_arch = "wasm32")))]
pub mod test_utils {
  use std::sync::{Mutex, MutexGuard};

  use ribir_algo::Rc;
  use tokio::{runtime::EnterGuard, sync::mpsc::unbounded_channel, task::LocalSet};

  use crate::{
    context::{AppCtx, app_ctx::APP_CTX},
    event_loop::EventLoop,
    test_helper::{TestShell, TestWindow},
    window::{UiEvent, Window},
  };

  pub struct AppCtxScopeGuard {
    _guard: (Option<TestRuntimeGuard>, MutexGuard<'static, ()>),
  }

  impl Drop for AppCtxScopeGuard {
    fn drop(&mut self) {
      // Drop TestRuntimeGuard first (runs cleanup tasks)
      self._guard.0.take();

      // Clean up scheduler resources before releasing the lock.
      // This ensures LocalSet is dropped in the same thread that created it.
      AppCtx::clear_scheduler();

      // MutexGuard will be dropped automatically, releasing the lock
    }
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
      AppCtx::reset_scheduler();
      let guard = AppCtx::reset_test_env();

      AppCtxScopeGuard { _guard: (Some(guard), locker) }
    }
  }

  pub struct TestRuntimeGuard {
    _runtime_guard: EnterGuard<'static>,
    _local_enter_guard: tokio::task::LocalEnterGuard,
  }

  impl Drop for TestRuntimeGuard {
    fn drop(&mut self) {
      AppCtx::run_until_stalled();
      for wnd_id in AppCtx::windows().borrow().keys() {
        let _ = AppCtx::send_ui_event(UiEvent::CloseRequest { wnd_id: *wnd_id });
      }
      AppCtx::run_until_stalled();
      AppCtx::shared()
        .core_event_sender
        .borrow_mut()
        .take();
      AppCtx::run_until_stalled();
    }
  }

  impl AppCtx {
    pub fn new_test_frame(wnd: &TestWindow) {
      wnd.run_frame_tasks();
      AppCtx::run_until_stalled();
      assert!(
        AppCtx::send_ui_event(UiEvent::RedrawRequest {
          wnd_id: wnd.id(),
          demand: crate::window::RedrawDemand::Normal,
        }),
        "failed to queue redraw request for test window",
      );

      // Continuously run the event loop until the frame is fully processed.
      // Complex widgets may require multiple poll cycles to complete their layout.
      let ctx = AppCtx::shared();
      let local_set = &*ctx.local_set.borrow();
      for _ in 0..1000 {
        wnd.run_frame_tasks();
        super::RUNTIME.block_on(local_set.run_until(tokio::task::yield_now()));

        // Check if the tree is no longer dirty, indicating the frame is complete
        if !wnd.tree().is_dirty() {
          break;
        }
      }
      AppCtx::run_until_stalled();
    }

    pub fn run_until_stalled() {
      let ctx = AppCtx::shared();
      let local_set = &*ctx.local_set.borrow();
      for _ in 0..10 {
        if ctx.spawn_count.get() == 0 {
          break;
        }
        super::RUNTIME.block_on(local_set.run_until(tokio::task::yield_now()));
      }
    }

    pub fn run_until<F: std::future::Future>(fut: F) -> F::Output {
      let ctx = AppCtx::shared();
      let local_set = &*ctx.local_set.borrow();
      super::RUNTIME.block_on(local_set.run_until(fut))
    }

    fn reset_scheduler() {
      let old = Self::clear_scheduler();
      std::mem::forget(old);
    }

    fn clear_scheduler() -> LocalSet {
      let ctx = AppCtx::shared();
      ctx.spawn_count.set(0);
      std::mem::replace(&mut *ctx.local_set.borrow_mut(), LocalSet::new())
    }

    pub fn reset_test_env() -> TestRuntimeGuard {
      let (sender, receiver) = unbounded_channel();
      let old_sender =
        Option::replace(&mut *AppCtx::shared().core_event_sender.borrow_mut(), sender);
      assert!(old_sender.is_none(), "reset_test_env expects AppCtx::core_event_sender to be empty",);

      *AppCtx::shared().shell.borrow_mut() = Some(Box::new(TestShell {}));

      // Enter the LocalSet context so that tokio::task::spawn_local (used by rxRust)
      // can spawn tasks. We leak the LocalSet reference to get a 'static lifetime
      // for the enter guard. This is safe because reset_scheduler already uses
      // mem::forget for the old LocalSet, so we're already accepting this leak in
      // test.
      let _local_enter_guard = {
        let local_set = AppCtx::shared().local_set.borrow();
        // SAFETY: We leak the reference to create a 'static lifetime. The LocalSet
        // will be properly forgotten in reset_scheduler when a new test starts.
        let local_set_ref: &'static LocalSet = unsafe { &*(&*local_set as *const LocalSet) };
        local_set_ref.enter()
      };

      AppCtx::spawn_local(async move {
        let event_loop = EventLoop::new(receiver);
        event_loop.run().await;
      });
      let _runtime_guard = AppCtx::enter();
      TestRuntimeGuard { _runtime_guard, _local_enter_guard }
    }

    pub fn insert_window(wnd: Rc<Window>) {
      AppCtx::windows()
        .borrow_mut()
        .insert(wnd.id(), wnd);
    }
  }
}
