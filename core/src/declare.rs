use crate::{context::BuildCtx, prelude::Pipe, state::ModifyScope};
use rxrust::ops::box_it::BoxOp;
use std::convert::Infallible;

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
  /// build the object with the given context, return the object and not care
  /// about if this object is subscribed to other or not.
  fn build_declare(self, ctx: &BuildCtx) -> Self::Target;
}

/// The type use to store the init value of the field when declare a object.
pub enum DeclareInit<V> {
  Value(V),
  Pipe(Pipe<V>),
}

type ValueStream<V> = BoxOp<'static, (ModifyScope, V), Infallible>;

impl<V: 'static> DeclareInit<V> {
  pub fn unzip(self) -> (V, Option<ValueStream<V>>) {
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

impl<V: 'static, U: From<V> + 'static> DeclareFrom<Pipe<V>, Pipe<()>> for DeclareInit<U> {
  #[inline]
  fn declare_from(value: Pipe<V>) -> Self { Self::Pipe(value.map(U::from)) }
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
