use std::any::{Any, TypeId};

use smallvec::SmallVec;

use crate::state::*;

/// A type can composed by many types, this trait help us to query the type and
/// the inner type by type id.
pub trait Query: Any {
  /// Queries all types that match the provided type id, returning their handles
  /// in an inside-to-outside order.
  fn query_all<'q>(&'q self, query_id: &QueryId, out: &mut SmallVec<[QueryHandle<'q>; 1]>);

  /// Queries all writers that match the provided type id, returning their
  /// handles in an inside-to-outside order.
  fn query_all_write<'q>(&'q self, query_id: &QueryId, out: &mut SmallVec<[QueryHandle<'q>; 1]>);

  /// Queries the type that matches the provided type id, returning its handle.
  /// This method always returns the outermost type.
  fn query(&self, query_id: &QueryId) -> Option<QueryHandle>;

  /// Queries the reference of the writer that matches the provided type id.
  fn query_write(&self, id: &QueryId) -> Option<QueryHandle>;

  /// Hint if this is a queryable type.
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
  pub(crate) data: Option<Box<dyn QueryAny>>,
}

impl<'a> QueryHandle<'a> {
  /// Downcast the to type `T` and returns a reference to it,
  /// return `None` if the type not match
  pub fn downcast_ref<T: Any>(&self) -> Option<&T> {
    match self.0 {
      InnerHandle::Ref(r) => r.q_downcast_ref::<T>(),
      InnerHandle::Owned(ref o) => downcast_from_state_ref(o),
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

    o.q_downcast_mut::<WriteRef<'static, T>>()
      .map(|v| &mut **v)
  }

  pub(crate) fn new(r: &'a dyn QueryAny) -> Self { QueryHandle(InnerHandle::Ref(r)) }

  pub(crate) fn from_read_ref<T: ?Sized + 'static>(r: ReadRef<'a, T>) -> Self {
    // Safety: The lifetime is maintained in the return handle and will be shortened
    // once the handle is downcast.
    let r: ReadRef<'static, T> = unsafe { std::mem::transmute(r) };
    QueryHandle(InnerHandle::Owned(Box::new(r)))
  }

  pub(crate) fn from_write_ref<T: ?Sized + 'static>(w: WriteRef<'a, T>) -> Self {
    // Safety: The lifetime is maintained in the return handle and will be shortened
    // once the handle is downcast.
    let w: WriteRef<'static, T> = unsafe { std::mem::transmute(w) };
    QueryHandle(InnerHandle::Owned(Box::new(w)))
  }

  pub fn into_ref<T: Any>(self) -> Option<QueryRef<'a, T>> {
    match self.0 {
      InnerHandle::Ref(r) => r
        .q_downcast_ref::<T>()
        .map(|type_ref| QueryRef { type_ref, data: None }),
      InnerHandle::Owned(o) => {
        let inner = downcast_from_state_ref::<T>(&o)?;
        let type_ref = unsafe { &*(inner as *const T) };
        Some(QueryRef { type_ref, data: Some(o) })
      }
    }
  }

  pub fn into_mut<T: Any>(self) -> Option<WriteRef<'a, T>> {
    let InnerHandle::Owned(o) = self.0 else {
      return None;
    };

    o.is::<WriteRef<'static, T>>().then(|| unsafe {
      let raw = Box::into_raw(o);
      *Box::from_raw(raw as *mut WriteRef<'static, T>)
    })
  }
}

#[allow(clippy::borrowed_box)] // The state reference must be a trait boxed.
fn downcast_from_state_ref<T: 'static>(owned: &Box<dyn QueryAny>) -> Option<&T> {
  owned
    .q_downcast_ref::<ReadRef<'static, T>>()
    .map(|v| &**v)
    .or_else(|| {
      owned
        .q_downcast_ref::<WriteRef<'static, T>>()
        .map(|v| &**v)
    })
}

