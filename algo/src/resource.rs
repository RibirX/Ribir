use std::{
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
#[derive(Debug, Serialize, Deserialize)]
pub struct Resource<T>(triomphe::Arc<T>);

impl<T> Resource<T> {
  #[inline]
  pub fn new(v: T) -> Self { Resource(triomphe::Arc::new(v)) }

  #[inline]
  pub fn as_ptr(this: &Self) -> *const () { triomphe::Arc::as_ptr(&this.0) as *const () }
}

impl<T> Clone for Resource<T> {
  #[inline]
  fn clone(&self) -> Self { Self(self.0.clone()) }
}

impl<T> Deref for Resource<T> {
  type Target = T;

  fn deref(&self) -> &Self::Target { self.0.deref() }
}

impl<T> PartialEq for Resource<T> {
  #[inline]
  fn eq(&self, other: &Self) -> bool { triomphe::Arc::ptr_eq(&self.0, &other.0) }
}

impl<T> Eq for Resource<T> {}

impl<T> Hash for Resource<T> {
  #[inline]
  fn hash<H: Hasher>(&self, state: &mut H) { triomphe::Arc::as_ptr(&self.0).hash(state); }
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
