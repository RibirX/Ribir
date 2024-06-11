use std::any::{Any, TypeId};

use smallvec::SmallVec;

use crate::state::{
  MapReader, MapWriterAsReader, ReadRef, Reader, StateReader, StateWriter, WriteRef,
};

/// A type can composed by many types, this trait help us to query the type and
/// the inner type by its type id
pub trait Query: Any {
  /// Queries all types that match the provided type id, returning their handles
  /// in an inside-to-outside order.
  fn query_all(&self, type_id: TypeId) -> SmallVec<[QueryHandle; 1]>;

  /// Queries the type that matches the provided type id, returning its handle.
  /// This method always returns the outermost type.
  fn query(&self, type_id: TypeId) -> Option<QueryHandle>;
}

/// A dynamic handle to a query result of a data, so we can use it in a trait
/// object.
pub struct QueryHandle<'a>(InnerHandle<'a>);

/// A reference to a query result of a data, it's similar to `&T`.
pub struct QueryRef<'a, T> {
  pub(crate) type_ref: &'a T,
  pub(crate) _data: Option<Box<dyn QueryResult + 'a>>,
}

impl<'a> QueryHandle<'a> {
  pub fn new(r: &'a dyn Any) -> Self { QueryHandle(InnerHandle::Ref(r)) }

  /// Downcast the to type `T` and returns a reference to it,
  /// return `None` if the type not match
  pub fn downcast_ref<T: Any>(&self) -> Option<&T> {
    match self.0 {
      InnerHandle::Ref(r) => r.downcast_ref::<T>(),
      InnerHandle::Owned(ref o) => query_downcast_ref(o.as_ref()),
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
    (o.query_type() == TypeId::of::<WriteRef<'static, T>>()).then(|| {
      // SAFETY: the creater guarantees that the query type is `WriteRef<T>`,
      unsafe { &mut **(o.as_mut() as *mut dyn QueryResult as *mut WriteRef<'a, T>) }
    })
  }

  pub(crate) fn owned(o: Box<dyn QueryResult + 'a>) -> Self { QueryHandle(InnerHandle::Owned(o)) }

  pub fn into_ref<T: Any>(self) -> Option<QueryRef<'a, T>> {
    match self.0 {
      InnerHandle::Ref(r) if r.type_id() == TypeId::of::<T>() => {
        Some(QueryRef { type_ref: r.downcast_ref::<T>().unwrap(), _data: None })
      }
      InnerHandle::Owned(o) => {
        let r = query_downcast_ref(o.as_ref());
        // hold the _data to keep the data alive
        r.map(|r| QueryRef { type_ref: r, _data: Some(o) })
      }
      _ => None,
    }
  }

  pub fn into_mut<T: Any>(self) -> Option<WriteRef<'a, T>> {
    let InnerHandle::Owned(o) = self.0 else {
      return None;
    };
    (o.query_type() == TypeId::of::<WriteRef<'static, T>>()).then(|| {
      // SAFETY: the creater guarantees that the query type is `WriteRef<T>`,
      let w_r = unsafe {
        let ptr = Box::into_raw(o);
        Box::from_raw(ptr as *mut WriteRef<'a, T>)
      };
      *w_r
    })
  }
}

fn query_downcast_ref<'a, T: Any>(q: &(dyn QueryResult + 'a)) -> Option<&'a T> {
  let q_type = q.query_type();
  if q_type == TypeId::of::<T>() {
    // SAFETY: the creater guarantees that the query type is `T`,
    let t = unsafe { &*(q as *const dyn QueryResult as *const T) };
    Some(t)
  } else if q_type == TypeId::of::<WriteRef<'static, T>>() {
    // SAFETY: the creater guarantees that the query type is `WriteRef<T>`,
    let wr = unsafe { &*(q as *const dyn QueryResult as *const WriteRef<'a, T>) };
    Some(wr)
  } else if q_type == TypeId::of::<ReadRef<'static, T>>() {
    // SAFETY: the creater guarantees that the query type is `WriteRef<T>`,
    let rr = unsafe { &*(q as *const dyn QueryResult as *const ReadRef<'a, T>) };
    Some(rr)
  } else {
    None
  }
}
enum InnerHandle<'a> {
  Ref(&'a dyn Any),
  Owned(Box<dyn QueryResult + 'a>),
}

pub(crate) trait QueryResult {
  fn query_type(&self) -> TypeId;
}

impl<'a> QueryResult for &'a dyn Any {
  fn query_type(&self) -> TypeId { Any::type_id(*self) }
}

impl<'a, T: Any> QueryResult for WriteRef<'a, T> {
  fn query_type(&self) -> TypeId { TypeId::of::<WriteRef<'static, T>>() }
}

impl<'a, T: Any> QueryResult for ReadRef<'a, T> {
  fn query_type(&self) -> TypeId { TypeId::of::<ReadRef<'static, T>>() }
}

impl<'a, T> std::ops::Deref for QueryRef<'a, T> {
  type Target = T;

  fn deref(&self) -> &Self::Target { self.type_ref }
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
      Some(QueryHandle::owned(Box::new(self.write())))
    } else if type_id == self.type_id() {
      Some(QueryHandle::new(self))
    } else {
      None
    }
  }
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
        Some(QueryHandle::owned(Box::new(self.read())))
      } else if type_id == self.type_id() {
        Some(QueryHandle::new(self))
      } else {
        None
      }
    }
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
  use crate::{
    data_widget::Queryable,
    reset_test_env,
    state::{PartData, State, StateReader, StateWriter},
  };

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
