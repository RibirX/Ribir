use std::cell::RefCell;

use priority_queue::PriorityQueue;
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
  observer: O,
  priority: P,
}

/// This trait defines an Observable that can be assigned a priority. The
/// `Priority` will collect the values and emitted in order by the priority.
pub trait PriorityObservable: Observable + Sized {
  /// The item type must be owned (`'static`) so it can be queued.
  type Item: Clone + 'static;
  type Err: Clone + 'static;

  /// This method specifies a static priority to the Observable in the priority
  /// queue of the window. A lower priority value indicates higher priority.
  fn value_priority(
    self, priority: i64, wnd_id: WindowId,
  ) -> Self::With<PriorOp<Self::Inner, WindowPriority<impl FnMut() -> i64>>> {
    self.fn_priority(move || priority, wnd_id)
  }

  /// The method defines the priority for the Observable in the window's
  /// priority queue. The priority value is calculated for each emitted value by
  /// the function `f`, with lower values indicating higher priority.
  fn fn_priority(
    self, f: impl FnMut() -> i64 + 'static, wnd_id: WindowId,
  ) -> Self::With<PriorOp<Self::Inner, WindowPriority<impl FnMut() -> i64>>> {
    self.priority(WindowPriority { wnd_id, priority: f })
  }

  fn priority<P: Priority + 'static>(self, priority: P) -> Self::With<PriorOp<Self::Inner, P>> {
    self.transform(|source| PriorOp { source, priority })
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

impl<T> PriorityObservable for T
where
  T: Observable + Sized + 'static,
  T::Err: Clone + 'static,
  for<'a> T::Item<'a>: Clone + 'static,
{
  type Item = T::Item<'static>;
  type Err = T::Err;
}

impl<S, P> ObservableType for PriorOp<S, P>
where
  S: ObservableType,
{
  type Item<'a>
    = S::Item<'a>
  where
    Self: 'a;
  type Err = S::Err;
}

type PriorObserverCtx<C, P> =
  <C as Context>::With<PriorObserver<<C as Context>::RcMut<Option<<C as Context>::Inner>>, P>>;

impl<C, S, P> CoreObservable<C> for PriorOp<S, P>
where
  C: Context,
  S: CoreObservable<PriorObserverCtx<C, P>> + 'static,
  PrioritySubscription<C::RcMut<Option<C::Inner>>>: Subscription,
{
  type Unsub = SourceWithHandle<S::Unsub, PrioritySubscription<C::RcMut<Option<C::Inner>>>>;

  fn subscribe(self, context: C) -> Self::Unsub {
    let Self { source, priority } = self;
    let rc_observer = C::RcMut::from(None);
    let context = context.transform(|observer| {
      *rc_observer.rc_deref_mut() = Some(observer);
      PriorObserver { observer: rc_observer.clone(), priority }
    });

    let source = source.subscribe(context);

    SourceWithHandle { source, handle: PrioritySubscription(rc_observer) }
  }
}

impl<RcO, P, Item: 'static, Err: 'static> Observer<Item, Err> for PriorObserver<RcO, P>
where
  RcO: Observer<Item, Err> + Clone + 'static,
  P: Priority + 'static,
{
  fn next(&mut self, value: Item) {
    let priority = self.priority.priority();
    if let Some(queue) = self.priority.queue() {
      let mut observer = self.observer.clone();
      let task: Box<dyn FnOnce()> = Box::new(move || observer.next(value));
      queue.add(PriorityTask(task), priority);
    } else {
      self.observer.next(value)
    }
  }

  fn error(mut self, err: Err) {
    let priority = self.priority.priority();
    if let Some(queue) = self.priority.queue() {
      let observer = self.observer.clone();
      let task: Box<dyn FnOnce()> = Box::new(move || observer.error(err));
      queue.add(PriorityTask(task), priority + 1);
    } else {
      self.observer.error(err)
    }
  }

  fn complete(mut self) {
    let priority = self.priority.priority();
    if let Some(queue) = self.priority.queue() {
      let observer = self.observer.clone();
      let task: Box<dyn FnOnce()> = Box::new(move || {
        observer.complete();
      });
      queue.add(PriorityTask(task), priority + 1);
    } else {
      self.observer.complete()
    }
  }

  fn is_closed(&self) -> bool { self.observer.is_closed() }
}

pub struct PrioritySubscription<RcO>(RcO);

impl<RcO, O> Subscription for PrioritySubscription<RcO>
where
  RcO: RcDerefMut<Target = Option<O>>,
{
  fn unsubscribe(self) { self.0.rc_deref_mut().take(); }

  fn is_closed(&self) -> bool { self.0.rc_deref().is_none() }
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
  use crate::{prelude::*, reset_test_env, test_helper::TestWindow};

  #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
  #[test]
  fn prior_smoke() {
    reset_test_env!();

    let wnd = TestWindow::from_widget(fn_widget!(Void));
    let wnd_id = wnd.id();

    let r = Stateful::new(Vec::new());

    let result = r.clone_writer();
    Local::of(2)
      .value_priority(2, wnd_id)
      .subscribe(move |v: i32| result.write().push(v));

    let result = r.clone_writer();
    Local::of(1)
      .value_priority(1, wnd_id)
      .subscribe(move |v| result.write().push(v));

    let result = r.clone_writer();
    Local::of(3)
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
    let wnd = TestWindow::from_widget(fn_widget!(Void));
    let wnd_id = wnd.id();

    let result = r.clone_writer();
    Local::of(2)
      .fn_priority(|| 2, wnd_id)
      .subscribe(move |v| result.write().push(v));
    let result = r.clone_writer();
    Local::of(1)
      .fn_priority(|| 1, wnd_id)
      .subscribe(move |v| result.write().push(v));
    let result = r.clone_writer();
    Local::of(3)
      .fn_priority(|| 3, wnd_id)
      .subscribe(move |v| result.write().push(v));

    wnd.draw_frame();
    assert_eq!(*r.read(), vec![1, 2, 3]);
  }
}
