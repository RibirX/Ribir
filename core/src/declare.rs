use crate::{context::BuildCtx, pipe::Pipe, prelude::BoxPipe, state::ModifyScope};
use rxrust::ops::box_it::BoxOp;
use std::convert::Infallible;

/// The next version of `Declare` trait. It will replace the `Declare` trait
/// after it is stable.
pub trait Declare {
  type Builder: DeclareBuilder;
  fn declare_builder() -> Self::Builder;
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
  Pipe(BoxPipe<V>),
}

pub type ValueStream<V> = BoxOp<'static, (ModifyScope, V), Infallible>;

impl<V: 'static> DeclareInit<V> {
  pub fn unzip(self) -> (V, Option<ValueStream<V>>) {
    match self {
      Self::Value(v) => (v, None),
      Self::Pipe(v) => {
        let (v, pipe) = v.into_pipe().unzip();
        (v, Some(pipe))
      }
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

impl<P: Pipe + 'static, V> DeclareFrom<P, &dyn Pipe<Value = ()>> for DeclareInit<V>
where
  V: From<P::Value> + 'static,
{
  #[inline]
  fn declare_from(value: P) -> Self {
    let pipe = Box::new(value.map(Into::into));
    Self::Pipe(BoxPipe::pipe(pipe))
  }
}
