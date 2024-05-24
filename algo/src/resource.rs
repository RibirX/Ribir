use std::{
  any::Any,
  hash::{Hash, Hasher},
  ops::Deref,
};

use serde::{Deserialize, Serialize};

/// A smarter pointer help us to share resource in application, and it' use a
/// cheap way to compare if two resource is same one.
///
/// # Notice
/// Compare two `Resource` just compare if it come form same resource not
/// compare its content.
#[derive(Debug, Deserialize)]
pub struct Resource<T: ?Sized>(triomphe::Arc<T>);

impl<T: Sized> Resource<T> {
  #[inline]
  pub fn new(v: T) -> Self { Resource(triomphe::Arc::new(v)) }

  #[inline]
  pub fn into_any(self) -> Resource<dyn Any>
  where
    T: Sized + Any,
  {
    let ptr = triomphe::Arc::into_raw(self.0) as *const dyn Any;
    let ptr: triomphe::Arc<dyn Any> = unsafe { triomphe::Arc::from_raw(ptr) };
    Resource(ptr)
  }
}
impl<T: ?Sized> Resource<T> {
  #[inline]
  pub fn as_ptr(this: &Self) -> *const () { triomphe::Arc::as_ptr(&this.0) as *const () }
}

impl<T: ?Sized> Clone for Resource<T> {
  #[inline]
  fn clone(&self) -> Self { Self(self.0.clone()) }
}

impl<T: ?Sized> Deref for Resource<T> {
  type Target = T;

  fn deref(&self) -> &Self::Target { self.0.deref() }
}

impl<T: ?Sized> PartialEq for Resource<T> {
  #[inline]
  fn eq(&self, other: &Self) -> bool { triomphe::Arc::ptr_eq(&self.0, &other.0) }
}

impl<T: ?Sized> Eq for Resource<T> {}

impl<T: ?Sized> Hash for Resource<T> {
  #[inline]
  fn hash<H: Hasher>(&self, state: &mut H) { triomphe::Arc::as_ptr(&self.0).hash(state); }
}

impl<T: Serialize> Serialize for Resource<T> {
  fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
    self.0.serialize(serializer)
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
}
