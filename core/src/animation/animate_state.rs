use std::convert::Infallible;

use rxrust::observable::boxed::LocalBoxedObservable;

use super::*;
use crate::prelude::*;

/// Trait to help animations update state.
pub trait AnimateState {
  type Value: Clone;

  fn get(&self) -> Self::Value;
  fn set(&self, v: Self::Value);
  fn revert(&self, v: Self::Value);
  fn animate_state_modifies(&self) -> LocalBoxedObservable<'static, ModifyInfo, Infallible>;
  fn calc_lerp_value(&mut self, from: &Self::Value, to: &Self::Value, rate: f32) -> Self::Value;

  #[doc(hidden)]
  fn dec_writer_count(&self);

  #[doc(hidden)]
  fn inc_writer_count(&self);

  /// Creates an animation that smoothly transitions a writer's value on every
  /// change.
  ///
  /// Use this when the current state value is already the correct "from"
  /// value. The initial animation start value is `self.get()`.
  fn transition(
    self, transition: impl Transition + 'static,
  ) -> Stateful<Animate<TransitionUncountedState<Self>>>
  where
    Self: Sized + 'static,
    Self::Value: PartialEq + 'static,
  {
    let init = self.get();
    self.transition_with_init(init, transition)
  }

  /// Creates an animation that smoothly transitions a writer's value on every
  /// change, with a specified initial value.
  ///
  /// Use this when the first observed target value should animate from a
  /// custom initial value instead of the current state value.
  ///
  /// Typical case: call `transition_with_init(...)` before binding/writing the
  /// property value, so the first sync can animate from `init_value`.
  fn transition_with_init(
    self, init_value: Self::Value, transition: impl Transition + 'static,
  ) -> Stateful<Animate<TransitionUncountedState<Self>>>
  where
    Self: Sized + 'static,
    Self::Value: PartialEq + 'static,
  {
    let init_trigger = Local::of(self.get());
    let modifies = self.animate_state_modifies();

    let mut animate = Animate::declarer();
    animate
      .with_transition(transition)
      .with_from(init_value)
      .with_state(TransitionUncountedState::new(self));
    let animate = animate.finish();

    // Keep `animate` alive by capturing it in the source subscription closure.
    // `animate.state` holds a count-neutral writer wrapper, so this does not
    // keep writer_count > 0 in a cycle.
    let _ = modifies
      .map({
        let animate = animate.clone_writer();
        move |_| animate.read().state.get()
      })
      .merge(init_trigger)
      .distinct_until_changed()
      .pairwise()
      .subscribe({
        let animate = animate.clone_writer();
        move |(old, _)| {
          animate.write().from = old;
          animate.run();
        }
      });
    animate
  }
}

#[doc(hidden)]
pub struct TransitionUncountedState<S: AnimateState + 'static> {
  state: S,
}

impl<S: AnimateState + 'static> TransitionUncountedState<S> {
  #[inline]
  fn new(state: S) -> Self {
    // Neutralize this wrapper's writer-count contribution.
    state.dec_writer_count();
    Self { state }
  }
}

impl<S: AnimateState + 'static> Drop for TransitionUncountedState<S> {
  fn drop(&mut self) { self.state.inc_writer_count(); }
}

/// A state with a lerp function as an animation state that use the `lerp_fn`
/// function to calc the linearly lerp value by rate, and not require the value
/// type of the state to implement the `Lerp` trait.
///
/// User can use it if the value type of the state is not implement the `Lerp`
/// or override the lerp algorithm of the value type of state.
pub struct CustomLerpState<S, F> {
  lerp_fn: F,
  state: S,
}

pub type LerpFnState<S, F> = CustomLerpState<S, F>;

struct StateWriterAdapter<S>(S);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AnimateStatePackEnd;

#[derive(Clone, Debug, PartialEq)]
pub struct AnimateStatePack<H, T> {
  pub head: H,
  pub tail: T,
}

impl<H, T> AnimateStatePack<H, T> {
  #[inline]
  pub fn new(head: H, tail: T) -> Self { Self { head, tail } }
}

#[macro_export]
macro_rules! animate_state_pack {
  ($head:expr $(,)?) => {
    $crate::animation::AnimateStatePack::new($head, $crate::animation::AnimateStatePackEnd)
  };
  ($head:expr, $($tail:expr),+ $(,)?) => {
    $crate::animation::AnimateStatePack::new($head, $crate::animate_state_pack!($($tail),+))
  };
}
pub use animate_state_pack;

