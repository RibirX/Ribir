use std::any::{Any, TypeId};

use smallvec::SmallVec;

use crate::state::{
  MapReader, MapWriterAsReader, PartData, ReadRef, Reader, StateReader, StateWriter, WriteRef,
};

/// A type can composed by many types, this trait help us to query the type and
/// the inner type by type id.
pub trait Query: Any {
  /// Queries all types that match the provided type id, returning their handles
  /// in an inside-to-outside order.
  fn query_all(&self, type_id: TypeId) -> SmallVec<[QueryHandle; 1]>;

  /// Queries the type that matches the provided type id, returning its handle.
  /// This method always returns the outermost type.
  fn query(&self, type_id: TypeId) -> Option<QueryHandle>;

  /// Hint this is a non-queryable type.
  fn queryable(&self) -> bool { true }
}

/// This wrapper transforms a non-queryable type into a queryable one, limiting
/// query access to its own type only.
///
/// If a state writer, such as `State<i32>`, is provided, the `State<i32>` can
/// be queried, but the `i32` cannot. Typically, there is no need to wrap
/// a state writer with `Queryable` since the state writer is already inherently
/// queryable.
pub struct Queryable<T: Any>(pub T);

/// A dynamic handle to a query result of a data, so we can use it in a trait
/// object.
pub struct QueryHandle<'a>(InnerHandle<'a>);

/// A reference to a query result of a data, it's similar to `&T`.
pub struct QueryRef<'a, T: ?Sized> {
  pub(crate) type_ref: &'a T,
  pub(crate) _data: Option<Box<dyn Any>>,
}

impl<'a> QueryHandle<'a> {
  /// Downcast the to type `T` and returns a reference to it,
  /// return `None` if the type not match
  pub fn downcast_ref<T: Any>(&self) -> Option<&T> {
    match self.0 {
      InnerHandle::Ref(r) => r.downcast_ref::<T>(),
      InnerHandle::Owned(ref o) => o
        .downcast_ref::<ReadRef<'static, dyn Any>>()
        .and_then(|r| r.downcast_ref::<T>())
        .or_else(|| {
          o.downcast_ref::<WriteRef<'static, dyn Any>>()
            .and_then(|w| w.downcast_ref::<T>())
        }),
    }
  }

  /// Attempts to downcast to type `T` and returns a mutable reference
  /// to it. If the types do not match, it returns `None`.
  ///
  /// This method can only return a mutable reference if the handle points
  /// to a `WriteRef`. This is because in Ribir, the final tree is immutable by
  /// default. Any modifications to the tree can only be made through the
  /// `WriteRef` of the `StateWriter`.
  pub fn downcast_mut<T: Any>(&mut self) -> Option<&mut T> {
    let InnerHandle::Owned(ref mut o) = self.0 else {
      return None;
    };

    o.downcast_mut::<WriteRef<'static, dyn Any>>()?
      .downcast_mut::<T>()
  }

  pub(crate) fn new(r: &'a dyn Any) -> Self { QueryHandle(InnerHandle::Ref(r)) }

  pub(crate) fn from_read_ref(r: ReadRef<'a, dyn Any>) -> Self {
    // Safety: The lifetime is maintained in the return handle and will be shortened
    // once the handle is downcast.
    let r: ReadRef<'static, dyn Any> = unsafe { std::mem::transmute(r) };
    QueryHandle(InnerHandle::Owned(Box::new(r)))
  }

  pub(crate) fn from_write_ref(w: WriteRef<'a, dyn Any>) -> Self {
    // Safety: The lifetime is maintained in the return handle and will be shortened
    // once the handle is downcast.
    let w: WriteRef<'static, dyn Any> = unsafe { std::mem::transmute(w) };
    QueryHandle(InnerHandle::Owned(Box::new(w)))
  }

  pub fn into_ref<T: Any>(self) -> Option<QueryRef<'a, T>> {
    match self.0 {
      InnerHandle::Ref(r) if r.type_id() == TypeId::of::<T>() => {
        Some(QueryRef { type_ref: r.downcast_ref::<T>().unwrap(), _data: None })
      }
      InnerHandle::Owned(o) => {
        let inner = o
          .downcast_ref::<ReadRef<'static, dyn Any>>()
          .and_then(|r| r.downcast_ref::<T>())
          .or_else(|| {
            o.downcast_ref::<WriteRef<'static, dyn Any>>()
              .and_then(|w| w.downcast_ref::<T>())
          })?;
        let type_ref = unsafe { &*(inner as *const T) };
        Some(QueryRef { type_ref, _data: Some(o) })
      }
      _ => None,
    }
  }