impl<'q, T: ?Sized> QueryRef<'q, T> {
  /// Makes a new `QueryRef` for a component of the borrowed data.
  ///
  /// This is an associated function that needs to be used as
  /// `QueryRef::map(...)`. A method would interfere with methods of the same
  /// name on `T` used through `Deref`.
  ///
  /// # Examples
  ///
  /// ```
  /// use ribir_core::prelude::*;
  ///
  /// let data = Queryable((5u32, 'b'));
  /// let q1 = data
  ///   .query(&QueryId::of::<(u32, char)>())
  ///   .and_then(|h| h.into_ref::<(u32, char)>())
  ///   .unwrap();
  ///
  /// let q2: QueryRef<'_, u32> = QueryRef::map(q1, |t: &(u32, char)| &t.0);
  /// assert_eq!(*q2, 5)
  /// ```
  pub fn map<U: ?Sized>(orig: Self, map: impl FnOnce(&T) -> &U) -> QueryRef<'q, U> {
    let Self { type_ref, data: _data } = orig;
    let type_ref = map(type_ref);
    QueryRef { type_ref, data: _data }
  }

  /// Makes a new `QueryRef` for an optional component of the borrowed data. The
  /// original guard is returned as an `Err(..)` if the closure returns
  /// `None`.
  ///
  /// This is an associated function that needs to be used as
  /// `QueryRef::filter_map(...)`. A method would interfere with methods of the
  /// same name on `T` used through `Deref`.
  ///
  /// # Examples
  ///
  /// ```
  /// use ribir_core::prelude::*;
  ///
  /// let q = Queryable(vec![1u32, 2, 3]);
  /// let q1: QueryRef<'_, Vec<u32>> = q
  ///   .query(&QueryId::of::<Vec<u32>>())
  ///   .and_then(|h| h.into_ref::<Vec<u32>>())
  ///   .unwrap();
  /// let q2: Result<QueryRef<'_, u32>, _> = QueryRef::filter_map(q1, |v: &Vec<u32>| v.get(1));
  /// assert_eq!(*q2.unwrap(), 2);
  /// ```
  pub fn filter_map<U: ?Sized, F>(orig: Self, f: F) -> Result<QueryRef<'q, U>, Self>
  where
    F: FnOnce(&T) -> Option<&U>,
  {
    match f(orig.type_ref) {
      Some(value) => Ok(QueryRef { type_ref: value, data: orig.data }),
      None => Err(orig),
    }
  }
}
enum InnerHandle<'a> {
  Ref(&'a dyn QueryAny),
  Owned(Box<dyn QueryAny>),
}

impl<'a, T: ?Sized> std::ops::Deref for QueryRef<'a, T> {
  type Target = T;

  fn deref(&self) -> &Self::Target { self.type_ref }
}

impl<T: Any> Query for Queryable<T> {
  fn query_all<'q>(&'q self, query_id: &QueryId, out: &mut SmallVec<[QueryHandle<'q>; 1]>) {
    if let Some(h) = self.query(query_id) {
      out.push(h)
    }
  }

  fn query_all_write<'q>(&'q self, _: &QueryId, _: &mut SmallVec<[QueryHandle<'q>; 1]>) {}

  fn query(&self, query_id: &QueryId) -> Option<QueryHandle> {
    (query_id == &QueryId::of::<T>()).then(|| QueryHandle::new(&self.0))
  }

  fn query_write(&self, _: &QueryId) -> Option<QueryHandle> { None }

  fn queryable(&self) -> bool { true }
}

impl<T: StateWriter> Query for T
where
  T::Value: 'static + Sized,
{
  fn query_all<'q>(&'q self, query_id: &QueryId, out: &mut SmallVec<[QueryHandle<'q>; 1]>) {
    // The value of the writer and the writer itself cannot be queried
    // at the same time.
    if let Some(h) = self.query(query_id) {
      out.push(h)
    }
  }

  fn query_all_write<'q>(&'q self, query_id: &QueryId, out: &mut SmallVec<[QueryHandle<'q>; 1]>) {
    if let Some(h) = self.query_write(query_id) {
      out.push(h)
    }
  }

  fn query(&self, query_id: &QueryId) -> Option<QueryHandle> {
    if query_id == &QueryId::of::<T::Value>() {
      Some(QueryHandle::from_read_ref(self.read()))
    } else if query_id == &QueryId::of::<T>() {
      Some(QueryHandle::new(self))
    } else {
      None
    }
  }

  fn query_write(&self, query_id: &QueryId) -> Option<QueryHandle> {
    if query_id == &QueryId::of::<T::Value>() {
      Some(QueryHandle::from_write_ref(self.write()))
    } else if query_id == &QueryId::of::<T>() {
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
    fn query_all<'q>(&'q self, query_id: &QueryId, out: &mut SmallVec<[QueryHandle<'q>; 1]>) {
      if let Some(h) = self.query(query_id) {
        out.push(h)
      }
    }

    fn query_all_write<'q>(&'q self, _: &QueryId, _: &mut SmallVec<[QueryHandle<'q>; 1]>) {}

    fn query(&self, query_id: &QueryId) -> Option<QueryHandle> {
      if query_id == &QueryId::of::<V>() {
        Some(QueryHandle::from_read_ref(self.read()))
      } else if query_id == &QueryId::of::<Self>() {
        Some(QueryHandle::new(self))
      } else {
        None
      }
    }

    fn query_write(&self, _: &QueryId) -> Option<QueryHandle> { None }

    fn queryable(&self) -> bool { true }
  };
}

impl<S, F, V: 'static> Query for MapReader<S, F>
where
  Self: StateReader<Value = V>,
{
  impl_query_for_reader!();
}

impl<V: 'static> Query for Reader<V> {
  impl_query_for_reader!();
}

impl<V: 'static> Query for Box<dyn StateReader<Value = V>> {
  impl_query_for_reader!();
}

impl<V: 'static, R: StateReader<Value = V>> Query for Watcher<R> {
  impl_query_for_reader!();
}

