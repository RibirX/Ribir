use std::{borrow::Borrow, fmt::Debug, sync::Arc};

#[derive(Hash, PartialEq, Eq)]

///! A copy on write smart pointer shared value without deep clone .
pub enum CowRc<B: ToOwned + ?Sized + 'static> {
  /// Borrowed  data
  Borrowed(&'static B),
  /// Owned data
  Owned(Arc<B::Owned>),
}

impl<B: ?Sized + ToOwned> std::ops::Deref for CowRc<B>
where
  B::Owned: std::borrow::Borrow<B>,
{
  type Target = B;

  fn deref(&self) -> &B {
    match self {
      CowRc::Borrowed(borrowed) => borrowed,
      CowRc::Owned(ref owned) => (&**owned).borrow(),
    }
  }
}

impl<T: ToOwned + ?Sized> Clone for CowRc<T> {
  #[inline]
  fn clone(&self) -> Self {
    match self {
      CowRc::Borrowed(borrowed) => CowRc::Borrowed(borrowed),
      CowRc::Owned(owned) => CowRc::Owned(owned.clone()),
    }
  }
}

impl<B: ?Sized> Debug for CowRc<B>
where
  B: Debug + ToOwned,
  B::Owned: Debug,
{
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match *self {
      CowRc::Borrowed(ref b) => Debug::fmt(b, f),
      CowRc::Owned(ref o) => Debug::fmt(o, f),
    }
  }
}

impl<T: ToOwned + ?Sized> CowRc<T> {
  #[inline]
  pub fn borrowed(v: &'static T) -> Self { CowRc::Borrowed(v) }

  #[inline]
  pub fn owned(v: T::Owned) -> Self { CowRc::Owned(Arc::new(v)) }

  /// Return true if the data is borrowed
  #[inline]
  pub fn is_borrowed(&self) -> bool { matches!(self, CowRc::Borrowed(_)) }

  ///  Return  true if the data is owned
  #[inline]
  pub fn is_owned(&self) -> bool { matches!(self, CowRc::Owned(_)) }

  /// Acquires a mutable reference to the owned form of the data.
  /// Clones the data if it is not already owned or other pointer to the same
  /// allocation
  pub fn to_mut(&mut self) -> &mut T::Owned
  where
    T::Owned: Clone,
  {
    if let CowRc::Borrowed(borrowed) = self {
      let a = Arc::new(borrowed.to_owned());
      *self = CowRc::Owned(a);
    }

    let arc = match self {
      CowRc::Borrowed(_) => unreachable!(),
      CowRc::Owned(owned) => owned,
    };
    // Safety:  `Arc::get_mut` and `Arc::make_mut` cannot borrow both in logic.
    let split_lf: &mut _ = unsafe { &mut *(arc as *mut Arc<_>) };
    Arc::get_mut(split_lf).unwrap_or_else(|| Arc::make_mut(arc))
  }
}

impl<T: ToOwned + ?Sized> From<&'static T> for CowRc<T> {
  fn from(borrowed: &'static T) -> Self { CowRc::borrowed(borrowed) }
}

impl<T: ToOwned<Owned = T>> From<T> for CowRc<T> {
  fn from(owned: T) -> Self { CowRc::owned(owned) }
}

#[test]
fn smoke() {
  static V: i32 = 1;

  let mut cow = CowRc::borrowed(&V);
  let c_cow = cow.clone();

  assert_eq!(cow, c_cow);
  // have same pointer address
  assert!(std::ptr::eq(&*cow as *const i32, &*c_cow as *const i32));

  *cow.to_mut() = 2;
  // cow should deep cloned.
  assert!(cow.is_owned());
  assert!(c_cow.is_borrowed());
  assert_eq!(&*cow, &2);
  assert_eq!(&*c_cow, &1);
}
