mod readonly;
mod stateful;
use std::rc::Rc;

pub use readonly::*;
use rxrust::prelude::SubscribeNext;
pub use stateful::*;

use crate::dynamic_widget::DynWidget;

/// Enum to store both stateless and stateful object.
pub enum State<W> {
  Stateless(W),
  Stateful(Stateful<W>),
}

impl<W> From<W> for State<W> {
  #[inline]
  fn from(w: W) -> Self { State::Stateless(w) }
}

impl<W> From<Stateful<W>> for State<W> {
  #[inline]
  fn from(w: Stateful<W>) -> Self { State::Stateful(w) }
}

impl<W: IntoStateful> State<W> {
  pub fn into_writable(self) -> Stateful<W> {
    match self {
      State::Stateless(w) => w.into_stateful(),
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

impl<D: 'static> From<Stateful<DynWidget<D>>> for State<D> {
  fn from(value: Stateful<DynWidget<D>>) -> Self {
    let c_value = value.clone();
    let v = value.silent_ref().dyns.take().unwrap();
    let v = v.into_stateful();
    let c_v = v.clone();
    value.modifies().subscribe(move |_| {
      if let Some(v) = c_value.silent_ref().dyns.take() {
        *c_v.state_ref() = v;
      }
    });
    v.into()
  }
}

impl<D: 'static> From<DynWidget<D>> for State<D> {
  #[inline]
  fn from(value: DynWidget<D>) -> Self { State::Stateless(value.into_inner()) }
}
