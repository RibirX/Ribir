use crate::{context::BuildCtx, prelude::Pipe};
use rxrust::ops::box_it::BoxOp;
use std::convert::Infallible;

pub trait Declare {
  type Builder: DeclareBuilder;
  fn declare_builder() -> Self::Builder;
}

/// The next version of `Declare` trait. It will replace the `Declare` trait
/// after it is stable.
pub trait Declare2 {
  type Builder: DeclareBuilder;
  fn declare2_builder() -> Self::Builder;
}

/// widget builder use to construct a widget in  `widget!`. See the [mod level
/// document](declare) to know how to use it.
pub trait DeclareBuilder {
  type Target;
  fn build(self, ctx: &BuildCtx) -> Self::Target;
}

/// The type use to store the init value of the field when declare a object.
pub enum DeclareInit<V> {
  Value(V),
  Pipe(Pipe<V>),
}

impl<V: 'static> DeclareInit<V> {
  pub fn unzip(self) -> (V, Option<BoxOp<'static, V, Infallible>>) {
    match self {
      Self::Value(v) => (v, None),
      Self::Pipe(v) => {
        let (v, pipe) = v.unzip();
        (v, Some(pipe))
      }
    }
  }

  pub fn value(&self) -> &V {
    match self {
      Self::Value(v) => v,
      Self::Pipe(v) => v.value(),
    }
  }

  pub fn value_mut(&mut self) -> &mut V {
    match self {
      Self::Value(v) => v,
      Self::Pipe(v) => v.value_mut(),
    }
  }
}

impl<T: Default> Default for DeclareInit<T> {
  #[inline]
  fn default() -> Self { Self::Value(T::default()) }
}

pub trait DeclareFrom<V, M> {
  fn declare_from(value: V) -> Self;
}

impl<V, U: From<V>> DeclareFrom<V, ()> for DeclareInit<U> {
  #[inline]
  fn declare_from(value: V) -> Self { Self::Value(value.into()) }
}

impl<V, U: From<V>> DeclareFrom<V, Option<()>> for DeclareInit<Option<U>> {
  #[inline]
  fn declare_from(value: V) -> Self { Self::Value(Some(value.into())) }
}

impl<V: 'static, U: From<V> + 'static> DeclareFrom<Pipe<V>, Pipe<()>> for DeclareInit<U> {
  #[inline]
  fn declare_from(value: Pipe<V>) -> Self { Self::Pipe(value.map(U::from)) }
}

impl<V: 'static, U: From<V> + 'static> DeclareFrom<Pipe<V>, Option<Pipe<()>>>
  for DeclareInit<Option<U>>
{
  #[inline]
  fn declare_from(value: Pipe<V>) -> Self { Self::Pipe(value.map(|v| Some(v.into()))) }
}

#[derive(Debug, PartialEq, Hash)]
pub struct DeclareStripOption<O>(O);

impl<V> From<V> for DeclareStripOption<Option<V>> {
  #[inline]
  fn from(value: V) -> Self { Self(Some(value)) }
}

impl<V> From<Option<V>> for DeclareStripOption<Option<V>> {
  #[inline]
  fn from(value: Option<V>) -> Self { Self(value) }
}

impl<V> DeclareStripOption<Option<V>> {
  #[inline]
  pub fn into_option_value(self) -> Option<V> { self.0 }
}

#[cfg(test)]
mod tests {
  use super::*;
  use ribir_painter::{Brush, Color};

  #[test]
  fn inner_value_into() {
    assert_eq!(
      DeclareStripOption::from(Brush::from(Color::RED)),
      DeclareStripOption(Some(Brush::from(Color::RED)))
    );
  }

  #[test]
  fn option_self_can_use_with_strip() {
    assert_eq!(
      DeclareStripOption::from(Some(Brush::from(Color::RED))),
      DeclareStripOption(Some(Brush::from(Color::RED)))
    )
  }
}

/// struct help the generate code have better type hint.
#[derive(Clone)]
pub struct DeclareFieldValue<F>(F);

impl<R, F> DeclareFieldValue<F>
where
  F: FnMut() -> R,
{
  #[inline]
  pub fn new(f: F) -> Self { Self(f) }

  #[inline]
  pub fn value(&mut self) -> R { (self.0)() }
}