impl<S: AnimateState + 'static> AnimateState for TransitionUncountedState<S> {
  type Value = S::Value;

  #[inline]
  fn get(&self) -> Self::Value { self.state.get() }

  #[inline]
  fn set(&self, v: Self::Value) { self.state.set(v) }

  #[inline]
  fn revert(&self, v: Self::Value) { self.state.revert(v) }

  #[inline]
  fn animate_state_modifies(&self) -> LocalBoxedObservable<'static, ModifyInfo, Infallible> {
    self.state.animate_state_modifies()
  }

  #[inline]
  fn dec_writer_count(&self) { self.state.dec_writer_count(); }

  #[inline]
  fn inc_writer_count(&self) { self.state.inc_writer_count(); }

  #[inline]
  fn calc_lerp_value(&mut self, from: &Self::Value, to: &Self::Value, rate: f32) -> Self::Value {
    self.state.calc_lerp_value(from, to, rate)
  }
}

impl<S> AnimateState for S
where
  S: StateWriter,
  S::Value: Clone + Lerp,
{
  type Value = S::Value;

  #[inline]
  fn get(&self) -> Self::Value { self.read().clone() }

  #[inline]
  fn set(&self, v: Self::Value) { *self.shallow() = v; }

  #[inline]
  fn revert(&self, v: Self::Value) {
    let mut w = self.write();
    *w = v;
    w.forget_modifies();
  }

  #[inline]
  fn animate_state_modifies(&self) -> LocalBoxedObservable<'static, ModifyInfo, Infallible> {
    StateWatcher::raw_modifies(self)
      .filter(|s| s.contains(ModifyEffect::all()))
      .box_it()
  }

  #[inline]
  fn dec_writer_count(&self) { StateWriter::dec_writer_count(self); }

  #[inline]
  fn inc_writer_count(&self) { StateWriter::inc_writer_count(self); }

  fn calc_lerp_value(&mut self, from: &Self::Value, to: &Self::Value, rate: f32) -> Self::Value {
    from.lerp(to, rate)
  }
}

impl<S, F> AnimateState for CustomLerpState<S, F>
where
  S: AnimateState,
  F: FnMut(&S::Value, &S::Value, f32) -> S::Value,
{
  type Value = S::Value;

  #[inline]
  fn get(&self) -> Self::Value { self.state.get() }

  #[inline]
  fn set(&self, v: Self::Value) { self.state.set(v) }

  #[inline]
  fn revert(&self, v: Self::Value) { self.state.revert(v) }

  #[inline]
  fn animate_state_modifies(&self) -> LocalBoxedObservable<'static, ModifyInfo, Infallible> {
    self.state.animate_state_modifies()
  }

  #[inline]
  fn dec_writer_count(&self) { self.state.dec_writer_count(); }

  #[inline]
  fn inc_writer_count(&self) { self.state.inc_writer_count(); }

  #[inline]
  fn calc_lerp_value(&mut self, from: &S::Value, to: &S::Value, rate: f32) -> S::Value {
    (self.lerp_fn)(from, to, rate)
  }
}

impl<S, F> CustomLerpState<S, F>
where
  S: AnimateState,
  F: FnMut(&S::Value, &S::Value, f32) -> S::Value,
{
  #[inline]
  pub fn from_state(state: S, lerp_fn: F) -> Self { Self { state, lerp_fn } }
}

impl<S, F> CustomLerpState<S, F>
where
  S: StateWriter,
  S::Value: Clone,
  F: FnMut(&S::Value, &S::Value, f32) -> S::Value + 'static,
{
  #[inline]
  pub fn from_writer(state: S, lerp_fn: F) -> impl AnimateState<Value = S::Value> {
    CustomLerpState { state: StateWriterAdapter(state), lerp_fn }
  }
}

impl<S> AnimateState for StateWriterAdapter<S>
where
  S: StateWriter,
  S::Value: Clone,
{
  type Value = S::Value;

  #[inline]
  fn get(&self) -> Self::Value { self.0.read().clone() }

  #[inline]
  fn set(&self, v: Self::Value) { *self.0.shallow() = v; }

  #[inline]
  fn revert(&self, v: Self::Value) {
    let mut w = self.0.write();
    *w = v;
    w.forget_modifies();
  }

  #[inline]
  fn animate_state_modifies(&self) -> LocalBoxedObservable<'static, ModifyInfo, Infallible> {
    StateWatcher::raw_modifies(&self.0)
      .filter(|s| s.contains(ModifyEffect::all()))
      .box_it()
  }

  #[inline]
  fn dec_writer_count(&self) { self.0.dec_writer_count(); }

  #[inline]
  fn inc_writer_count(&self) { self.0.inc_writer_count(); }

  #[inline]
  fn calc_lerp_value(&mut self, _from: &Self::Value, _to: &Self::Value, _rate: f32) -> Self::Value {
    unreachable!("StateWriterAdapter only serves as CustomLerpState's storage adapter.")
  }
}