  pub fn into_mut<T: Any>(self) -> Option<WriteRef<'a, T>> {
    let InnerHandle::Owned(o) = self.0 else {
      return None;
    };

    let w = *o.downcast::<WriteRef<'static, dyn Any>>().ok()?;
    WriteRef::filter_map(w, |v| v.downcast_mut::<T>().map(PartData::from_ref_mut)).ok()
  }
}

enum InnerHandle<'a> {
  Ref(&'a dyn Any),
  Owned(Box<dyn Any>),
}

impl<'a, T: ?Sized> std::ops::Deref for QueryRef<'a, T> {
  type Target = T;

  fn deref(&self) -> &Self::Target { self.type_ref }
}

impl<T: Any> Query for Queryable<T> {
  fn query_all(&self, type_id: TypeId) -> smallvec::SmallVec<[QueryHandle; 1]> {
    self.query(type_id).into_iter().collect()
  }

  fn query(&self, type_id: TypeId) -> Option<QueryHandle> {
    (type_id == self.0.type_id()).then(|| QueryHandle::new(&self.0))
  }

  fn queryable(&self) -> bool { true }
}

impl<T: StateWriter + 'static> Query for T
where
  T::Value: 'static + Sized,
{
  fn query_all(&self, type_id: TypeId) -> smallvec::SmallVec<[QueryHandle; 1]> {
    // The value of the writer and the writer itself cannot be queried
    // at the same time.
    self.query(type_id).into_iter().collect()
  }

  fn query(&self, type_id: TypeId) -> Option<QueryHandle> {
    if type_id == TypeId::of::<T::Value>() {
      let w = WriteRef::map(self.write(), |v| PartData::from_ref(v as &dyn Any));
      Some(QueryHandle::from_write_ref(w))
    } else if type_id == self.type_id() {
      Some(QueryHandle::new(self))
    } else {
      None
    }
  }

  fn queryable(&self) -> bool { true }
}

macro_rules! impl_query_for_reader {
  () => {
    // The value of the reader and the reader itself cannot be queried
    // at the same time.
    fn query_all(&self, type_id: TypeId) -> SmallVec<[QueryHandle; 1]> {
      self.query(type_id).into_iter().collect()
    }

    fn query(&self, type_id: TypeId) -> Option<QueryHandle> {
      if type_id == TypeId::of::<V>() {
        let r = ReadRef::map(self.read(), |v| PartData::from_ref(v as &dyn Any));
        Some(QueryHandle::from_read_ref(r))
      } else if type_id == self.type_id() {
        Some(QueryHandle::new(self))
      } else {
        None
      }
    }

    fn queryable(&self) -> bool { true }
  };
}

impl<S, F, V> Query for MapReader<S, F>
where
  Self: StateReader<Value = V>,
  V: Any,
{
  impl_query_for_reader!();
}

impl<S, F, V> Query for MapWriterAsReader<S, F>
where
  Self: StateReader<Value = V>,
  V: Any,
{
  impl_query_for_reader!();
}

impl<V: Any> Query for Reader<V> {
  impl_query_for_reader!();
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{reset_test_env, state::State};

  #[test]
  fn query_ref() {
    reset_test_env!();

    struct X;
    let x = Queryable(X);
    let mut h = x.query(TypeId::of::<X>()).unwrap();
    assert!(h.downcast_ref::<X>().is_some());
    assert!(h.downcast_mut::<X>().is_none());
    assert!(h.into_ref::<X>().is_some());
    let h = x.query(TypeId::of::<X>()).unwrap();
    assert!(h.into_mut::<X>().is_none());
  }

  #[test]
  fn query_state() {
    reset_test_env!();

    let x = State::value(0i32);
    let mut h = x.query(TypeId::of::<i32>()).unwrap();
    assert!(h.downcast_ref::<i32>().is_some());
    assert!(h.downcast_mut::<i32>().is_some());
    assert!(h.into_mut::<i32>().is_some());
  }

  #[test]
  fn query_split_state() {
    reset_test_env!();

    struct X {
      a: i32,
      _b: i32,
    }

    let x = State::value(X { a: 0, _b: 1 });
    let y = x.split_writer(|x| PartData::from_ref_mut(&mut x.a));
    let mut h = y.query(TypeId::of::<i32>()).unwrap();
    assert!(h.downcast_ref::<i32>().is_some());
    assert!(h.downcast_mut::<i32>().is_some());
  }

  #[test]
  fn query_reader_only() {
    reset_test_env!();

    let x = State::value(0i32).clone_reader();
    let mut h = x.query(TypeId::of::<i32>()).unwrap();
    assert!(h.downcast_ref::<i32>().is_some());
    assert!(h.downcast_mut::<i32>().is_none());
    assert!(h.into_mut::<i32>().is_none());
  }
}
