use std::{
  collections::BTreeMap,
  future::Future,
  mem::swap,
  sync::{
    atomic::{AtomicUsize, Ordering},
    Mutex,
  },
  task::{Poll, Waker},
};

use once_cell::sync::Lazy;
use rxrust::scheduler::BoxFuture;

use crate::ticker::{Duration, Instant};

#[derive(Default)]
pub(crate) struct TimeReactor {
  timers: BTreeMap<(Instant, usize), Waker>,
}

impl TimeReactor {
  pub(crate) fn timeout_wakers(&mut self, mut before: Instant) -> impl Iterator<Item = Waker> {
    before += Duration::from_nanos(1);
    let mut notifies = self.timers.split_off(&(before, 0));
    swap(&mut self.timers, &mut notifies);
    notifies.into_values()
  }

  pub(crate) fn recently_timeout(&self) -> Option<Instant> {
    self.timers.keys().next().map(|(t, _)| *t)
  }

  fn insert_timer(&mut self, when: Instant, waker: Waker) -> usize {
    // Generate a new timer ID, deal with timer's time conflict.
    static ID_GENERATOR: AtomicUsize = AtomicUsize::new(1);
    let id = ID_GENERATOR.fetch_add(1, Ordering::Relaxed);

    self.timers.insert((when, id), waker);
    id
  }

  fn remove_timer(&mut self, when: Instant, id: usize) { self.timers.remove(&(when, id)); }
}

pub(crate) static TIME_REACTOR: Lazy<Mutex<TimeReactor>> =
  Lazy::new(|| Mutex::new(TimeReactor::default()));

pub struct Timer {
  id: Option<usize>,
  when: Instant,
}

impl Timer {
  pub fn new(when: Instant) -> Self { Self { id: None, when } }

  pub fn reset(&mut self, timer: Instant) {
    if let Some(id) = self.id.take() {
      TIME_REACTOR
        .lock()
        .unwrap()
        .remove_timer(self.when, id)
    }
    self.when = timer;
  }

  pub fn recently_timeout() -> Option<Instant> { TIME_REACTOR.lock().unwrap().recently_timeout() }

  pub fn new_timer_future(dur: Duration) -> BoxFuture<'static, ()> {
    Box::pin(Timer::new(Instant::now() + dur))
  }

  pub fn wake_timeout_futures() {
    let notifies = TIME_REACTOR
      .lock()
      .unwrap()
      .timeout_wakers(Instant::now());
    notifies.for_each(|waker| waker.wake());
  }
}

impl Future for Timer {
  type Output = ();
  fn poll(
    mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>,
  ) -> std::task::Poll<Self::Output> {
    let now = Instant::now();
    let when = self.as_ref().when;
    if let Some(id) = self.as_mut().id.take() {
      TIME_REACTOR
        .lock()
        .unwrap()
        .remove_timer(when, id);
    }
    if now >= when {
      return Poll::Ready(());
    }

    self.as_mut().id = Some(
      TIME_REACTOR
        .lock()
        .unwrap()
        .insert_timer(when, cx.waker().clone()),
    );
    Poll::Pending
  }
}
