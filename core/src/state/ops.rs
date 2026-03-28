use std::convert::Infallible;

use rxrust::{
  ops::{
    distinct_until_changed::{DistinctUntilChanged, DistinctUntilChangedObserver},
    map_to::MapTo,
    switch_map::{SwitchMap, SwitchMapOuterObserver},
  },
  subscription::IntoBoxedSubscription,
};

use super::{PriorOp, Priority, WindowPriority};
use crate::{prelude::*, window::WindowId};

pub struct DelayBool<S> {
  source: S,
  show_delay: Duration,
  hide_delay: Duration,
}

type TimerCtx<C> = <<C as Context>::With<()> as Context>::With<Timer<<C as Context>::Scheduler>>;
type DelayInner<C> = <TimerCtx<C> as Context>::With<MapTo<Timer<<C as Context>::Scheduler>, bool>>;
type DelayFn<C> = Box<dyn FnMut(bool) -> DelayInner<C>>;
type DelayObserver<C> = DistinctUntilChangedObserver<
  SwitchMapOuterObserver<<C as Context>::Scope, <C as Context>::Inner, DelayFn<C>>,
  bool,
>;
type DelayPipeline<C, S> = SwitchMap<DistinctUntilChanged<S>, DelayFn<C>>;

impl<S> ObservableType for DelayBool<S>
where
  for<'a> S: ObservableType<Err = Infallible, Item<'a> = bool> + 'a,
{
  type Item<'a>
    = bool
  where
    Self: 'a;
  type Err = Infallible;
}

impl<C, S> CoreObservable<C> for DelayBool<S>
where
  C: Context,
  C::Scheduler: 'static,
  C::With<()>: ObservableFactory,
  for<'a> S: ObservableType<Err = Infallible, Item<'a> = bool> + 'a,
  S: CoreObservable<C::With<DelayObserver<C>>>,
  DelayPipeline<C, S>: CoreObservable<C, Unsub: IntoBoxedSubscription<C::BoxedSubscription>>,
{
  type Unsub = C::BoxedSubscription;

  fn subscribe(self, context: C) -> Self::Unsub {
    let Self { source, show_delay, hide_delay } = self;

    SwitchMap {
      source: DistinctUntilChanged(source),
      func: Box::new(move |value| {
        let delay = if value { show_delay } else { hide_delay };
        <C::With<()> as ObservableFactory>::timer(delay).map_to(value)
      }) as DelayFn<C>,
    }
    .subscribe(context)
    .into_boxed()
  }
}

/// Observable extensions used by Ribir-specific state and event flows.
pub trait RibirObservable: Observable + Sized {
  /// This method specifies a static priority to the Observable in the priority
  /// queue of the window. A lower priority value indicates higher priority.
  fn value_priority(
    self, priority: i64, wnd_id: WindowId,
  ) -> Self::With<PriorOp<Self::Inner, WindowPriority<impl FnMut() -> i64>>>
  where
    Self: 'static,
    Self::Err: Clone + 'static,
    for<'a> Self::Item<'a>: Clone + 'static,
  {
    self.fn_priority(move || priority, wnd_id)
  }

  /// The method defines the priority for the Observable in the window's
  /// priority queue. The priority value is calculated for each emitted value by
  /// the function `f`, with lower values indicating higher priority.
  fn fn_priority(
    self, f: impl FnMut() -> i64 + 'static, wnd_id: WindowId,
  ) -> Self::With<PriorOp<Self::Inner, WindowPriority<impl FnMut() -> i64>>>
  where
    Self: 'static,
    Self::Err: Clone + 'static,
    for<'a> Self::Item<'a>: Clone + 'static,
  {
    self.priority(WindowPriority::new(wnd_id, f))
  }

  /// Collect observable emissions into the window priority queue.
  fn priority<P: Priority + 'static>(self, priority: P) -> Self::With<PriorOp<Self::Inner, P>>
  where
    Self: 'static,
    Self::Err: Clone + 'static,
    for<'a> Self::Item<'a>: Clone + 'static,
  {
    self.transform(|source| PriorOp::new(source, priority))
  }

