use std::{
  cell::RefCell,
  sync::{LazyLock, Mutex, MutexGuard},
  task::{Context, RawWaker, RawWakerVTable, Waker},
};

pub use futures::task::SpawnError;
use futures::{Future, executor::LocalPool, task::LocalSpawnExt};
use pin_project_lite::pin_project;
use ribir_algo::Sc;
use ribir_painter::{TypographyStore, font_db::FontDB};
use rxrust::scheduler::NEW_TIMER_FN;

use crate::{
  builtin_widgets::Theme,
  clipboard::{Clipboard, MockClipboard},
  local_sender::LocalSender,
  prelude::FuturesLocalScheduler,
  state::{StateWriter, Stateful},
  timer::Timer,
  widget::GenWidget,
  window::{ShellWindow, Window, WindowId},
};

pub trait RuntimeWaker {
  fn clone_box(&self) -> Box<dyn RuntimeWaker + Send>;
  fn wake(&self);
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
  runtime_waker: RefCell<Box<dyn RuntimeWaker + Send>>,
  scheduler: FuturesLocalScheduler,
  executor: RefCell<LocalPool>,

  #[cfg(feature = "tokio-async")]
  tokio_runtime: tokio::runtime::Runtime,
}

#[allow(dead_code)]
pub struct AppCtxScopeGuard(MutexGuard<'static, ()>);

static APP_CTX: LazyLock<LocalSender<AppCtx>> = LazyLock::new(|| {
  let _ = NEW_TIMER_FN.set(Timer::new_timer_future);
  LocalSender::new(AppCtx::default())
});

impl AppCtx {
  /// Obtain the global application context. Please note that it is not
  /// thread-safe and should only be accessed in the initial thread that
  /// utilizes it.
  #[track_caller]
  pub fn shared() -> &'static Self { &APP_CTX }

  /// Get the theme of the application.
  #[track_caller]
  pub fn app_theme() -> &'static Stateful<Theme> { &Self::shared().app_theme }

  pub fn new_window(shell_wnd: Box<dyn ShellWindow>, content: GenWidget) -> Sc<Window> {
    let wnd = Window::new(shell_wnd);
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

  /// Get the scheduler of the application.
  #[track_caller]
  pub fn scheduler() -> FuturesLocalScheduler { Self::shared().scheduler.clone() }

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

  /// Runs all tasks in the local(usually means on the main thread) pool and
  /// returns if no more progress can be made on any task.
  #[track_caller]
  pub fn run_until_stalled() -> usize {
    let mut count = 0;
    let mut executor = Self::shared().executor.borrow_mut();
    while executor.try_run_one() {
      count += 1;
    }
    count
  }

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

  /// Set the runtime waker of the application, this should be called before
  /// application startup.
  ///
  /// # Safety
  /// This should be only called before application startup. The behavior is
  /// undefined if you call it in a running application.
  #[track_caller]
  pub fn set_runtime_waker(waker: Box<dyn RuntimeWaker + Send>) {
    *Self::shared().runtime_waker.borrow_mut() = waker;
  }

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
  /// If your application want create multi `AppCtx` instances, hold a scope for
  /// every instance. Otherwise, the behavior is undefined.
  #[track_caller]
  pub fn new_lock_scope() -> AppCtxScopeGuard {
    static LOCK: Mutex<()> = Mutex::new(());

    let locker = LOCK.lock().unwrap_or_else(|e| {
      // Only clear for test, so we have a clear error message.
      #[cfg(test)]
      LOCK.clear_poison();

      e.into_inner()
    });

    APP_CTX.reset();

    AppCtxScopeGuard(locker)
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
}

impl AppCtx {
  #[cfg(not(target_family = "wasm"))]
  pub fn wait_future<F: Future>(f: F) -> F::Output { futures::executor::block_on(f) }

  #[inline]
  pub fn spawn_local<Fut>(future: Fut) -> Result<(), SpawnError>
  where
    Fut: Future<Output = ()> + 'static,
  {
    let ctx = AppCtx::shared();
    ctx
      .scheduler
      .spawn_local(WakerFuture { fut: future, waker: ctx.runtime_waker.borrow().clone() })
  }
}

pin_project! {
  struct WakerFuture<F> {
    #[pin]
    fut: F,
    waker: Box<dyn RuntimeWaker + Send>,
  }
}

impl Clone for Box<dyn RuntimeWaker + Send> {
  fn clone(&self) -> Self { self.clone_box() }
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
      drop(this);
    }

    unsafe fn wake_by_ref(this: *const ()) {
      let waker = this as *mut RawLocalWaker;
      let (w, ribir_waker) = unsafe { &*waker };
      w.wake_by_ref();
      ribir_waker.wake();
    }

    unsafe fn drop(this: *const ()) {
      let waker = this as *mut RawLocalWaker;
      let _ = Box::from_raw(waker);
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

impl Drop for AppCtxScopeGuard {
  fn drop(&mut self) { APP_CTX.take(); }
}

#[cfg(feature = "tokio-async")]
pub mod tokio_async {
  use std::{cell::UnsafeCell, pin::Pin, task::Poll};

  use futures::{Future, FutureExt, Stream, StreamExt, future::RemoteHandle};
  use triomphe::Arc;

  impl AppCtx {
    pub fn tokio_runtime() -> &'static tokio::runtime::Runtime {
      let ctx = AppCtx::shared();
      &ctx.tokio_runtime
    }
  }

  use super::AppCtx;

  /// Compatible with Streams that depend on the tokio runtime.
  ///
  /// Stream dependent on the tokio runtime may not work properly when generated
  /// using in the ribir runtime(AppCtx::spawn_local()), you should call
  /// to_ribir_stream() to convert it.
  pub trait TokioToRibirStream
  where
    Self: Sized + Stream + Unpin + Send + 'static,
    Self::Item: Send,
  {
    fn to_ribir_stream(self) -> impl Stream<Item = Self::Item> {
      LocalWaitStream { stream: Arc::new(SyncUnsafeCell::new(self)), receiver: None }
    }
  }

  /// Compatible with futures that depend on the tokio runtime.
  ///
  /// future dependent on the tokio runtime may not work properly when generated
  /// using the ribir runtime (AppCtx::spawn_local()), you should call
  /// to_ribir_future() to convert it.
  pub trait TokioToRibirFuture
  where
    Self: Sized + Future + Send + 'static,
    Self::Output: Send,
  {
    fn to_ribir_future(self) -> impl Future<Output = <Self as Future>::Output> {
      async move {
        let (remote, handle) = self.remote_handle();
        AppCtx::tokio_runtime().spawn(remote);
        handle.await
      }
    }
  }

  impl<S> TokioToRibirStream for S
  where
    S: Stream + Unpin + Send + Sized + 'static,
    S::Item: Send,
  {
  }

  impl<F> TokioToRibirFuture for F
  where
    F: Future + Send + Sized + 'static,
    F::Output: Send,
  {
  }

  struct SyncUnsafeCell<T> {
    inner: UnsafeCell<T>,
  }

  unsafe impl<T> Sync for SyncUnsafeCell<T> {}

  impl<T> SyncUnsafeCell<T> {
    fn new(v: T) -> Self { Self { inner: UnsafeCell::new(v) } }
    fn get(&self) -> *mut T { self.inner.get() }
  }

  struct LocalWaitStream<S: Stream> {
    stream: Arc<SyncUnsafeCell<S>>,
    receiver: Option<RemoteHandle<Option<S::Item>>>,
  }

  impl<S: Stream> Stream for LocalWaitStream<S>
  where
    S: Stream + Unpin + Send + 'static,
    S::Item: Send,
  {
    type Item = S::Item;
    fn poll_next(
      self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
      let this = Pin::get_mut(self);
      if this.receiver.is_none() {
        let stream = this.stream.clone();
        let (remote, handle) = async move {
          let stream = unsafe { &mut *stream.get() };
          stream.next().await
        }
        .remote_handle();

        AppCtx::tokio_runtime().spawn(remote);
        this.receiver = Some(handle);
      }

      let item = this.receiver.as_mut().unwrap().poll_unpin(cx);
      if item.is_ready() {
        this.receiver = None;
      }
      item
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
      assert!(self.receiver.is_none());
      unsafe { &*self.stream.get() }.size_hint()
    }
  }
}

impl Default for AppCtx {
  fn default() -> Self {
    let app_theme = Stateful::new(Theme::default());

    let mut font_db = FontDB::default();
    font_db.load_system_fonts();

    let font_db = Sc::new(RefCell::new(font_db));
    let typography_store = RefCell::new(TypographyStore::new(font_db.clone()));

    let executor = LocalPool::new();
    let scheduler = executor.spawner();
    AppCtx {
      font_db,
      app_theme,
      typography_store,
      clipboard: RefCell::new(Box::new(MockClipboard {})),
      executor: RefCell::new(executor),
      scheduler,
      runtime_waker: RefCell::new(Box::new(MockWaker)),
      windows: RefCell::new(ahash::HashMap::default()),

      #[cfg(feature = "tokio-async")]
      tokio_runtime: tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap(),
    }
  }
}

#[cfg(test)]
mod tests {
  use std::task::Poll;

  use triomphe::Arc;

  use super::*;
  use crate::reset_test_env;

  #[derive(Default)]
  struct Trigger {
    ready: bool,
    waker: Option<Waker>,
  }

  impl Trigger {
    fn trigger(&mut self) {
      if self.ready {
        return;
      }
      self.ready = true;
      if let Some(waker) = self.waker.take() {
        waker.wake();
      }
    }

    fn pedding(&mut self, waker: &Waker) { self.waker = Some(waker.clone()) }
  }

  struct ManualFuture {
    trigger: Sc<RefCell<Trigger>>,
    cnt: usize,
  }

  impl Future for ManualFuture {
    type Output = usize;
    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
      if self.trigger.borrow().ready {
        Poll::Ready(self.cnt)
      } else {
        self.trigger.borrow_mut().pedding(cx.waker());
        Poll::Pending
      }
    }
  }

  #[test]
  fn local_future_smoke() {
    reset_test_env!();

    struct WakerCnt(Arc<Mutex<usize>>);
    impl RuntimeWaker for WakerCnt {
      fn wake(&self) { *self.0.lock().unwrap() += 1; }
      fn clone_box(&self) -> Box<dyn RuntimeWaker + Send> { Box::new(WakerCnt(self.0.clone())) }
    }

    let ctx_wake_cnt = Arc::new(Mutex::new(0));
    let wake_cnt = ctx_wake_cnt.clone();
    AppCtx::set_runtime_waker(Box::new(WakerCnt(wake_cnt)));

    let triggers = (0..3)
      .map(|_| Sc::new(RefCell::new(Trigger::default())))
      .collect::<Vec<_>>();
    let futs = triggers
      .clone()
      .into_iter()
      .map(|trigger| ManualFuture { trigger, cnt: 1 });

    let acc = Sc::new(RefCell::new(0));
    let sum = acc.clone();
    let _ = AppCtx::spawn_local(async move {
      for fut in futs {
        let v = fut.await;
        *acc.borrow_mut() += v;
      }
    });
    AppCtx::run_until_stalled();
    let mut waker_cnt = *ctx_wake_cnt.lock().unwrap();

    // when no trigger, nothing will change
    AppCtx::run_until_stalled();
    assert_eq!(*sum.borrow(), 0);
    assert_eq!(*ctx_wake_cnt.lock().unwrap(), waker_cnt);

    // once call trigger, the ctx.waker will be call once, and future step forward
    for (idx, trigger) in triggers.into_iter().enumerate() {
      trigger.borrow_mut().trigger();
      waker_cnt += 1;
      assert_eq!(*ctx_wake_cnt.lock().unwrap(), waker_cnt);
      AppCtx::run_until_stalled();
      assert_eq!(*sum.borrow(), idx + 1);
    }
  }

  #[cfg(feature = "tokio-async")]
  mod tokio_tests {
    use std::{
      sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
      },
      time::{Duration, Instant},
    };

    use tokio_stream::{StreamExt, wrappers::IntervalStream};

    use crate::{context::*, reset_test_env};

    #[derive(Default)]
    struct MockWaker {
      cnt: Arc<AtomicUsize>,
    }

    impl RuntimeWaker for MockWaker {
      fn wake(&self) {
        self
          .cnt
          .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
      }
      fn clone_box(&self) -> Box<dyn RuntimeWaker + Send> {
        Box::new(MockWaker { cnt: self.cnt.clone() })
      }
    }

    #[test]
    fn tokio_runtime() {
      reset_test_env!();
      let waker = MockWaker::default();
      AppCtx::set_runtime_waker(waker.clone_box());

      let _ = AppCtx::spawn_local(
        async {
          tokio::time::sleep(Duration::from_millis(1)).await;
        }
        .to_ribir_future(),
      );
      AppCtx::run_until_stalled();
      assert_eq!(waker.cnt.load(Ordering::Relaxed), 0);

      let finish = AtomicUsize::new(0);
      let mut start = Instant::now();
      AppCtx::wait_future(async {
        async {
          tokio::time::sleep(Duration::from_millis(100)).await;
        }
        .to_ribir_future()
        .await;
        finish.fetch_add(1, Ordering::SeqCst);
      });
      assert!(Instant::now().duration_since(start).as_millis() >= 100);
      assert_eq!(waker.cnt.load(Ordering::Relaxed), 1);

      start = Instant::now();
      AppCtx::wait_future(async {
        let interval = async { tokio::time::interval(Duration::from_millis(10)) }
          .to_ribir_future()
          .await;
        let mut stream = IntervalStream::new(interval).to_ribir_stream();

        stream.next().await;
        stream.next().await;
        stream.next().await;
        finish.fetch_add(1, Ordering::SeqCst);
      });

      assert!(Instant::now().duration_since(start).as_millis() >= 20);
      assert_eq!(finish.load(Ordering::Relaxed), 2);
    }
  }
}
