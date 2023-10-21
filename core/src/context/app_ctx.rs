use crate::{
  builtin_widgets::{FullTheme, InheritTheme, Theme},
  clipboard::{Clipboard, MockClipboard},
  widget::WidgetBuilder,
  window::{ShellWindow, Window, WindowId},
};
use pin_project_lite::pin_project;
use std::{
  cell::RefCell,
  rc::Rc,
  sync::{Mutex, MutexGuard, Once},
  task::{Context, RawWaker, RawWakerVTable, Waker},
  thread::ThreadId,
};

use crate::prelude::FuturesLocalScheduler;
pub use futures::task::SpawnError;
use futures::{
  executor::{block_on, LocalPool},
  task::LocalSpawnExt,
  Future,
};
use ribir_text::shaper::TextShaper;
use ribir_text::{font_db::FontDB, TextReorder, TypographyStore};

pub trait RuntimeWaker {
  fn clone_box(&self) -> Box<dyn RuntimeWaker + Send>;
  fn wake(&self);
}

/// The context is shared throughout the application, "AppCtx" is not
/// thread-safe, and only allowed to be used in the first thread that uses
/// "AppCtx".
///
/// All mutable methods of "AppCtx" are unsafe, and should call the mutable
/// methods before application startup, all the mutable calls during the
/// application running is dangerous. Because the reference of "AppCtx" maybe
/// already hold by others.
pub struct AppCtx {
  app_theme: Theme,
  windows: RefCell<ahash::HashMap<WindowId, Rc<Window>>>,
  font_db: Rc<RefCell<FontDB>>,
  shaper: TextShaper,
  reorder: TextReorder,
  typography_store: TypographyStore,
  clipboard: RefCell<Box<dyn Clipboard>>,
  runtime_waker: Box<dyn RuntimeWaker + Send>,
  scheduler: FuturesLocalScheduler,
  executor: RefCell<LocalPool>,
  triggers: TriggerMap,
}

