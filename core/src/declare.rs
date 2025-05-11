use std::convert::Infallible;

use rxrust::ops::box_it::BoxOp;

use crate::prelude::*;

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
  fn finish(self) -> Self::Target;
}

/// A enum that represents a value that can be either a value or a pipe.
pub enum PipeValue<V> {
  Value(V),
  Pipe(BoxPipe<V>),
}

pub type ValueStream<V> = BoxOp<'static, (ModifyScope, V), Infallible>;

impl<V: 'static> PipeValue<V> {
  pub fn unzip(self) -> (V, Option<ValueStream<V>>) {
    match self {
      Self::Value(v) => (v, None),
      Self::Pipe(v) => {
        let (v, pipe) = v.into_pipe().unzip(ModifyScope::DATA, None);
        (v, Some(pipe))
      }
    }
  }

  pub fn map<F, U: 'static>(self, f: F) -> PipeValue<U>
  where
    F: Fn(V) -> U + 'static,
  {
    match self {
      Self::Value(v) => PipeValue::Value(f(v)),
      Self::Pipe(v) => v.into_pipe().map(f).r_into(),
    }
  }
}

impl<T: Default> Default for PipeValue<T> {
  fn default() -> Self { Self::Value(T::default()) }
}

pub struct ValueKind<K: ?Sized>(PhantomData<fn() -> K>);
impl<T: RInto<V, K>, V, K: ?Sized> RFrom<T, ValueKind<K>> for PipeValue<V> {
  fn r_from(value: T) -> Self { Self::Value(value.r_into()) }
}

impl<P, V: 'static, K: ?Sized + 'static> RFrom<P, dyn Pipe<Value = K>> for PipeValue<V>
where
  P: Pipe<Value: RInto<V, K>>,
{
  fn r_from(value: P) -> Self {
    let pipe = Box::new(value.map(RInto::r_into));
    Self::Pipe(BoxPipe::pipe(pipe))
  }
}
