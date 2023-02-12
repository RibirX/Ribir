mod readonly;
mod stateful;
use std::rc::Rc;

pub use readonly::*;
use rxrust::prelude::ObservableItem;
pub use stateful::*;

use crate::dynamic_widget::DynWidget;

/// Enum to store both stateless and stateful object.
pub enum State<W> {
  Stateless(W),
  Stateful(Stateful<W>),
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

impl<W> From<W> for State<W> {
  #[inline]
  fn from(w: W) -> Self { State::Stateless(w) }
}

impl<W> From<Stateful<W>> for State<W> {
  #[inline]
  fn from(w: Stateful<W>) -> Self { State::Stateful(w) }
}

impl<D: 'static> From<Stateful<DynWidget<D>>> for State<D> {
  fn from(value: Stateful<DynWidget<D>>) -> Self {
    let c_value = value.clone();
    let v = value.silent_ref().dyns.take().unwrap();
    let v = Stateful::new(v);
    let c_v = v.clone();
    value.modifies().subscribe(move |_| {
      if c_value.silent_ref().dyns.is_some() {
        *c_v.state_ref() = c_value.silent_ref().dyns.take().unwrap();
      }
    });
    v.into()
  }
}
