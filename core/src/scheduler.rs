use std::{
  cell::RefCell,
  future::Future,
  pin::Pin,
  task::{Context, RawWaker, RawWakerVTable, Waker},
};

use pin_project_lite::pin_project;
use tokio::task::LocalSet;
use tokio_run_until_stalled::*;

cfg_if::cfg_if! {
  if #[cfg(target_arch = "wasm32")] {
    mod wasm_scheduler;
    pub type RibirScheduler = wasm_scheduler::WasmScheduler;
  } else {
    mod tokio_scheduler;
    pub type RibirScheduler = tokio_scheduler::TokioScheduler;
  }
}

#[cfg(not(target_arch = "wasm32"))]
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;
#[cfg(target_arch = "wasm32")]
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;

#[derive(Default)]
enum LocalPoolState {
  #[default]
  Empty,
  WaitToRun(LocalSet),
  Running,
}

impl LocalPoolState {
  fn spawn_local(&mut self, fut: impl Future<Output = ()> + 'static) {
    match self {
      LocalPoolState::Empty => {
        let local_set = LocalSet::new();
        local_set.spawn_local(fut);
        *self = LocalPoolState::WaitToRun(local_set);
      }
      LocalPoolState::WaitToRun(local_set) => {
        local_set.spawn_local(fut);
      }
      LocalPoolState::Running => {
        tokio::task::spawn_local(fut);
      }
    }
  }

  fn take_to_run(&mut self) -> Option<LocalSet> {
    match std::mem::replace(self, LocalPoolState::Running) {
      LocalPoolState::WaitToRun(local_set) => Some(local_set),
      LocalPoolState::Empty => Some(LocalSet::new()),
      _ => None,
    }
  }

  fn is_empty(&self) -> bool { matches!(self, LocalPoolState::Empty) }

  fn is_running(&self) -> bool { matches!(self, LocalPoolState::Running) }

  fn reset(&mut self) -> &mut Self {
    *self = LocalPoolState::Empty;
    self
  }

  fn add_local_set(&mut self, local_set: LocalSet) {
    match self {
      LocalPoolState::Empty => *self = LocalPoolState::WaitToRun(local_set),
      LocalPoolState::WaitToRun(runtime) => {
        runtime.spawn_local(local_set);
      }
      LocalPoolState::Running => {
        tokio::task::spawn_local(local_set);
      }
    }
  }
}

pub trait RuntimeWaker {
  fn clone_box(&self) -> Box<dyn RuntimeWaker + Send>;
  fn wake(&self);
}

impl Clone for Box<dyn RuntimeWaker + Send> {
  fn clone(&self) -> Self { self.clone_box() }
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
      unsafe { drop(this) };
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

#[derive(Default)]
pub struct LocalPool {
  local_set: RefCell<LocalPoolState>,
}

impl LocalPool {
  pub fn spawn_local(&self, fut: impl Future<Output = ()> + 'static) {
    self.local_set.borrow_mut().spawn_local(fut);
  }

  pub fn run_until_stalled(&self, runtime_waker: Option<Box<dyn RuntimeWaker + Send>>) {
    if self.local_set.borrow().is_empty() {
      return;
    }

    if self.local_set.borrow().is_running() {
      panic!("Local pool is already running");
    }

    let local_set = self.local_set.borrow_mut().take_to_run().unwrap();
    let fut = local_set.run_until_stalled();
    let local_set = if let Some(waker) = runtime_waker {
      self
        .run_non_block(WakerFuture { fut, waker })
        .into_local_set()
    } else {
      self.run_non_block(fut).into_local_set()
    };
    self.local_set.borrow_mut().reset();
    if let Some(local_set) = local_set {
      self
        .local_set
        .borrow_mut()
        .add_local_set(local_set);
    }
  }

  #[cfg(not(target_arch = "wasm32"))]
  fn run_until<F: Future>(&self, fut: F, rt: &tokio::runtime::Runtime) -> F::Output {
    if self.local_set.borrow().is_running() {
      panic!("Local pool is already running");
    }

    let local_set = self.local_set.borrow_mut().take_to_run().unwrap();

    let res = rt.block_on(local_set.run_until(fut));
    self
      .local_set
      .borrow_mut()
      .reset()
      .add_local_set(local_set);
    res
  }

  #[cfg(not(target_arch = "wasm32"))]
  fn run(&self, rt: &tokio::runtime::Runtime) {
    if self.local_set.borrow().is_empty() {
      return;
    }

    if self.local_set.borrow().is_running() {
      panic!("Local pool is already running");
    }

    let local_set = self.local_set.borrow_mut().take_to_run().unwrap();
    rt.block_on(local_set);
    self.local_set.borrow_mut().reset();
  }

  pub fn is_empty(&self) -> bool { self.local_set.borrow().is_empty() }

  #[allow(unused_mut)]
  fn run_non_block<F: Future + Unpin>(&self, mut fut: F) -> F::Output {
    cfg_if::cfg_if! {
      if #[cfg(not(target_arch = "wasm32"))] {
        crate::scheduler::tokio_scheduler::RUNTIME.block_on(fut)
      } else {
        fn mock_waker() -> Waker {
          fn clone(_: *const ()) -> RawWaker { RawWaker::new(std::ptr::null(), &VTABLE) }
          unsafe fn wake(_: *const ()) {}
          unsafe fn wake_by_ref(_: *const ()) {}
          unsafe fn drop(_: *const ()) {}

          static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, wake, wake_by_ref, drop);
          let raw = RawWaker::new(std::ptr::null(), &VTABLE);
          unsafe { Waker::from_raw(raw) }
        }

        let waker = mock_waker();
        let mut cx = Context::from_waker(&waker);

        match Pin::new(&mut fut).poll(&mut cx) {
          std::task::Poll::Ready(res) => res,
          std::task::Poll::Pending => unreachable!(),
        }
      }
    }
  }
}