type TriggerMap = RefCell<ahash::HashMap<*const (), Box<dyn FnOnce()>>>;
pub struct AppCtxScopeGuard(MutexGuard<'static, ()>);

static mut INIT_THREAD_ID: Option<ThreadId> = None;
static mut APP_CTX_INIT: Once = Once::new();
static mut APP_CTX: Option<AppCtx> = None;

impl AppCtx {
  /// Get the global application context, it's not thread safe, you can only use
  /// it in the first thread that uses it.
  #[track_caller]
  pub fn shared() -> &'static Self { unsafe { Self::shared_mut() } }

  /// Get the theme of the application.
  #[track_caller]
  pub fn app_theme() -> &'static Theme { &Self::shared().app_theme }

  pub fn new_window(shell_wnd: Box<dyn ShellWindow>, content: impl WidgetBuilder) -> Rc<Window> {
    let wnd = Window::new(shell_wnd);
    let id = wnd.id();

    Self::shared().windows.borrow_mut().insert(id, wnd.clone());
    wnd.set_content_widget(content);

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

  /// Get the scheduler of the application.
  #[track_caller]
  pub fn scheduler() -> FuturesLocalScheduler { Self::shared().scheduler.clone() }

  /// Get the clipboard of the application.
  #[track_caller]
  pub fn clipboard() -> &'static RefCell<Box<dyn Clipboard>> { &Self::shared().clipboard }

  /// Get the typography store of the application.
  #[track_caller]
  pub fn typography_store() -> &'static TypographyStore { &Self::shared().typography_store }

  /// Get the font database of the application.
  #[track_caller]
  pub fn font_db() -> &'static Rc<RefCell<FontDB>> { &Self::shared().font_db }

  /// Add a trigger task to the application, this task use an pointer address as
  /// its identity. You can use the identity to trigger this task in a
  /// deterministic time by calling `AppCtx::trigger_task`.
  pub fn add_trigger_task(trigger: *const (), task: Box<dyn FnOnce()>) {
    let mut tasks = AppCtx::shared().triggers.borrow_mut();
    let task: Box<dyn FnOnce()> = if let Some(t) = tasks.remove(&trigger) {
      Box::new(move || {
        t();
        task();
      })
    } else {
      Box::new(task)
    };
    tasks.insert(trigger, Box::new(task));
  }

  /// Trigger the task by the pointer address identity. Returns true if the task
  /// is found.
  pub fn trigger_task(trigger: *const ()) -> bool {
    let task = Self::shared().triggers.borrow_mut().remove(&trigger);
    if let Some(task) = task {
      task();
      true
    } else {
      false
    }
  }

  /// Runs all tasks in the local(usually means on the main thread) pool and
  /// returns if no more progress can be made on any task.
  #[track_caller]
  pub fn run_until_stalled() {
    let mut executor = Self::shared().executor.borrow_mut();
    while executor.try_run_one() {}
  }

  /// Loads the font from the theme config and import it into the font database.
  #[track_caller]
  pub fn load_font_from_theme(theme: &Theme) {
    let mut font_db = Self::shared().font_db.borrow_mut();
    load_font_from_theme(theme, &mut font_db);
  }

  /// Check if the calling thread is the thread that initializes the `AppCtx`,
  /// you needn't use this method manually, it's called automatically when you
  /// use the methods of `AppCtx`. But it's useful when you want your code to
  /// keep same behavior like `AppCtx`.
  #[track_caller]
  pub fn thread_check() {
    let current_thread = std::thread::current().id();
    unsafe {
      if Some(current_thread) != INIT_THREAD_ID {
        panic!(
          "AppCtx::shared() should be called only in one thread {:?} != {:?}.",
          Some(current_thread),
          INIT_THREAD_ID
        );
      }
    }
  }

  /// Set the theme of the application
  ///
  /// # Safety
  /// This should be only called before application startup. The behavior is
  /// undefined if you call it in a running application.
  #[track_caller]
  pub unsafe fn set_app_theme(theme: FullTheme) {
    Self::shared_mut().app_theme = Theme::Full(theme);
    load_font_from_theme(Self::app_theme(), &mut Self::font_db().borrow_mut());
  }

  /// Set the shared clipboard of the application, this should be called before
  /// application startup.
  ///
  /// # Safety
  /// This should be only called before application startup. The behavior is
  /// undefined if you call it in a running application.
  #[track_caller]
  pub unsafe fn set_clipboard(clipboard: Box<dyn Clipboard>) {
    Self::shared_mut().clipboard = RefCell::new(clipboard);
  }

  /// Set the runtime waker of the application, this should be called before
  /// application startup.
  /// # Safety
  /// This should be only called before application startup. The behavior is
  /// undefined if you call it in a running application.
  #[track_caller]
  pub unsafe fn set_runtime_waker(waker: Box<dyn RuntimeWaker + Send>) {
    Self::shared_mut().runtime_waker = waker;
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
  pub unsafe fn new_lock_scope() -> AppCtxScopeGuard {
    static LOCK: Mutex<()> = Mutex::new(());

    let locker = LOCK.lock().unwrap_or_else(|e| {
      // Only clear for test, so we have a clear error message.
      #[cfg(test)]
      LOCK.clear_poison();

      e.into_inner()
    });

    AppCtxScopeGuard(locker)
  }

  #[track_caller]
  pub(crate) fn end_frame() {
    // todo: frame cache is not a good algorithm? because not every text will
    // relayout in every frame.
    let ctx = unsafe { Self::shared_mut() };
    ctx.shaper.end_frame();
    ctx.reorder.end_frame();
    ctx.typography_store.end_frame();
  }

  #[track_caller]
  unsafe fn shared_mut() -> &'static mut Self {
    APP_CTX_INIT.call_once(|| {
      let app_theme = Theme::Full(<_>::default());
      let mut font_db = FontDB::default();
      font_db.load_system_fonts();
      load_font_from_theme(&app_theme, &mut font_db);
      let font_db = Rc::new(RefCell::new(font_db));
      let shaper = TextShaper::new(font_db.clone());
      let reorder = TextReorder::default();
      let typography_store = TypographyStore::new(reorder.clone(), font_db.clone(), shaper.clone());

      let executor = LocalPool::new();
      let scheduler = executor.spawner();

      let ctx = AppCtx {
        font_db,
        app_theme,
        shaper,
        reorder,
        typography_store,
        clipboard: RefCell::new(Box::new(MockClipboard {})),
        executor: RefCell::new(executor),
        scheduler,
        runtime_waker: Box::new(MockWaker),
        windows: RefCell::new(ahash::HashMap::default()),
        triggers: RefCell::new(ahash::HashMap::default()),
      };

      INIT_THREAD_ID = Some(std::thread::current().id());
      APP_CTX = Some(ctx);
    });

    Self::thread_check();
    APP_CTX.as_mut().unwrap_unchecked()
  }
}

impl AppCtx {
  pub fn wait_future<F: Future>(f: F) -> F::Output { block_on(f) }

  #[inline]
  pub fn spawn_local<Fut>(future: Fut) -> Result<(), SpawnError>
  where
    Fut: Future<Output = ()> + 'static,
  {
    let ctx = AppCtx::shared();
    ctx.scheduler.spawn_local(LocalFuture {
      fut: future,
      waker: ctx.runtime_waker.clone(),
    })
  }
}

pin_project! {
  struct LocalFuture<F> {
    #[pin]
    fut: F,
    waker: Box<dyn RuntimeWaker + Send>,
  }
}

impl Clone for Box<dyn RuntimeWaker + Send> {
  fn clone(&self) -> Self { self.clone_box() }
}

impl<F> LocalFuture<F>
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
    let raw = RawWaker::new(
      Box::leak(data) as *const RawLocalWaker as *const (),
      &VTABLE,
    );
    unsafe { Waker::from_raw(raw) }
  }
}

