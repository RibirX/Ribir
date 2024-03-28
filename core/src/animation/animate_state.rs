use std::convert::Infallible;

use rxrust::{observable::ObservableExt, ops::box_it::BoxOp, prelude::BoxIt};

use super::*;
use crate::state::{ModifyScope, StateWatcher, StateWriter};

/// Trait to help animate update the state.
pub trait AnimateStateSetter {
  type C: AnimateStateSetter<Value = Self::Value>;
  type Value: Clone;

  fn get(&self) -> Self::Value;
  fn set(&self, v: Self::Value);
  fn animate_state_modifies(&self) -> BoxOp<'static, ModifyScope, Infallible>;
  fn clone_setter(&self) -> Self::C;
}

/// Trait to help animate calc the lerp value.
pub trait AnimateState: AnimateStateSetter {
  fn calc_lerp_value(&mut self, from: &Self::Value, to: &Self::Value, rate: f32) -> Self::Value;
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
  type C = S::Writer;
  type Value = S::Value;

  #[inline]
  fn get(&self) -> Self::Value { self.read().clone() }

  #[inline]
  fn set(&self, v: Self::Value) { *self.shallow() = v; }

  #[inline]
  fn clone_setter(&self) -> Self::C { self.clone_writer() }

  #[inline]
  fn animate_state_modifies(&self) -> BoxOp<'static, ModifyScope, Infallible> {
    StateWatcher::modifies(self)
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
{
  type C = S::C;
  type Value = S::Value;

  #[inline]
  fn get(&self) -> Self::Value { self.state.get() }

  #[inline]
  fn set(&self, v: Self::Value) { self.state.set(v) }

  #[inline]
  fn clone_setter(&self) -> Self::C { self.state.clone_setter() }

  #[inline]
  fn animate_state_modifies(&self) -> BoxOp<'static, ModifyScope, Infallible> {
    self.state.animate_state_modifies()
  }
}

impl<S, F, V> AnimateState for LerpFnState<S, F>
where
  S: AnimateStateSetter<Value = V>,
  F: FnMut(&V, &V, f32) -> V,
{
  #[inline]
  fn calc_lerp_value(&mut self, from: &V, to: &V, rate: f32) -> V { (self.lerp_fn)(from, to, rate) }
}

impl<V, S, F> LerpFnState<S, F>
where
  S: AnimateStateSetter<Value = V>,
  F: FnMut(&V, &V, f32) -> V,
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
        type C = ($([<S $tuple>]::C), *);
        type Value = ($([<S $tuple>]::Value), *);

        fn get(&self) -> Self::Value {
          ($(self.$tuple.get()),*)
        }


        fn set(&self, v: Self::Value) {
          $(self.$tuple.set(v.$tuple);) *
        }

        #[inline]
        fn clone_setter(&self) -> Self::C {
          ( $(self.$tuple.clone_setter()),*)
        }

        fn animate_state_modifies(&self) -> BoxOp<'static, ModifyScope, Infallible> {
          rxrust::observable::from_iter([$(self.$tuple.animate_state_modifies()), *])
            .merge_all(usize::MAX)
            .box_it()
        }
      }

      impl<$([<S $tuple>]), *> AnimateState for ($([<S $tuple>]),*)
      where
        $([<S $tuple>]: AnimateState,)*
      {
        #[inline]
        fn calc_lerp_value(
          &mut self,
          from: &Self::Value,
          to: &Self::Value,
          rate: f32
        ) -> Self::Value
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
    let mut group = (State::value(1.), State::value(2.));
    let half = group.calc_lerp_value(&(0., 0.), &group.get(), 0.5);
    assert_eq!(half, (0.5, 1.));
  }
}
