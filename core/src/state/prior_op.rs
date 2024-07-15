use std::cell::RefCell;

use priority_queue::PriorityQueue;
use ribir_algo::Sc;
use rxrust::prelude::*;

use super::AppCtx;
use crate::window::WindowId;

/// A priority queue of tasks. So that tasks with higher priority will be
/// executed first.
#[derive(Default)]
pub struct PriorityTaskQueue(RefCell<PriorityQueue<PriorityTask, i64, ahash::RandomState>>);

pub struct PriorOp<S, P> {
  source: S,
  priority: P,
}

pub struct PriorityTask(Box<dyn FnOnce()>);

pub struct PriorObserver<O, P> {
  observer: Sc<RefCell<Option<O>>>,
  priority: P,
}

/// This trait defines an Observable that can be assigned a priority. The
/// `Priority` will collect the values and emitted in order by the priority.
pub trait PriorityObservable<Item, Err>: ObservableExt<Item, Err> {
  /// This method specifies a static priority to the Observable in the priority
  /// queue of the window. A lower priority value indicates higher priority.
  fn value_priority(
    self, priority: i64, wnd_id: WindowId,
  ) -> PriorOp<Self, WindowPriority<impl FnMut() -> i64>> {
    self.fn_priority(move || priority, wnd_id)
  }

  /// The method defines the priority for the Observable in the window's
  /// priority queue. The priority value is calculated for each emitted value by
  /// the function `f`, with lower values indicating higher priority.
  fn fn_priority(
    self, f: impl FnMut() -> i64, wnd_id: WindowId,
  ) -> PriorOp<Self, WindowPriority<impl FnMut() -> i64>> {
    PriorOp { source: self, priority: WindowPriority { wnd_id, priority: f } }
  }

  fn priority<P: Priority>(self, priority: P) -> PriorOp<Self, P> {
    PriorOp { source: self, priority }
  }
}

/// This trait is used to determine the priority of a task and the queue used to
/// collect these tasks.
pub trait Priority {
  fn priority(&mut self) -> i64;

  fn queue(&mut self) -> Option<&PriorityTaskQueue>;
}

pub struct WindowPriority<P> {
  wnd_id: WindowId,
  priority: P,
}

impl<P> WindowPriority<P> {
  pub fn new(wnd_id: WindowId, priority: P) -> Self { Self { wnd_id, priority } }
}

impl<P: FnMut() -> i64> Priority for WindowPriority<P> {
  fn priority(&mut self) -> i64 { (self.priority)() }

  fn queue(&mut self) -> Option<&PriorityTaskQueue> {
    AppCtx::get_window(self.wnd_id).map(|wnd| {
      let queue = wnd.priority_task_queue();
      // Safety: This trait is only used within this module, and we can ensure that
      // the window is valid when utilizing the `PriorityTaskQueue`.
      unsafe { std::mem::transmute(queue) }
    })
  }
}

impl Priority for Box<dyn Priority> {
  fn priority(&mut self) -> i64 { (**self).priority() }

  fn queue(&mut self) -> Option<&PriorityTaskQueue> { (**self).queue() }
}

impl<Item, Err, T> PriorityObservable<Item, Err> for T where T: ObservableExt<Item, Err> {}

impl<Item: 'static, Err: 'static, O, S, P> Observable<Item, Err, O> for PriorOp<S, P>
where
  O: Observer<Item, Err> + 'static,
  S: Observable<Item, Err, PriorObserver<O, P>> + 'static,
  P: Priority + 'static,
{
  type Unsub = ZipSubscription<S::Unsub, PrioritySubscription<O>>;

  fn actual_subscribe(self, observer: O) -> Self::Unsub {
    let Self { source, priority } = self;
    let observer = Sc::new(RefCell::new(Some(observer)));
    let o2 = observer.clone();
    let u = source.actual_subscribe(PriorObserver { observer, priority });
    ZipSubscription::new(u, PrioritySubscription(o2))
  }
}

impl<Item, Err, S, P> ObservableExt<Item, Err> for PriorOp<S, P>
where
  S: ObservableExt<Item, Err>,
  P: Priority,
{
}