impl<F> Future for LocalFuture<F>
where
  F: Future,
{
  type Output = F::Output;
  fn poll(
    self: std::pin::Pin<&mut Self>,
    cx: &mut std::task::Context<'_>,
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

pub fn load_font_from_theme(theme: &Theme, font_db: &mut FontDB) {
  match theme {
    Theme::Full(FullTheme { font_bytes, font_files, .. })
    | Theme::Inherit(InheritTheme { font_bytes, font_files, .. }) => {
      if let Some(font_bytes) = font_bytes {
        font_bytes
          .iter()
          .for_each(|data| font_db.load_from_bytes(data.clone()));
      }
      if let Some(font_files) = font_files {
        font_files.iter().for_each(|path| {
          let _ = font_db.load_font_file(path);
        });
      }
    }
  }
}

impl Drop for AppCtxScopeGuard {
  fn drop(&mut self) {
    let ctx = AppCtx::shared();

    {
      // clear resources may introduce new triggers, so keep it to delay release.
      let _windows = std::mem::take(&mut *ctx.windows.borrow_mut());
      while !ctx.triggers.borrow().is_empty() {
        let _vec = std::mem::take(&mut *ctx.triggers.borrow_mut());
      }
    }

    // Safety: this guard guarantee only one thread can access the `AppCtx`.
    unsafe {
      APP_CTX = None;
      INIT_THREAD_ID = None;
      APP_CTX_INIT = Once::new();
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use futures::Future;
  use std::{
    cell::RefCell,
    rc::Rc,
    sync::Arc,
    sync::Mutex,
    task::{Poll, Waker},
  };

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
    trigger: Rc<RefCell<Trigger>>,
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
    let _guard = unsafe { AppCtx::new_lock_scope() };

    struct WakerCnt(Arc<Mutex<usize>>);
    impl RuntimeWaker for WakerCnt {
      fn wake(&self) { *self.0.lock().unwrap() += 1; }
      fn clone_box(&self) -> Box<dyn RuntimeWaker + Send> { Box::new(WakerCnt(self.0.clone())) }
    }

    let ctx_wake_cnt = Arc::new(Mutex::new(0));
    let wake_cnt = ctx_wake_cnt.clone();
    unsafe { AppCtx::set_runtime_waker(Box::new(WakerCnt(wake_cnt))) }

    let triggers = (0..3)
      .map(|_| Rc::new(RefCell::new(Trigger::default())))
      .collect::<Vec<_>>();
    let futs = triggers
      .clone()
      .into_iter()
      .map(|trigger| ManualFuture { trigger, cnt: 1 });

    let acc = Rc::new(RefCell::new(0));
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
}
