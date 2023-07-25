mod readonly;
mod stateful;
use std::rc::Rc;

pub use readonly::*;
use rxrust::prelude::ObservableItem;
pub use stateful::*;

use crate::{
  context::BuildCtx,
  dynamic_widget::DynWidget,
  widget::{Compose, WidgetBuilder, WidgetId},
};

/// Enum to store both stateless and stateful object.
#[derive(Clone)]
pub enum State<W> {
  Stateless(W),
  Stateful(Stateful<W>),
}

impl<C: Compose> WidgetBuilder for State<C> {
  #[inline]
  fn build(self, ctx: &BuildCtx) -> WidgetId { Compose::compose(self).build(ctx) }
}

impl<W> State<W> {
  pub fn into_writable(self) -> Stateful<W> {
    match self {
      State::Stateless(w) => Stateful::new(w),
      State::Stateful(w) => w,
    }
  }

  pub fn into_readonly(self) -> Readonly<W> {
    match self {
      State::Stateless(w) => Readonly::Stateless(Rc::new(w)),
      State::Stateful(w) => match w.try_into_inner() {
        Ok(w) => Readonly::Stateless(Rc::new(w)),
        Err(s) => Readonly::Stateful(s),
      },
    }
  }
}

pub(crate) trait StateFrom<V> {
  fn state_from(value: V) -> Self;
}

impl<W> StateFrom<W> for State<W> {
  #[inline]
  fn state_from(value: W) -> State<W> { State::Stateless(value) }
}

impl<W> StateFrom<Stateful<W>> for State<W> {
  #[inline]
  fn state_from(value: Stateful<W>) -> State<W> { State::Stateful(value) }
}

impl<W: 'static> StateFrom<Stateful<DynWidget<W>>> for State<W> {
  #[inline]
  fn state_from(value: Stateful<DynWidget<W>>) -> State<W> {
    let c_value = value.clone();
    let v = value.silent_ref().dyns.take().unwrap();
    let v = Stateful::new(v);
    let c_v = v.clone();
    value.modifies().subscribe(move |_| {
      if c_value.silent_ref().dyns.is_some() {
        let mut c_value = c_value.silent_ref();
        *c_v.state_ref() = c_value.dyns.take().unwrap();

        // In this widget, we subscribed the `child` modifies, then spread it.
        // When we spread it, we modifies it, a circular occur. So we forget
        // the modify of take its children to break the circular.
        //
        // In other side, `child` is a stateful dynamic widget and use as
        // child here, and all its content all a black box, so others
        // should not depends on it.
        c_value.forget_modifies();
      }
    });
    State::Stateful(v)
  }
}

impl<W, T> From<T> for State<W>
where
  Self: StateFrom<T>,
{
  fn from(value: T) -> Self { StateFrom::state_from(value) }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn fix_dyn_widget_to_state_circular_mut_borrow_panic() {
    let dyn_widget = Stateful::new(DynWidget { dyns: Some(1) });
    let c_dyns = dyn_widget.clone();
    let _: State<i32> = dyn_widget.into();
    {
      c_dyns.state_ref().dyns = Some(2);
    }
  }
}
