use std::convert::Infallible;

use rxrust::{observable::ObservableExt, ops::box_it::BoxOp, prelude::BoxIt};

use super::*;
use crate::prelude::*;

/// Trait to help animate update the state.
pub trait AnimateStateSetter {
  type Value: Clone;

  fn get(&self) -> Self::Value;
  fn set(&self, v: Self::Value);
  fn revert(&self, v: Self::Value);
  fn animate_state_modifies(&self) -> BoxOp<'static, ModifyInfo, Infallible>;
}

pub trait AnimateState: AnimateStateSetter {
  fn calc_lerp_value(&mut self, from: &Self::Value, to: &Self::Value, rate: f32) -> Self::Value;

  /// Creates an animation that smoothly transitions a writer's value on every
  /// change.
  fn transition(self, transition: impl Transition + 'static) -> Stateful<Animate<Self>>
  where
    Self: Sized,
    Self::Value: PartialEq,
  {
    let init_value = observable::of(self.get());

    let mut animate = Animate::declarer();
    animate
      .with_transition(transition)
      .with_from(self.get())
      .with_state(self);
    let animate = animate.finish();

    // fixme: circle reference here
    let modifies = animate.read().state.animate_state_modifies();
    modifies
      .map({
        let animate = animate.clone_writer();
        move |_| animate.read().state.get()
      })
      .merge(init_value)
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

/// A state with a lerp function as an animation state that use the `lerp_fn`
/// function to calc the linearly lerp value by rate, and not require the value
/// type of the state to implement the `Lerp` trait.
///
/// User can use it if the value type of the state is not implement the `Lerp`
/// or override the lerp algorithm of the value type of state.
pub struct LerpFnState<S, F> {
  lerp_fn: F,
  state: S,
}

impl<S> AnimateStateSetter for S
where
  S: StateWriter,
  S::Value: Clone,
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
  fn animate_state_modifies(&self) -> BoxOp<'static, ModifyInfo, Infallible> {
    StateWatcher::raw_modifies(self)
      .filter(|s| s.contains(ModifyEffect::all()))
      .box_it()
  }
}

impl<S> AnimateState for S
where
  S: StateWriter,
  S::Value: Clone + Lerp,
{
  fn calc_lerp_value(&mut self, from: &Self::Value, to: &Self::Value, rate: f32) -> Self::Value {
    from.lerp(to, rate)
  }
}

impl<S, F> AnimateStateSetter for LerpFnState<S, F>
where
  S: AnimateStateSetter,
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
  fn animate_state_modifies(&self) -> BoxOp<'static, ModifyInfo, Infallible> {
    self.state.animate_state_modifies()
  }
}

impl<S, F> AnimateState for LerpFnState<S, F>
where
  S: AnimateStateSetter,
  F: FnMut(&S::Value, &S::Value, f32) -> S::Value,
{
  #[inline]
  fn calc_lerp_value(&mut self, from: &S::Value, to: &S::Value, rate: f32) -> S::Value {
    (self.lerp_fn)(from, to, rate)
  }
}

impl<S, F> LerpFnState<S, F>
where
  S: AnimateStateSetter,
  F: FnMut(&S::Value, &S::Value, f32) -> S::Value,
{
  #[inline]
  pub fn new(state: S, lerp_fn: F) -> Self { Self { state, lerp_fn } }
}

macro_rules! impl_animate_state_for_tuple {
  (@($($tuple: tt), *) $next: tt $(, $rest: tt)*) => {
    impl_animate_state_for_tuple!(@($($tuple),*));
    impl_animate_state_for_tuple!(@($($tuple,)* $next) $($rest),*);
  };

  (@($($tuple: tt),*)) => {
    paste::paste!{
      impl<$([<S $tuple>]), *> AnimateStateSetter for ($([<S $tuple>]), *)
      where
        $([<S $tuple>]: AnimateStateSetter), *
      {
        type Value = ($([<S $tuple>]::Value), *);

        fn get(&self) -> Self::Value {
          ($(self.$tuple.get()),*)
        }


        fn set(&self, v: Self::Value) {
          $(self.$tuple.set(v.$tuple);) *
        }

        fn revert(&self, v: Self::Value) {
          $(self.$tuple.revert(v.$tuple);) *
        }

        fn animate_state_modifies(&self) -> BoxOp<'static, ModifyInfo, Infallible> {
          rxrust::observable::from_iter([$(self.$tuple.animate_state_modifies()), *])
            .merge_all(usize::MAX)
            .box_it()
        }
      }

      impl<$([<S $tuple>]), *> AnimateState for ($([<S $tuple>]), *)
      where
        $([<S $tuple>]: AnimateState), *
      {
        #[inline]
        fn calc_lerp_value(
          &mut self,
          from: &<Self as AnimateStateSetter>::Value,
          to: &<Self as AnimateStateSetter>::Value,
          rate: f32
        ) -> <Self as AnimateStateSetter>::Value
        {
          (
            $(self.$tuple.calc_lerp_value(&from.$tuple, &to.$tuple, rate),) *
          )
        }
      }
    }
  };
  ($t1: tt, $t2: tt $(, $t: tt)*) => {
    impl_animate_state_for_tuple!(@($t1, $t2) $($t),*);
  };
}

impl_animate_state_for_tuple!(0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15);

#[cfg(test)]
mod tests {
  use crate::{prelude::*, reset_test_env};

  #[test]
  fn group_two() {
    reset_test_env!();
    let mut group = (Stateful::new(1.), Stateful::new(2.));
    let half = group.calc_lerp_value(&(0., 0.), &group.get(), 0.5);
    assert_eq!(half, (0.5, 1.));
  }
}
