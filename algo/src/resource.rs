use std::{
  hash::{Hash, Hasher},
  ops::Deref,
  rc::Rc,
};

use serde::{Deserialize, Serialize};

/// Enum to store both share and local resource. Share resources can have many
/// copy, and every copy as the same one. Local resources means is individual
/// and no other copy exist, two local resources will not as same one even its
/// content is equal.
#[derive(Eq, Debug, Clone)]
pub enum Resource<T> {
  Share(ShareResource<T>),
  Local(T),
}

/// A smarter pointer help us to share resource in application, and it' use a
/// cheap way to compare if two resource is same one.
///
/// # Notice
/// Compare two `ShareResource` just compare if it come form same resource not
/// compare its content.
#[derive(Debug, Serialize, Deserialize)]
pub struct ShareResource<T>(Rc<T>);

impl<T> ShareResource<T> {
  #[inline]
  pub fn new(v: T) -> Self { ShareResource(Rc::new(v)) }
  #[inline]
  pub fn as_ptr(this: &Self) -> *const () { Rc::as_ptr(&this.0) as *const () }
}

impl<T> Clone for ShareResource<T> {
  #[inline]
  fn clone(&self) -> Self { Self(self.0.clone()) }
}

impl<T> Deref for Resource<T> {
  type Target = T;

  fn deref(&self) -> &Self::Target {
    match self {
      Resource::Share(s) => s.deref(),
      Resource::Local(l) => l,
    }
  }
}

impl<T> PartialEq for Resource<T> {
  #[inline]
  fn eq(&self, other: &Self) -> bool {
    match (self, other) {
      (Resource::Share(a), Resource::Share(b)) => a == b,
      _ => false,
    }
  }
}

impl<T> Deref for ShareResource<T> {
  type Target = T;

  fn deref(&self) -> &Self::Target { self.0.deref() }
}

impl<T> PartialEq for ShareResource<T> {
  #[inline]
  fn eq(&self, other: &Self) -> bool { Rc::ptr_eq(&self.0, &other.0) }
}

impl<T> Eq for ShareResource<T> {}

impl<T> Hash for ShareResource<T> {
  #[inline]
  fn hash<H: Hasher>(&self, state: &mut H) { Rc::as_ptr(&self.0).hash(state); }
}

impl<T> From<T> for Resource<T> {
  #[inline]
  fn from(r: T) -> Self { Resource::Local(r) }
}

impl<T> From<ShareResource<T>> for Resource<T> {
  #[inline]
  fn from(s: ShareResource<T>) -> Self { Resource::Share(s) }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn compare() {
    let a = ShareResource::new(5);
    let b = ShareResource::new(5);
    assert_ne!(a, b);

    let a2 = a.clone();
    assert_eq!(a, a2);
    assert!(5 == *a2);
  }

  #[test]
  fn hash() {
    let a = ShareResource::new("a");
    let mut map = std::collections::HashSet::new();
    map.insert(a.clone());

    assert!(!map.contains(&ShareResource::new("a")));
    assert!(map.contains(&a));
  }

  #[test]
  fn share_local_compare() {
    let s = ShareResource::new(1);
    let l = Resource::Local(1);
    assert_ne!(l, s.into());
    assert_ne!(l, 1.into());
  }
}
