use std::{
  any::Any,
  hash::{Hash, Hasher},
  marker::PhantomData,
  ops::Deref,
};

use rclite::Arc;
use serde::{Deserialize, Serialize};

/// A smarter pointer help us to share resource in application, and it' use a
/// cheap way to compare if two resource is same one.
///
/// # Notice
/// Compare two `Resource` just compare if it come form same resource not
/// compare its content.
#[derive(Debug)]
pub struct Resource<T: ?Sized = dyn Any> {
  inner: Arc<Box<dyn Any>>,
  _marker: PhantomData<*const T>,
}

// Resource is Send + Sync if T is Send + Sync
unsafe impl<T: ?Sized + Send + Sync> Send for Resource<T> {}
unsafe impl<T: ?Sized + Send + Sync> Sync for Resource<T> {}

impl<T: 'static> Resource<T> {
  #[inline]
  pub fn new(v: T) -> Self {
    Resource { inner: Arc::new(Box::new(v) as Box<dyn Any>), _marker: PhantomData }
  }

  /// Convert to a type-erased Resource while preserving pointer identity.
  /// This allows the Resource to be used as a cache key that will match
  /// regardless of how it was obtained (directly or via clone + into_any).
  #[inline]
  pub fn into_any(self) -> Resource<dyn Any> {
    Resource { inner: self.inner, _marker: PhantomData }
  }
}

impl<T: ?Sized> Resource<T> {
  #[inline]
  pub fn as_ptr(this: &Self) -> *const () { Arc::as_ptr(&this.inner) as *const () }
}

impl<T: 'static> Deref for Resource<T> {
  type Target = T;

  fn deref(&self) -> &Self::Target {
    self
      .inner
      .downcast_ref::<T>()
      .expect("Resource type mismatch")
  }
}

impl Deref for Resource<dyn Any> {
  type Target = dyn Any;

  fn deref(&self) -> &Self::Target { self.inner.as_ref().as_ref() }
}

impl<T> From<T> for Resource<T>
where
  T: 'static,
{
  #[inline]
  fn from(value: T) -> Self { Self::new(value) }
}

impl<T: ?Sized> Clone for Resource<T> {
  #[inline]
  fn clone(&self) -> Self { Self { inner: self.inner.clone(), _marker: PhantomData } }
}

impl<T: ?Sized> PartialEq for Resource<T> {
  #[inline]
  fn eq(&self, other: &Self) -> bool { Arc::ptr_eq(&self.inner, &other.inner) }
}

impl<T: ?Sized> Eq for Resource<T> {}

impl<T: ?Sized> Hash for Resource<T> {
  #[inline]
  fn hash<H: Hasher>(&self, state: &mut H) { Arc::as_ptr(&self.inner).hash(state); }
}

impl<T: Serialize + 'static> Serialize for Resource<T> {
  fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
    (**self).serialize(serializer)
  }
}

impl<'de, T: Deserialize<'de> + 'static> Deserialize<'de> for Resource<T> {
  fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
    T::deserialize(deserializer).map(Resource::new)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn compare() {
    let a = Resource::new(5);
    let b = Resource::new(5);
    assert_ne!(a, b);

    #[allow(clippy::redundant_clone)]
    let a2 = a.clone();
    assert_eq!(a, a2);
    assert!(5 == *a2);
  }

  #[test]
  fn hash() {
    let a = Resource::new("a");
    let mut map = std::collections::HashSet::new();
    map.insert(a.clone());

    assert!(!map.contains(&Resource::new("a")));
    assert!(map.contains(&a));
  }

  #[test]
  fn share_local_compare() {
    let a = Resource::new(1);
    let b = Resource::new(1);
    assert_ne!(b, a);
  }

  #[test]
  fn into_any_preserves_identity() {
    let a = Resource::new(42);
    let b = a.clone();

    // into_any should preserve identity
    let a_any = a.into_any();
    let b_any = b.into_any();

    assert_eq!(a_any, b_any);
  }

  #[test]
  fn into_any_different_resources() {
    let a = Resource::new(42);
    let b = Resource::new(42);

    let a_any = a.into_any();
    let b_any = b.into_any();

    // Different resources should not be equal
    assert_ne!(a_any, b_any);
  }
}