impl Lerp for AnimateStatePackEnd {
  #[inline]
  fn lerp(&self, _: &Self, _: f32) -> Self { AnimateStatePackEnd }
}

impl<H, T> Lerp for AnimateStatePack<H, T>
where
  H: Lerp,
  T: Lerp,
{
  #[inline]
  fn lerp(&self, to: &Self, rate: f32) -> Self {
    AnimateStatePack::new(self.head.lerp(&to.head, rate), self.tail.lerp(&to.tail, rate))
  }
}

impl AnimateState for AnimateStatePackEnd {
  type Value = AnimateStatePackEnd;

  #[inline]
  fn get(&self) -> Self::Value { AnimateStatePackEnd }

  #[inline]
  fn set(&self, _v: Self::Value) {}

  #[inline]
  fn revert(&self, _v: Self::Value) {}

  #[inline]
  fn animate_state_modifies(&self) -> LocalBoxedObservable<'static, ModifyInfo, Infallible> {
    Local::empty()
      .map(|_| -> ModifyInfo { unreachable!() })
      .box_it()
  }

  #[inline]
  fn dec_writer_count(&self) {}

  #[inline]
  fn inc_writer_count(&self) {}

  #[inline]
  fn calc_lerp_value(&mut self, _from: &Self::Value, _to: &Self::Value, _rate: f32) -> Self::Value {
    AnimateStatePackEnd
  }
}

impl<H, T> AnimateState for AnimateStatePack<H, T>
where
  H: AnimateState,
  T: AnimateState,
{
  type Value = AnimateStatePack<H::Value, T::Value>;

  #[inline]
  fn get(&self) -> Self::Value { AnimateStatePack::new(self.head.get(), self.tail.get()) }

  #[inline]
  fn set(&self, v: Self::Value) {
    self.head.set(v.head);
    self.tail.set(v.tail);
  }

  #[inline]
  fn revert(&self, v: Self::Value) {
    self.head.revert(v.head);
    self.tail.revert(v.tail);
  }

  #[inline]
  fn animate_state_modifies(&self) -> LocalBoxedObservable<'static, ModifyInfo, Infallible> {
    Local::from_iter([self.head.animate_state_modifies(), self.tail.animate_state_modifies()])
      .merge_all(usize::MAX)
      .box_it()
  }

  #[inline]
  fn dec_writer_count(&self) {
    self.head.dec_writer_count();
    self.tail.dec_writer_count();
  }

  #[inline]
  fn inc_writer_count(&self) {
    self.head.inc_writer_count();
    self.tail.inc_writer_count();
  }

  #[inline]
  fn calc_lerp_value(&mut self, from: &Self::Value, to: &Self::Value, rate: f32) -> Self::Value {
    AnimateStatePack::new(
      self
        .head
        .calc_lerp_value(&from.head, &to.head, rate),
      self
        .tail
        .calc_lerp_value(&from.tail, &to.tail, rate),
    )
  }
}

#[cfg(test)]
mod tests {
  use crate::{prelude::*, reset_test_env};

  #[test]
  fn pack_two() {
    reset_test_env!();
    let mut group = animate_state_pack!(Stateful::new(1.), Stateful::new(2.));
    let half = group.calc_lerp_value(&animate_state_pack!(0., 0.), &group.get(), 0.5);
    assert_eq!(half, animate_state_pack!(0.5, 1.));
  }

  #[test]
  fn transition_with_init_drop_no_cycle() {
    reset_test_env!();

    let state = Stateful::new(0);
    let w = fn_widget! {
      let _animate = state.clone_writer().transition_with_init(
        0,
        EasingTransition { easing: easing::LINEAR, duration: Duration::ZERO },
      );
      @Void {}
    };
    let wnd = crate::test_helper::TestWindow::from_widget(w);
    wnd.draw_frame();
    drop(wnd);
    AppCtx::run_until_stalled();
  }

  #[test]
  fn transition_with_init_part_writer_drop_no_cycle() {
    reset_test_env!();

    let state = Stateful::new((0, 0));
    let w = fn_widget! {
      let part = state.clone_writer().part_writer("0".into(), |v| PartMut::new(&mut v.0));
      let _animate = part.transition_with_init(
        0,
        EasingTransition { easing: easing::LINEAR, duration: Duration::ZERO },
      );
      @Void {}
    };
    let wnd = crate::test_helper::TestWindow::from_widget(w);
    wnd.draw_frame();
    drop(wnd);
    AppCtx::run_until_stalled();
  }
}
