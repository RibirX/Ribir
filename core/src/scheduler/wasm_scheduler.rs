use std::future::Future;

use wasm_bindgen_futures::spawn_local;

use crate::{prelude::Duration, scheduler::BoxFuture};
pub struct WasmScheduler {}

impl WasmScheduler {
  pub fn spawn_local(fut: impl Future<Output = ()> + 'static) { spawn_local(fut); }

  pub fn timer(duration: Duration) -> BoxFuture<'static, ()> {
    Box::pin(gloo_timers::future::sleep(duration))
  }
}
