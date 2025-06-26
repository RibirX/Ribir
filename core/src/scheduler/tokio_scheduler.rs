use std::{future::Future, sync::LazyLock};

use tokio::runtime::Runtime;

use crate::{prelude::Duration, scheduler::BoxFuture};

mod thread_local {
  use std::future::Future;

  use tokio::{runtime::Runtime, task::LocalSet};

  use crate::scheduler::LocalPool;

  thread_local! {
    // tokio's context need to be visit before thread_local LOCAL_RUNTIME, otherwise it will panic because
    // the Context will be disposed before thread_local LOCAL_RUNTIME, which
    // will visit Context.
    static ONCE_INIT: () =  std::mem::drop(LocalSet::new().enter());
    static LOCAL_POOL: LocalPool = LocalPool::default();
  }
  pub struct LocalPoolAccessor {}

  impl LocalPoolAccessor {
    pub fn run_until_stalled(&self) {
      LOCAL_POOL.with(|local_pool| {
        local_pool.run_until_stalled(None);
      });
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn run_until<F: Future>(&self, fut: F, rt: &Runtime) -> F::Output {
      LOCAL_POOL.with(|local_pool| local_pool.run_until(fut, rt))
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn run(&self, rt: &Runtime) { LOCAL_POOL.with(|local_pool| local_pool.run(rt)); }

    pub fn spawn_local(&self, fut: impl Future<Output = ()> + 'static) {
      LOCAL_POOL.with(|local_pool| {
        local_pool.spawn_local(fut);
      });
    }
  }

  pub fn local_pool() -> LocalPoolAccessor {
    ONCE_INIT.with(|_| ());
    LocalPoolAccessor {}
  }
}

use thread_local::*;

pub(crate) static RUNTIME: LazyLock<Runtime> =
  LazyLock::new(|| Runtime::new().expect("Failed building the Runtime"));

pub struct TokioScheduler {}

impl TokioScheduler {
  pub fn enter() -> tokio::runtime::EnterGuard<'static> { RUNTIME.enter() }

  pub fn spawn_local(fut: impl Future<Output = ()> + 'static) { local_pool().spawn_local(fut); }

  pub fn run_until<F: Future>(fut: F) -> F::Output { local_pool().run_until(fut, &RUNTIME) }

  pub fn run() { local_pool().run(&RUNTIME) }

  pub fn run_until_stalled() { local_pool().run_until_stalled(); }

  pub fn timer(duration: Duration) -> BoxFuture<'static, ()> {
    Box::pin(tokio::time::sleep(duration))
  }

  pub fn spawn<F>(fut: F) -> tokio::task::JoinHandle<F::Output>
  where
    F: Future + 'static + Send,
    F::Output: Send,
  {
    RUNTIME.spawn(fut)
  }
}
