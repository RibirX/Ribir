use std::cell::RefCell;

use priority_queue::PriorityQueue;
use ribir_algo::Sc;
use rxrust::prelude::*;

/// A priority queue of tasks. So that tasks with higher priority will be
/// executed first.
#[derive(Clone, Default)]
pub struct PriorityTaskQueue<'a>(
  Sc<RefCell<PriorityQueue<PriorityTask<'a>, i64, ahash::RandomState>>>,
);

pub struct PriorOp<'a, P, S> {
  prior_fn: P,
  source: S,
  scheduler: PriorityTaskQueue<'a>,
}

pub struct PriorityTask<'a>(Box<dyn FnOnce() + 'a>);

pub struct PriorObserver<'a, O, F> {
  observer: Sc<RefCell<Option<O>>>,
  prior_fn: F,
  scheduler: PriorityTaskQueue<'a>,
}

/// A trait for Observable that can be assigned a priority queue to collect its
/// value with a priority. The values will be emitted in order when the
/// `PriorityTaskQueue::run` method is called.
pub trait PriorityObservable<Item, Err>: ObservableExt<Item, Err> {
  /// Specify the priority queue an Observable should use to collect its values
  /// with a priority. The lower the priority value, the higher the priority.
  fn prior(
    self, prior: i64, scheduler: PriorityTaskQueue,
  ) -> PriorOp<impl FnMut() -> i64 + 'static, Self>
  where
    Self: Sized,
  {
    PriorOp { prior_fn: move || prior, source: self, scheduler }
  }

  /// Specify the priority queue an Observable should use to collect its values
  /// and every value will be assigned a priority by the given function. The
  /// lower the priority value, the higher the priority.
  fn prior_by<P>(self, prior_fn: P, scheduler: PriorityTaskQueue) -> PriorOp<P, Self>
  where
    Self: Sized,
    P: FnMut() -> i64,
  {
    PriorOp { prior_fn, source: self, scheduler }
  }
}

impl<Item, Err, T> PriorityObservable<Item, Err> for T where T: ObservableExt<Item, Err> {}

impl<'a, Item: 'a, Err: 'a, O, S, P> Observable<Item, Err, O> for PriorOp<'a, P, S>
where
  O: Observer<Item, Err> + 'a,
  S: Observable<Item, Err, PriorObserver<'a, O, P>>,
  P: FnMut() -> i64,
{
  type Unsub = ZipSubscription<S::Unsub, PrioritySubscription<O>>;

  fn actual_subscribe(self, observer: O) -> Self::Unsub {
    let Self { prior_fn, source, scheduler } = self;
    let observer = Sc::new(RefCell::new(Some(observer)));
    let o2 = observer.clone();
    let u = source.actual_subscribe(PriorObserver { observer, prior_fn, scheduler });
    ZipSubscription::new(u, PrioritySubscription(o2))
  }
}

impl<'a, Item: 'a, Err: 'a, S, P> ObservableExt<Item, Err> for PriorOp<'a, P, S>
where
  S: ObservableExt<Item, Err>,
  P: FnMut() -> i64,
{
}

impl<'a, Item: 'a, Err: 'a, O, F> Observer<Item, Err> for PriorObserver<'a, O, F>
where
  O: Observer<Item, Err> + 'a,
  F: FnMut() -> i64,
{
  fn next(&mut self, value: Item) {
    let priority = (self.prior_fn)();
    let observer = self.observer.clone();
    let task = PriorityTask(Box::new(move || {
      if let Some(o) = observer.borrow_mut().as_mut() {
        o.next(value)
      }
    }));
    self.scheduler.add(task, priority)
  }

  fn error(mut self, err: Err) {
    let priority = (self.prior_fn)();
    let task = PriorityTask(Box::new(move || {
      if let Some(o) = self.observer.borrow_mut().take() {
        o.error(err)
      }
    }));
    self.scheduler.add(task, priority + 1)
  }

  fn complete(mut self) {
    let priority = (self.prior_fn)();
    let task = PriorityTask(Box::new(move || {
      if let Some(o) = self.observer.borrow_mut().take() {
        o.complete()
      }
    }));
    self.scheduler.add(task, priority + 1)
  }

  fn is_finished(&self) -> bool { self.observer.borrow().is_none() }
}

pub struct PrioritySubscription<O>(Sc<RefCell<Option<O>>>);

impl<O> Subscription for PrioritySubscription<O> {
  fn unsubscribe(self) { self.0.borrow_mut().take(); }

  fn is_closed(&self) -> bool { self.0.borrow().is_none() }
}

impl<'a> PartialEq for PriorityTask<'a> {
  fn eq(&self, _: &Self) -> bool {
    // Three isn't two task that are equal.
    false
  }
}

impl<'a> Eq for PriorityTask<'a> {}

impl<'a> std::hash::Hash for PriorityTask<'a> {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) { std::ptr::hash(&*self.0, state) }
}

impl<'a> PriorityTaskQueue<'a> {
  pub fn is_empty(&self) -> bool { self.0.borrow().is_empty() }

  pub fn pop(&self) -> Option<(PriorityTask<'a>, i64)> { self.0.borrow_mut().pop() }

  /// Add a task to the queue with a priority.
  pub fn add(&self, task: PriorityTask<'a>, priority: i64) {
    self.0.borrow_mut().push(task, -priority);
  }
}

impl<'a> PriorityTask<'a> {
  /// Create a new task.
  pub fn new(f: impl FnOnce() + 'static) -> Self { PriorityTask(Box::new(f)) }

  pub fn run(self) { (self.0)() }
}

#[cfg(test)]
mod tests {
  use super::*;
  #[cfg(target_arch = "wasm32")]
  use crate::test_helper::wasm_bindgen_test;

  #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
  #[test]
  fn prior_smoke() {
    let result = RefCell::new(Vec::new());
    let queue = PriorityTaskQueue::default();

    observable::of(2)
      .prior(2, queue.clone())
      .subscribe(|v| result.borrow_mut().push(v));

    observable::of(1)
      .prior(1, queue.clone())
      .subscribe(|v| result.borrow_mut().push(v));

    observable::of(3)
      .prior(3, queue.clone())
      .subscribe(|v| result.borrow_mut().push(v));

    while let Some((task, _p)) = queue.pop() {
      task.run()
    }

    assert_eq!(*result.borrow(), vec![1, 2, 3]);
  }

  #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
  #[test]
  fn prior_by_smoke() {
    let result = RefCell::new(Vec::new());
    let queue = PriorityTaskQueue::default();

    observable::of(2)
      .prior_by(|| 2, queue.clone())
      .subscribe(|v| result.borrow_mut().push(v));
    observable::of(1)
      .prior_by(|| 1, queue.clone())
      .subscribe(|v| result.borrow_mut().push(v));
    observable::of(3)
      .prior_by(|| 3, queue.clone())
      .subscribe(|v| result.borrow_mut().push(v));

    while let Some((task, _p)) = queue.pop() {
      task.run()
    }
    assert_eq!(*result.borrow(), vec![1, 2, 3]);
  }
}
