//! This implementation is a fork from `std::cell::RefCell`, allowing us to
//! manage the borrow flag.
use std::{
  cell::{Cell, UnsafeCell},
  ops::{Deref, DerefMut},
  ptr::NonNull,
};

type BorrowFlag = isize;
const UNUSED: BorrowFlag = 0;

#[inline(always)]
fn is_reading(x: BorrowFlag) -> bool { x > UNUSED }

#[inline(always)]
fn is_writing(x: BorrowFlag) -> bool { x < UNUSED }

pub(crate) struct StateCell<W: ?Sized> {
  borrow_flag: Cell<BorrowFlag>,
  #[cfg(debug_assertions)]
  borrowed_at: Cell<Option<&'static std::panic::Location<'static>>>,
  data: UnsafeCell<W>,
}

impl<W> StateCell<W> {
  pub(crate) fn new(data: W) -> Self {
    StateCell {
      borrow_flag: Cell::new(UNUSED),
      #[cfg(debug_assertions)]
      borrowed_at: Cell::new(None),
      data: UnsafeCell::new(data),
    }
  }

  #[track_caller]
  pub(crate) fn read(&self) -> ValueRef<W> {
    let borrow = &self.borrow_flag;
    let b = borrow.get().wrapping_add(1);
    borrow.set(b);
    #[cfg(debug_assertions)]
    {
      // `borrowed_at` is always the *first* active borrow
      if b == 1 {
        self
          .borrowed_at
          .set(Some(std::panic::Location::caller()));
      }
    }
    if !is_reading(b) {
      // If a borrow occurred, then we must already have an outstanding borrow,
      // so `borrowed_at` will be `Some`
      #[cfg(debug_assertions)]
      panic!("Already mutably borrowed: {:?}", self.borrowed_at.get().unwrap());
      #[cfg(not(debug_assertions))]
      panic!("Already mutably borrowed");
    }

    // SAFETY: `BorrowRef` ensures that there is only immutable access
    // to the value while borrowed.
    let value = PartData::PartRef(unsafe { NonNull::new_unchecked(self.data.get()) });
    ValueRef { value, borrow: BorrowRef { borrow } }
  }

  pub(crate) fn write(&self) -> ValueMutRef<'_, W> {
    // NOTE: Unlike BorrowRefMut::clone, new is called to create the initial
    // mutable reference, and so there must currently be no existing
    // references. Thus, while clone increments the mutable refcount, here
    // we explicitly only allow going from UNUSED to UNUSED - 1.
    let borrow = &self.borrow_flag;
    if borrow.get() != UNUSED {
      #[cfg(debug_assertions)]
      panic!("Already borrowed at: {:?}", self.borrowed_at.get().unwrap());
      #[cfg(not(debug_assertions))]
      panic!("Already borrowed");
    }
    #[cfg(debug_assertions)]
    {
      // If a borrow occurred, then we must already have an outstanding borrow,
      // so `borrowed_at` will be `Some`
      self
        .borrowed_at
        .set(Some(std::panic::Location::caller()));
    }

    borrow.set(UNUSED - 1);
    let v_ref = BorrowRefMut { borrow };
    let value = PartData::PartRef(unsafe { NonNull::new_unchecked(self.data.get()) });
    ValueMutRef { value, borrow: v_ref }
  }

  pub(crate) fn is_unused(&self) -> bool { self.borrow_flag.get() == UNUSED }

  pub(super) fn into_inner(self) -> W { self.data.into_inner() }
}

/// A partial data of a state, which should be point to the part data of the
/// state.
#[derive(Clone)]
pub enum PartData<T> {
  PartRef(NonNull<T>),
  PartData(T),
}

impl<T> PartData<T> {
  /// Create a `PartData` from a reference.
  pub fn from_ref(v: &T) -> Self { PartData::PartRef(NonNull::from(v)) }

  /// Create a `PartData` from a mutable reference.
  pub fn from_ref_mut(v: &mut T) -> Self { PartData::PartRef(NonNull::from(v)) }

  /// Create a `PartData` from a type that should be point to the part data not
  /// a copy. E.g. Option<&T>, `Box`, `Arc`, `Rc`, etc.
  ///
  /// Caller should ensure that the data is not a copy.
  pub fn from_data(ptr_data: T) -> Self { PartData::PartData(ptr_data) }
}
pub(crate) struct ValueRef<'a, T> {
  pub(crate) value: PartData<T>,
  pub(crate) borrow: BorrowRef<'a>,
}

pub(crate) struct ValueMutRef<'a, T> {
  pub(crate) value: PartData<T>,
  pub(crate) borrow: BorrowRefMut<'a>,
}

pub(crate) struct BorrowRefMut<'b> {
  borrow: &'b Cell<BorrowFlag>,
}

pub(crate) struct BorrowRef<'b> {
  borrow: &'b Cell<BorrowFlag>,
}

impl<T> Deref for PartData<T> {
  type Target = T;
  fn deref(&self) -> &Self::Target {
    match self {
      PartData::PartRef(ptr) => unsafe { ptr.as_ref() },
      PartData::PartData(data) => data,
    }
  }
}

impl<T> DerefMut for PartData<T> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    match self {
      PartData::PartRef(ptr) => unsafe { ptr.as_mut() },
      PartData::PartData(data) => data,
    }
  }
}

impl Drop for BorrowRefMut<'_> {
  #[inline]
  fn drop(&mut self) {
    let borrow = self.borrow.get();
    debug_assert!(is_writing(borrow));
    self.borrow.set(borrow + 1);
  }
}

impl Drop for BorrowRef<'_> {
  #[inline]
  fn drop(&mut self) {
    let borrow = self.borrow.get();
    debug_assert!(is_reading(borrow));
    self.borrow.set(borrow - 1);
  }
}

impl<'b> BorrowRefMut<'b> {
  // Clones a `BorrowRefMut`.
  //
  // This is only valid if each `BorrowRefMut` is used to track a mutable
  // reference to a distinct, nonoverlapping range of the original object.
  // This isn't in a Clone impl so that code doesn't call this implicitly.
  #[inline]
  pub(crate) fn clone(&self) -> BorrowRefMut<'b> {
    let borrow = self.borrow.get();
    debug_assert!(is_writing(borrow));
    // Prevent the borrow counter from underflowing.
    assert!(borrow != BorrowFlag::MIN);
    self.borrow.set(borrow - 1);
    BorrowRefMut { borrow: self.borrow }
  }
}

impl BorrowRef<'_> {
  #[inline]
  pub(crate) fn clone(&self) -> Self {
    // Since this Ref exists, we know the borrow flag
    // is a reading borrow.
    let borrow = self.borrow.get();
    debug_assert!(is_reading(borrow));
    // Prevent the borrow counter from overflowing into
    // a writing borrow.
    assert!(borrow != BorrowFlag::MAX);
    self.borrow.set(borrow + 1);
    BorrowRef { borrow: self.borrow }
  }
}

impl<'a, T> Deref for ValueRef<'a, T> {
  type Target = T;
  #[inline]
  fn deref(&self) -> &Self::Target { &self.value }
}

impl<'a, T> Deref for ValueMutRef<'a, T> {
  type Target = T;
  #[inline]
  fn deref(&self) -> &Self::Target { &self.value }
}

impl<'a, T> DerefMut for ValueMutRef<'a, T> {
  #[inline]
  fn deref_mut(&mut self) -> &mut Self::Target { &mut self.value }
}