  /// Delay a boolean stream and cancel stale pending emissions when a new value
  /// arrives.
  fn delay_bool(
    self, show_delay: Duration, hide_delay: Duration,
  ) -> Self::With<DelayBool<Self::Inner>>
  where
    Self: 'static,
    for<'a> Self::Item<'a>: Clone + PartialEq + Into<bool>,
  {
    self.transform(|source| DelayBool { source, show_delay, hide_delay })
  }
}

impl<T> RibirObservable for T where T: Observable + Sized {}

#[cfg(test)]
mod tests {
  use std::{cell::RefCell, rc::Rc};

  use crate::{prelude::*, reset_test_env};

  // Use generous margins so the timer tests stay stable across CI runners and
  // platform-specific scheduler jitter.
  const SHOW_DELAY: Duration = Duration::from_millis(150);
  const HIDE_DELAY: Duration = Duration::from_millis(60);
  const TIMER_SLACK: Duration = Duration::from_millis(20);

  fn wait(duration: Duration) {
    AppCtx::run_until(AppCtx::timer(duration + TIMER_SLACK));
    AppCtx::run_until_stalled();
  }

  #[test]
  fn delayed_bool_emits_true_after_show_delay() {
    reset_test_env!();

    let mut source = Local::subject();
    let values: Rc<RefCell<Vec<bool>>> = Rc::default();
    let _sub = source
      .clone()
      .delay_bool(SHOW_DELAY, HIDE_DELAY)
      .subscribe({
        let values = values.clone();
        move |value| values.borrow_mut().push(value)
      });

    source.next(true);
    wait(Duration::from_millis(20));
    assert!(values.borrow().is_empty());

    wait(Duration::from_millis(110));
    assert_eq!(*values.borrow(), vec![true]);
  }

  #[test]
  fn delayed_bool_emits_false_after_hide_delay() {
    reset_test_env!();

    let mut source = Local::subject();
    let values: Rc<RefCell<Vec<bool>>> = Rc::default();
    let _sub = source
      .clone()
      .delay_bool(SHOW_DELAY, HIDE_DELAY)
      .subscribe({
        let values = values.clone();
        move |value| values.borrow_mut().push(value)
      });

    source.next(false);
    wait(Duration::from_millis(20));
    assert!(values.borrow().is_empty());

    wait(Duration::from_millis(40));
    assert_eq!(*values.borrow(), vec![false]);
  }

  #[test]
  fn delayed_bool_cancels_pending_true_when_false_arrives() {
    reset_test_env!();

    let mut source = Local::subject();
    let values: Rc<RefCell<Vec<bool>>> = Rc::default();
    let _sub = source
      .clone()
      .delay_bool(SHOW_DELAY, HIDE_DELAY)
      .subscribe({
        let values = values.clone();
        move |value| values.borrow_mut().push(value)
      });

    source.next(true);
    wait(Duration::from_millis(40));
    source.next(false);

    wait(Duration::from_millis(60));
    assert_eq!(*values.borrow(), vec![false]);
  }

  #[test]
  fn delayed_bool_ignores_repeated_values() {
    reset_test_env!();

    let mut source = Local::subject();
    let values: Rc<RefCell<Vec<bool>>> = Rc::default();
    let _sub = source
      .clone()
      .delay_bool(SHOW_DELAY, HIDE_DELAY)
      .subscribe({
        let values = values.clone();
        move |value| values.borrow_mut().push(value)
      });

    source.next(true);
    wait(Duration::from_millis(80));
    source.next(true);

    wait(Duration::from_millis(80));
    assert_eq!(*values.borrow(), vec![true]);
  }

  #[test]
  fn delayed_bool_unsubscribe_cancels_pending_timer() {
    reset_test_env!();

    let mut source = Local::subject();
    let values: Rc<RefCell<Vec<bool>>> = Rc::default();
    let sub = source
      .clone()
      .delay_bool(SHOW_DELAY, HIDE_DELAY)
      .subscribe({
        let values = values.clone();
        move |value| values.borrow_mut().push(value)
      });

    source.next(true);
    sub.unsubscribe();

    wait(SHOW_DELAY);
    assert!(values.borrow().is_empty());
  }
}
