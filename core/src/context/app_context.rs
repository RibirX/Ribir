use crate::{
  builtin_widgets::{FullTheme, InheritTheme, Theme},
  clipboard::{Clipboard, MockClipboard},
};
use pin_project_lite::pin_project;
use std::{
  cell::RefCell,
  ptr::NonNull,
  rc::Rc,
  task::{Context, RawWaker, RawWakerVTable, Waker},
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

#[derive(Clone)]
pub struct AppContext {
  // todo: tmp code, We'll share AppContext by reference.
  app_theme: NonNull<Theme>,
  pub font_db: Rc<RefCell<FontDB>>,
  pub shaper: TextShaper,
  pub reorder: TextReorder,
  pub typography_store: TypographyStore,
  pub clipboard: Rc<RefCell<dyn Clipboard>>,
  pub runtime_waker: Box<dyn RuntimeWaker + Send>,
  executor: Executor,
}

#[derive(Clone)]
pub struct Executor {
  local: Rc<RefCell<LocalPool>>,
}

impl Default for Executor {
  fn default() -> Self {
    Self {
      local: Rc::new(RefCell::new(LocalPool::default())),
    }
  }
}

impl AppContext {
  pub fn new(theme: FullTheme, runtime_waker: Box<dyn RuntimeWaker + Send>) -> Self {
    // temp leak
    let theme = Box::new(Theme::Full(theme));
    let app_theme = Box::leak(theme).into();

    let mut font_db = FontDB::default();
    font_db.load_system_fonts();
    let font_db = Rc::new(RefCell::new(font_db));
    let shaper = TextShaper::new(font_db.clone());
    let reorder = TextReorder::default();
    let typography_store = TypographyStore::new(reorder.clone(), font_db.clone(), shaper.clone());

    let ctx = AppContext {
      font_db,
      app_theme,
      shaper,
      reorder,
      typography_store,
      clipboard: Rc::new(RefCell::new(MockClipboard {})),
      executor: <_>::default(),
      runtime_waker,
    };
    ctx.load_font_from_theme(ctx.app_theme());
    ctx
  }

  pub fn app_theme(&self) -> &Theme { unsafe { self.app_theme.as_ref() } }

  // todo: should &mut self here, but we need to remove `init ctx =>` first
  #[allow(clippy::mut_from_ref)]
  pub fn app_theme_mut(&self) -> &mut Theme {
    let mut ptr = self.app_theme;
    // tmp code
    unsafe { &mut *ptr.as_mut() }
  }

  pub fn scheduler(&self) -> FuturesLocalScheduler { self.executor.local.borrow().spawner() }

  pub(crate) fn end_frame(&mut self) {
    // todo: frame cache is not a good choice? because not every text will relayout
    // in every frame.
    self.shaper.end_frame();
    self.reorder.end_frame();
    self.typography_store.end_frame();
  }

  pub fn load_font_from_theme(&self, theme: &Theme) {
    let mut font_db = self.font_db.borrow_mut();
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

  /// Runs all tasks in the local(usually means on the main thread) pool and
  /// returns if no more progress can be made on any task.
  pub fn run_until_stalled(&self) { self.executor.local.borrow_mut().run_until_stalled() }
}

impl Default for AppContext {
  fn default() -> Self { AppContext::new(<_>::default(), Box::new(MockWaker)) }
}

impl AppContext {
  pub fn wait_future<F: Future>(f: F) -> F::Output { block_on(f) }

  #[inline]
  pub fn spawn_local<Fut>(&self, future: Fut) -> Result<(), SpawnError>
  where
    Fut: Future<Output = ()> + 'static,
  {
    self.runtime_waker.wake();
    self
      .executor
      .local
      .borrow()
      .spawner()
      .spawn_local(LocalFuture {
        fut: future,
        waker: self.runtime_waker.clone(),
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
  fn local_waker(&self, cx: &mut std::task::Context<'_>) -> Waker {
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
    struct WakerCnt(Arc<Mutex<usize>>);
    impl RuntimeWaker for WakerCnt {
      fn wake(&self) { *self.0.lock().unwrap() += 1; }
      fn clone_box(&self) -> Box<dyn RuntimeWaker + Send> { Box::new(WakerCnt(self.0.clone())) }
    }

    let ctx_wake_cnt = Arc::new(Mutex::new(0));
    let wake_cnt = ctx_wake_cnt.clone();
    let ctx = AppContext::new(<_>::default(), Box::new(WakerCnt(wake_cnt)));

    let triggers = (0..3)
      .map(|_| Rc::new(RefCell::new(Trigger::default())))
      .collect::<Vec<_>>();
    let futs = triggers
      .clone()
      .into_iter()
      .map(|trigger| ManualFuture { trigger, cnt: 1 });

    let acc = Rc::new(RefCell::new(0));
    let sum = acc.clone();
    let _ = ctx.spawn_local(async move {
      for fut in futs {
        let v = fut.await;
        *acc.borrow_mut() += v;
      }
    });
    ctx.run_until_stalled();
    let mut waker_cnt = *ctx_wake_cnt.lock().unwrap();

    // when no trigger, nothing will change
    ctx.run_until_stalled();
    assert_eq!(*sum.borrow(), 0);
    assert_eq!(*ctx_wake_cnt.lock().unwrap(), waker_cnt);

    // once call trigger, the ctx.waker will be call once, and future step forward
    for (idx, trigger) in triggers.into_iter().enumerate() {
      trigger.borrow_mut().trigger();
      waker_cnt += 1;
      assert_eq!(*ctx_wake_cnt.lock().unwrap(), waker_cnt);
      ctx.run_until_stalled();
      assert_eq!(*sum.borrow(), idx + 1);
    }
  }
}
