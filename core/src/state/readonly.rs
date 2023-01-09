use std::rc::Rc;

use rxrust::{observable, ops::box_it::LocalBoxOp, prelude::Observable, subject::LocalSubject};

use super::{ModifyScope, StateRef, Stateful};

pub enum Readonly<W> {
  Stateful(Stateful<W>),
  Stateless(Rc<W>),
}

pub enum ReadRef<'a, W> {
  Stateful(StateRef<'a, W>),
  Stateless(&'a Rc<W>),
}

impl<W> Readonly<W> {
  /// Readonly never modify the inner data, use a `()` to mock the api.
  #[inline]
  pub fn modify_guard(&self) {}

  #[inline]
  pub fn state_ref(&self) -> ReadRef<W> {
    match self {
      Readonly::Stateful(s) => ReadRef::Stateful(s.state_ref()),
      Readonly::Stateless(w) => ReadRef::Stateless(w),
    }
  }

  #[inline]
  pub fn silent_ref(&self) -> ReadRef<W> {
    // For a read only reference, `state_ref` and `silent_ref` no difference.
    self.state_ref()
  }

  pub fn raw_modifies(&self) -> LocalSubject<'static, ModifyScope, ()> {
    match self {
      Readonly::Stateful(s) => s.raw_modifies(),
      Readonly::Stateless(_) => LocalSubject::default(),
    }
  }

  /// Notify when this widget be mutable accessed, no mather if the widget
  /// really be modified, the value is hint if it's only access by silent ref.
  #[inline]
  pub fn modifies(&self) -> LocalBoxOp<'static, (), ()> {
    match self {
      Readonly::Stateful(s) => s.modifies(),
      Readonly::Stateless(_) => observable::create(|_| {}).box_it(),
    }
  }

  /// Clone the stateful widget of which the reference point to. Require mutable
  /// reference because we try to early release inner borrow when clone occur.
  #[inline]
  pub fn clone_stateful(&self) -> Readonly<W> { self.clone() }
}

impl<'a, W> ReadRef<'a, W> {
  #[inline]
  pub fn silent(&self) -> &W { &**self }

  #[inline]
  pub fn raw_modifies(&self) -> LocalSubject<'static, ModifyScope, ()> {
    match self {
      ReadRef::Stateful(s) => s.raw_modifies(),
      ReadRef::Stateless(_) => LocalSubject::default(),
    }
  }

  #[inline]
  pub fn modifies(&self) -> LocalBoxOp<'static, (), ()> {
    match self {
      ReadRef::Stateful(s) => s.modifies(),
      ReadRef::Stateless(_) => observable::create(|_| {}).box_it(),
    }
  }

  pub fn clone_stateful(&self) -> Readonly<W> {
    match self {
      ReadRef::Stateful(s) => Readonly::Stateful(s.clone_stateful()),
      ReadRef::Stateless(r) => Readonly::Stateless((*r).clone()),
    }
  }
}

impl<W> Clone for Readonly<W> {
  fn clone(&self) -> Self {
    match self {
      Self::Stateful(arg0) => Self::Stateful(arg0.clone()),
      Self::Stateless(arg0) => Self::Stateless(arg0.clone()),
    }
  }
}

impl<'a, W> std::ops::Deref for ReadRef<'a, W> {
  type Target = W;

  fn deref(&self) -> &Self::Target {
    match self {
      ReadRef::Stateful(s) => s.deref(),
      ReadRef::Stateless(r) => r,
    }
  }
}