impl<V: 'static> Query for Box<dyn StateWatcher<Value = V>> {
  impl_query_for_reader!();
}

/// This type is used to identify a queryable type.
///
/// Instead of directly using `TypeId`, we utilize its name and memory layout
/// for a secondary check as `TypeId` is not unique between binaries.
///
/// Retain the `TypeId` for efficient comparisons.
#[derive(Debug, Eq, Clone, Copy)]
pub struct QueryId {
  type_id: TypeId,
  info: fn() -> TypeInfo,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub(crate) struct TypeInfo {
  pub(crate) name: &'static str,
  pub(crate) pkg_version: &'static str,
  pub(crate) layout: &'static std::alloc::Layout,
}

pub(crate) struct TypeInfoOf<T> {
  _phantom: std::marker::PhantomData<T>,
}

impl<T> TypeInfoOf<T> {
  const LAYOUT: std::alloc::Layout = std::alloc::Layout::new::<T>();

  pub(crate) fn type_info() -> TypeInfo {
    TypeInfo {
      name: std::any::type_name::<T>(),
      pkg_version: env!("CARGO_PKG_VERSION"),
      layout: &Self::LAYOUT,
    }
  }
}

impl QueryId {
  pub fn of<T: 'static>() -> Self {
    QueryId { type_id: TypeId::of::<T>(), info: || TypeInfoOf::<T>::type_info() }
  }

  pub fn is_same(&self, other: &QueryId) -> bool {
    if self.type_id == other.type_id { true } else { (self.info)() == (other.info)() }
  }
}

impl PartialEq for QueryId {
  fn eq(&self, other: &Self) -> bool { self.is_same(other) }
}

pub(crate) trait QueryAny: Any {
  fn query_id(&self) -> QueryId;
}

impl<T: Any> QueryAny for T {
  fn query_id(&self) -> QueryId { QueryId::of::<T>() }
}

impl dyn QueryAny {
  fn is<T: QueryAny>(&self) -> bool { self.query_id() == QueryId::of::<T>() }

  fn q_downcast_ref<T: QueryAny>(&self) -> Option<&T> {
    if self.is::<T>() {
      // SAFETY: just checked whether we are pointing to the correct type, and we can
      // rely on that check for memory safety because we have implemented Any
      // for all types; no other impls can exist as they would conflict with our
      // impl.
      unsafe { Some(&*(self as *const dyn QueryAny as *const T)) }
    } else {
      None
    }
  }

  fn q_downcast_mut<T: QueryAny>(&mut self) -> Option<&mut T> {
    if self.is::<T>() {
      // SAFETY: just checked whether we are pointing to the correct type, and we can
      // rely on that check for memory safety because we have implemented Any
      // for all types; no other impls can exist as they would conflict with our
      // impl.
      Some(unsafe { &mut *(self as *mut dyn QueryAny as *mut T) })
    } else {
      None
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{prelude::PartMut, reset_test_env, state::State};

  #[test]
  fn query_ref() {
    reset_test_env!();

    struct X;
    let x = Queryable(X);
    let mut h = x.query(&QueryId::of::<X>()).unwrap();
    assert!(h.downcast_ref::<X>().is_some());
    assert!(h.downcast_mut::<X>().is_none());
    assert!(h.into_ref::<X>().is_some());
    let h = x.query(&QueryId::of::<X>()).unwrap();
    assert!(h.into_mut::<X>().is_none());
  }

  #[test]
  fn query_state() {
    reset_test_env!();

    let x = State::value(0i32);
    {
      let h = x.query(&QueryId::of::<i32>()).unwrap();
      assert!(h.downcast_ref::<i32>().is_some());
    }
    let mut h = x.query_write(&QueryId::of::<i32>()).unwrap();
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
    let y = x.split_writer(|x| PartMut::new(&mut x.a));
    {
      let h = y.query(&QueryId::of::<i32>()).unwrap();
      assert!(h.downcast_ref::<i32>().is_some());
    }
    let mut h = y.query_write(&QueryId::of::<i32>()).unwrap();
    assert!(h.downcast_mut::<i32>().is_some());
  }

  #[test]
  fn query_reader_only() {
    reset_test_env!();

    let x = State::value(0i32).clone_reader();
    let mut h = x.query(&QueryId::of::<i32>()).unwrap();
    assert!(h.downcast_ref::<i32>().is_some());
    assert!(h.downcast_mut::<i32>().is_none());
    assert!(h.into_mut::<i32>().is_none());
  }
}