impl<Item: 'static, Err: 'static, O, P> Observer<Item, Err> for PriorObserver<O, P>
where
  O: Observer<Item, Err> + 'static,
  P: Priority + 'static,
{
  fn next(&mut self, value: Item) {
    let priority = self.priority.priority();
    if let Some(queue) = self.priority.queue() {
      let observer = self.observer.clone();
      let task: Box<dyn FnOnce()> = Box::new(move || {
        if let Some(o) = observer.borrow_mut().as_mut() {
          o.next(value)
        }
      });
      queue.add(PriorityTask(task), priority);
    } else if let Some(o) = self.observer.borrow_mut().as_mut() {
      o.next(value)
    }
  }

  fn error(mut self, err: Err) {
    let priority = self.priority.priority();
    if let Some(queue) = self.priority.queue() {
      let observer = self.observer.clone();
      let task: Box<dyn FnOnce()> = Box::new(move || {
        if let Some(o) = observer.borrow_mut().take() {
          o.error(err)
        }
      });
      queue.add(PriorityTask(task), priority + 1);
    } else if let Some(o) = self.observer.borrow_mut().take() {
      o.error(err)
    }
  }

  fn complete(mut self) {
    let priority = self.priority.priority();
    if let Some(queue) = self.priority.queue() {
      let observer = self.observer.clone();
      let task: Box<dyn FnOnce()> = Box::new(move || {
        if let Some(o) = observer.borrow_mut().take() {
          o.complete()
        }
      });
      queue.add(PriorityTask(task), priority + 1);
    } else if let Some(o) = self.observer.borrow_mut().take() {
      o.complete()
    }
  }

  fn is_finished(&self) -> bool { self.observer.borrow().is_none() }
}

pub struct PrioritySubscription<O>(Sc<RefCell<Option<O>>>);

impl<O> Subscription for PrioritySubscription<O> {
  fn unsubscribe(self) { self.0.borrow_mut().take(); }

  fn is_closed(&self) -> bool { self.0.borrow().is_none() }
}

impl PartialEq for PriorityTask {
  fn eq(&self, _: &Self) -> bool {
    // Three isn't two task that are equal.
    false
  }
}

impl Eq for PriorityTask {}

impl std::hash::Hash for PriorityTask {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) { std::ptr::hash(&*self.0, state) }
}

impl PriorityTaskQueue {
  pub fn is_empty(&self) -> bool { self.0.borrow().is_empty() }

  pub fn pop(&self) -> Option<(PriorityTask, i64)> { self.0.borrow_mut().pop() }

  /// Add a task to the queue with a priority.
  pub fn add(&self, task: PriorityTask, priority: i64) {
    self.0.borrow_mut().push(task, -priority);
  }
}

impl PriorityTask {
  /// Create a new task.
  pub fn new(f: impl FnOnce() + 'static) -> Self { PriorityTask(Box::new(f)) }

  pub fn run(self) { (self.0)() }
}

#[cfg(test)]
mod tests {
  use super::*;
  #[cfg(target_arch = "wasm32")]
  use crate::test_helper::wasm_bindgen_test;
  use crate::{
    prelude::Void,
    reset_test_env,
    state::{StateReader, StateWriter, Stateful},
    test_helper::TestWindow,
  };

  #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
  #[test]
  fn prior_smoke() {
    reset_test_env!();

    let mut wnd = TestWindow::new(Void);
    let wnd_id = wnd.id();

    let r = Stateful::new(Vec::new());

    let result = r.clone_writer();
    observable::of(2)
      .value_priority(2, wnd_id)
      .subscribe(move |v: i32| result.write().push(v));

    let result = r.clone_writer();
    observable::of(1)
      .value_priority(1, wnd_id)
      .subscribe(move |v| result.write().push(v));

    let result = r.clone_writer();
    observable::of(3)
      .value_priority(3, wnd_id)
      .subscribe(move |v| result.write().push(v));

    wnd.draw_frame();

    assert_eq!(*r.read(), vec![1, 2, 3]);
  }

  #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
  #[test]
  fn prior_by_smoke() {
    reset_test_env!();

    let r = Stateful::new(Vec::new());
    let mut wnd = TestWindow::new(Void);
    let wnd_id = wnd.id();

    let result = r.clone_writer();
    observable::of(2)
      .fn_priority(|| 2, wnd_id)
      .subscribe(move |v| result.write().push(v));
    let result = r.clone_writer();
    observable::of(1)
      .fn_priority(|| 1, wnd_id)
      .subscribe(move |v| result.write().push(v));
    let result = r.clone_writer();
    observable::of(3)
      .fn_priority(|| 3, wnd_id)
      .subscribe(move |v| result.write().push(v));

    wnd.draw_frame();
    assert_eq!(*r.read(), vec![1, 2, 3]);
  }
}
