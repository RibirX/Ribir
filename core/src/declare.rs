use std::convert::Infallible;

use rxrust::ops::box_it::BoxOp;

use crate::{context::BuildCtx, pipe::Pipe, prelude::BoxPipe, state::ModifyScope};

/// Trait used to create a widget declarer that can interact with the `BuildCtx`
/// to create a widget.
pub trait Declare {
  type Builder: ObjDeclarer;
  fn declarer() -> Self::Builder;
}

/// An object declarer is a type that can be used to create a object with the
/// given context.
pub trait ObjDeclarer {
  type Target;
  /// Finish the object creation with the given context.
  fn finish(self, ctx: &BuildCtx) -> Self::Target;
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

impl<P, V> DeclareFrom<P, &dyn Pipe<Value = ()>> for DeclareInit<V>
where
  P: Pipe + 'static,
  V: From<P::Value> + 'static,
{
  #[inline]
  fn declare_from(value: P) -> Self {
    let pipe = Box::new(value.map(Into::into));
    Self::Pipe(BoxPipe::pipe(pipe))
  }
}
