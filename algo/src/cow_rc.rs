use std::{
  borrow::{Borrow, BorrowMut},
  fmt::Debug,
  hash::Hash,
  ops::{Bound, Deref, Range, RangeBounds},
  sync::Arc,
};

#[derive(Eq)]
/// A copy on write smart pointer shared value without deep clone .
pub enum CowArc<B: ToOwned + ?Sized + 'static> {
  /// Borrowed  data
  Borrowed(&'static B),
  /// Owned data
  Owned(Arc<B::Owned>),
}

#[derive(Debug, Clone, Eq)]
pub struct Substr {
  str: CowArc<str>,
  rg: Range<usize>,
}

impl CowArc<str> {
  pub fn substr(&self, rg: impl RangeBounds<usize>) -> Substr {
    let start = match rg.start_bound() {
      Bound::Included(&n) => n,
      Bound::Excluded(&n) => n + 1,
      Bound::Unbounded => 0,
    };

    let end = match rg.end_bound() {
      Bound::Included(&n) => n + 1,
      Bound::Excluded(&n) => n,
      Bound::Unbounded => self.len(),
    };
    Substr { str: self.clone(), rg: Range { start, end } }
  }
}

impl Substr {
  pub fn substr(&self, rg: impl RangeBounds<usize>) -> Substr {
    let offset = self.rg.start;
    let mut start = match rg.start_bound() {
      Bound::Included(&n) => n,
      Bound::Excluded(&n) => n + 1,
      Bound::Unbounded => 0,
    };

    let mut end = match rg.end_bound() {
      Bound::Included(&n) => n + 1,
      Bound::Excluded(&n) => n,
      Bound::Unbounded => self.len(),
    };
    start += offset;
    end = self.rg.end.min(end + offset);

    Substr { str: self.str.clone(), rg: start..end }
  }
}
impl std::ops::Deref for Substr {
  type Target = str;

  fn deref(&self) -> &str {
    let Self { str, rg } = self;
    &str[rg.clone()]
  }
}

impl<B: ?Sized + ToOwned> std::ops::Deref for CowArc<B>
where
  B::Owned: std::borrow::Borrow<B>,
{
  type Target = B;

  fn deref(&self) -> &B {
    match self {
      CowArc::Borrowed(borrowed) => borrowed,
      CowArc::Owned(ref owned) => (**owned).borrow(),
    }
  }
}

impl<B: ?Sized + ToOwned> std::ops::DerefMut for CowArc<B>
where
  B::Owned: Clone + std::borrow::BorrowMut<B>,
{
  fn deref_mut(&mut self) -> &mut B { self.to_mut().borrow_mut() }
}

impl<T: ToOwned + ?Sized + 'static> Clone for CowArc<T> {
  #[inline]
  fn clone(&self) -> Self {
    match self {
      CowArc::Borrowed(borrowed) => CowArc::Borrowed(borrowed),
      CowArc::Owned(owned) => CowArc::Owned(owned.clone()),
    }
  }
}

impl<B: ?Sized> Debug for CowArc<B>
where
  B: Debug + ToOwned,
  B::Owned: Debug,
{
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match *self {
      CowArc::Borrowed(ref b) => Debug::fmt(b, f),
      CowArc::Owned(ref o) => Debug::fmt(o, f),
    }
  }
}

impl<T: ToOwned + ?Sized> CowArc<T> {
  #[inline]
  pub fn borrowed(v: &'static T) -> Self { CowArc::Borrowed(v) }

  #[inline]
  pub fn owned(v: T::Owned) -> Self { CowArc::Owned(Arc::new(v)) }

  /// Return if two `CowRc` pointer to same allocation.
  pub fn ptr_eq(&self, other: &Self) -> bool {
    match (self, other) {
      (CowArc::Borrowed(a), CowArc::Borrowed(b)) => std::ptr::eq(a, b),
      (CowArc::Owned(a), CowArc::Owned(b)) => Arc::ptr_eq(a, b),
      _ => false,
    }
  }

  /// Return true if the data is borrowed
  #[inline]
  pub fn is_borrowed(&self) -> bool { matches!(self, CowArc::Borrowed(_)) }

  ///  Return  true if the data is owned
  #[inline]
  pub fn is_owned(&self) -> bool { matches!(self, CowArc::Owned(_)) }

  /// Acquires a mutable reference to the owned form of the data.
  /// Clones the data if it is not already owned or other pointer to the same
  /// allocation
  pub fn to_mut(&mut self) -> &mut T::Owned
  where
    T::Owned: Clone,
  {
    if let CowArc::Borrowed(borrowed) = self {
      let a = Arc::new(borrowed.to_owned());
      *self = CowArc::Owned(a);
    }

    let arc = match self {
      CowArc::Borrowed(_) => unreachable!(),
      CowArc::Owned(owned) => owned,
    };
    // Safety:  `Arc::get_mut` and `Arc::make_mut` cannot borrow both in logic.
    let split_lf: &mut _ = unsafe { &mut *(arc as *mut Arc<_>) };
    Arc::get_mut(split_lf).unwrap_or_else(|| Arc::make_mut(arc))
  }
}

impl<T: ToOwned + ?Sized> From<&'static T> for CowArc<T> {
  fn from(borrowed: &'static T) -> Self { CowArc::borrowed(borrowed) }
}

impl<T: ToOwned<Owned = T>> From<T> for CowArc<T> {
  fn from(owned: T) -> Self { CowArc::owned(owned) }
}

impl From<String> for CowArc<str> {
  #[inline]
  fn from(str: String) -> Self { CowArc::owned(str) }
}

impl Default for CowArc<str> {
  fn default() -> Self { Self::from(String::default()) }
}

impl<B: ?Sized + ToOwned> Borrow<B> for CowArc<B> {
  fn borrow(&self) -> &B {
    match self {
      CowArc::Borrowed(b) => b,
      CowArc::Owned(o) => (**o).borrow(),
    }
  }
}

impl<B: ?Sized + ToOwned + PartialEq> PartialEq for CowArc<B> {
  fn eq(&self, other: &Self) -> bool {
    let a: &B = self.borrow();
    let b = other.borrow();
    a == b
  }
}

impl<B: ?Sized + ToOwned + Hash> Hash for CowArc<B> {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    let borrow: &B = self.borrow();
    borrow.hash(state);
  }
}

impl<T: std::ops::Index<Idx> + Clone, Idx> std::ops::Index<Idx> for CowArc<T> {
  type Output = T::Output;

  #[inline]
  fn index(&self, index: Idx) -> &Self::Output { &(self.deref())[index] }
}

impl std::ops::Index<Range<usize>> for Substr {
  type Output = str;

  #[inline]
  fn index(&self, rg: Range<usize>) -> &Self::Output { &(self.deref())[rg] }
}

impl PartialEq for Substr {
  #[inline]
  fn eq(&self, other: &Self) -> bool { self.deref() == other.deref() }
}

impl Hash for Substr {
  #[inline]
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) { self.deref().hash(state) }
}

impl<T: Into<CowArc<str>>> From<T> for Substr {
  fn from(v: T) -> Self {
    let str = v.into();
    str.substr(..)
  }
}

#[test]
fn smoke() {
  static V: i32 = 1;

  let mut cow = CowArc::borrowed(&V);
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
