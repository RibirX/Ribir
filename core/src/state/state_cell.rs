//! This implementation is a fork from `std::cell::RefCell`, allowing us to
//! manage the borrow flag.
use std::{
  cell::{Cell, UnsafeCell},
  marker::PhantomData,
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
  pub(crate) fn read(&self) -> ReadRef<W> {
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
    let inner = InnerPart::Ref(unsafe { NonNull::new_unchecked(self.data.get()) });
    ReadRef { inner, borrow: BorrowRef { borrow } }
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
    let inner = InnerPart::Ref(unsafe { NonNull::new_unchecked(self.data.get()) });
    ValueMutRef { inner, borrow: v_ref }
  }

  pub(crate) fn is_unused(&self) -> bool { self.borrow_flag.get() == UNUSED }

  pub(super) fn into_inner(self) -> W { self.data.into_inner() }
}

/// A partial reference value of a state, which should be point to the part data
/// of the state.
#[derive(Clone)]
pub struct PartRef<'a, T: ?Sized> {
  pub(crate) inner: InnerPart<T>,
  _phantom: PhantomData<&'a T>,
}

/// A partial mutable reference value of a state, which should be point to the
/// part data of the state.
#[derive(Clone)]
pub struct PartMut<'a, T: ?Sized> {
  pub(crate) inner: InnerPart<T>,
  _phantom: PhantomData<&'a mut T>,
}

#[derive(Clone)]
pub(crate) enum InnerPart<T: ?Sized> {
  Ref(NonNull<T>),
  // Box the `T` to allow it to be `?Sized`.
  Ptr(Box<T>),
}

impl<'a, T: ?Sized> PartRef<'a, T> {
  /// Create a `PartRef` from a reference.
  pub fn new(v: &T) -> Self {
    Self { inner: InnerPart::Ref(NonNull::from(v)), _phantom: PhantomData }
  }
}

impl<'a, T> PartRef<'a, T> {
  /// Create a `PartRef` from a pointer that points to the part data of the
  /// original data. For example, `Option<&T>`, `Box`, `Arc`, `Rc`, etc.
  ///
  /// The data used to create this `PartRef` must point to the data in your
  /// original data.
  ///
  ///
  /// # Example
  ///
  /// ```
  /// use ribir_core::prelude::*;
  ///
  /// let vec = Stateful::new(vec![1, 2, 3]);
  /// // We get the state of the second element.
  /// // `v.get(1)` returns an `Option<&i32>`, which is valid in the vector.
  /// let elem2 = vec.part_reader(|v| unsafe {
  ///   PartRef::from_ptr(std::mem::transmute::<_, Option<&'static i32>>(v.get(1)))
  /// });
  /// ```
  ///
  /// # Safety
  ///
  /// Exercise caution when using this method, as it can lead to dangling
  /// pointers in the state reference internals.
  /// ```
  /// use ribir_core::prelude::*;
  ///
  /// let ab = Stateful::new((1, 2));
  ///
  /// let ab2 = ab.part_reader(|v| unsafe { PartRef::from_ptr(*v) });
  ///
  /// // The `_a` may result in a dangling pointer issue since it utilizes the
  /// // value of `ab2.read()`. However, `ab2` copies the value of `ab` rather
  /// // than referencing it. When `ab2.read()` is dropped, `_a` still points to
  /// // it, making access to `_a` dangerous.
  /// let _a = ReadRef::map(ab2.read(), |v| unsafe { PartRef::from_ptr(v.0) });
  /// ```
  pub unsafe fn from_ptr(ptr_data: T) -> Self {
    Self { inner: InnerPart::Ptr(Box::new(ptr_data)), _phantom: PhantomData }
  }
}

impl<'a, T: ?Sized> PartMut<'a, T> {
  /// Create a `PartMut` from a mutable reference.
  pub fn new(v: &mut T) -> Self {
    Self { inner: InnerPart::Ref(NonNull::from(v)), _phantom: PhantomData }
  }
}

impl<'a, T> PartMut<'a, T> {
  /// Create a `PartMut` from a pointer that points to the part data of the
  /// original data. For example, `Option<&T>`, `Box`, `Arc`, `Rc`, etc.
  ///
  /// The data used to create this `PartMut` must point to the data in your
  /// original data.
  ///
  ///
  /// # Example
  ///
  /// ```
  /// use ribir_core::prelude::*;
  ///
  /// let vec = Stateful::new(vec![1, 2, 3]);
  /// // We get the state of the second element.
  /// // `v.get_mut(1)` returns an `Option<&mut i32>`, which is valid in the vector.
  /// let elem2 = vec.part_writer(PartialId::any(), |v| unsafe {
  ///   PartMut::from_ptr(std::mem::transmute::<_, Option<&'static i32>>(v.get_mut(1)))
  /// });
  /// ```
  ///
  /// # Safety
  ///
  /// Exercise caution when using this method, as it can lead to dangling
  /// pointers in the state reference internals.
  /// ```
  /// use ribir_core::prelude::*;
  ///
  /// let ab = Stateful::new((1, 2));
  ///
  /// let ab2 = ab.part_writer(PartialId::any(), |v| unsafe { PartMut::from_ptr(*v) });
  ///
  /// // The `_a` may result in a dangling pointer issue since it utilizes the
  /// // value of `ab2.write()`. However, `ab2` copies the value of `ab` rather
  /// // than referencing it. When `ab2.write()` is dropped, `_a` still points
  /// // to it, making access to `_a` dangerous.
  /// let _a = WriteRef::map(ab2.write(), |v| unsafe { PartMut::from_ptr(v.0) });
  /// ```
  ///
  /// Otherwise, your modifications will not be applied to the state.
  /// ```
  /// use ribir_core::prelude::*;
  ///
  /// let vec = Stateful::new(vec![1, 2, 3]);
  ///
  /// // We create a state of the second element. However, this state is a copy of
  /// // the vector because `v[1]` returns a copy of the value in the vector, not a
  /// // reference.
  /// let mut elem2 = vec.part_writer(PartialId::any(), |v| unsafe { PartMut::from_ptr(v[1]) });
  ///
  /// // This modification will not alter the `vec`.
  /// *elem2.write() = 20;
  /// ```
  pub unsafe fn from_ptr(ptr_data: T) -> Self {
    Self { inner: InnerPart::Ptr(Box::new(ptr_data)), _phantom: PhantomData }
  }
}

pub struct ReadRef<'a, T: ?Sized> {
  pub(crate) inner: InnerPart<T>,
  pub(crate) borrow: BorrowRef<'a>,
}

pub struct ValueMutRef<'a, T: ?Sized> {
  pub(crate) inner: InnerPart<T>,
  pub(crate) borrow: BorrowRefMut<'a>,
}

pub(crate) struct BorrowRefMut<'b> {
  borrow: &'b Cell<BorrowFlag>,
}

pub(crate) struct BorrowRef<'b> {
  borrow: &'b Cell<BorrowFlag>,
}

impl<T: ?Sized> Deref for InnerPart<T> {
  type Target = T;
  fn deref(&self) -> &Self::Target {
    match self {
      InnerPart::Ref(ptr) => unsafe { ptr.as_ref() },
      InnerPart::Ptr(data) => data,
    }
  }
}

impl<T: ?Sized> DerefMut for InnerPart<T> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    match self {
      InnerPart::Ref(ptr) => unsafe { ptr.as_mut() },
      InnerPart::Ptr(data) => data,
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

impl<'a, V: ?Sized> ReadRef<'a, V> {
  /// Make a new `ReadRef` by mapping the value of the current `ReadRef`.
  pub fn map<U: ?Sized>(r: ReadRef<'a, V>, f: impl FnOnce(&V) -> PartRef<U>) -> ReadRef<'a, U> {
    ReadRef { inner: f(&r.inner).inner, borrow: r.borrow }
  }

  /// Makes a new `ReadRef` for an optional component of the borrowed data. The
  /// original guard is returned as an `Err(..)` if the closure returns
  /// `None`.
  ///
  /// This is an associated function that needs to be used as
  /// `ReadRef::filter_map(...)`. A method would interfere with methods of the
  /// same name on `T` used through `Deref`.
  ///
  /// # Examples
  ///
  /// ```
  /// use ribir_core::prelude::*;
  ///
  /// let c = Stateful::new(vec![1, 2, 3]);
  /// let b1: ReadRef<'_, Vec<u32>> = c.read();
  /// let b2: Result<ReadRef<'_, u32>, _> = ReadRef::filter_map(b1, |v| v.get(1).map(PartRef::new));
  /// assert_eq!(*b2.unwrap(), 2);
  /// ```
  pub fn filter_map<U: ?Sized, M>(
    orig: ReadRef<'a, V>, part_map: M,
  ) -> std::result::Result<ReadRef<'a, U>, Self>
  where
    M: Fn(&V) -> Option<PartRef<U>>,
  {
    match part_map(&orig.inner) {
      Some(inner) => Ok(ReadRef { inner: inner.inner, borrow: orig.borrow }),
      None => Err(orig),
    }
  }

  /// Split the current `ReadRef` into two `ReadRef`s by mapping the value to
  /// two parts.
  pub fn map_split<U: ?Sized, W: ?Sized>(
    orig: ReadRef<'a, V>, f: impl FnOnce(&V) -> (PartRef<U>, PartRef<W>),
  ) -> (ReadRef<'a, U>, ReadRef<'a, W>) {
    let (a, b) = f(&*orig);
    let borrow = orig.borrow.clone();

    (ReadRef { inner: a.inner, borrow: borrow.clone() }, ReadRef { inner: b.inner, borrow })
  }

  pub(crate) fn mut_as_ref_map<U: ?Sized>(
    orig: ReadRef<'a, V>, f: impl FnOnce(&mut V) -> PartMut<U>,
  ) -> ReadRef<'a, U> {
    let ReadRef { mut inner, borrow } = orig;
    let value = f(&mut inner);
    ReadRef { inner: value.inner, borrow }
  }
}

impl<'a, T: ?Sized> Deref for ReadRef<'a, T> {
  type Target = T;
  #[inline]
  fn deref(&self) -> &Self::Target { &self.inner }
}

impl<'a, T: ?Sized> Deref for ValueMutRef<'a, T> {
  type Target = T;
  #[inline]
  fn deref(&self) -> &Self::Target { &self.inner }
}

impl<'a, T: ?Sized> DerefMut for ValueMutRef<'a, T> {
  #[inline]
  fn deref_mut(&mut self) -> &mut Self::Target { &mut self.inner }
}

use std::fmt::*;

use super::{QueryRef, WriteRef};

impl<T: ?Sized + Debug> Debug for ReadRef<'_, T> {
  fn fmt(&self, f: &mut Formatter<'_>) -> Result { Debug::fmt(&**self, f) }
}

impl<T: ?Sized + Debug> Debug for WriteRef<'_, T> {
  fn fmt(&self, f: &mut Formatter<'_>) -> Result { Debug::fmt(&**self, f) }
}

impl<T: ?Sized + Debug> Debug for QueryRef<'_, T> {
  fn fmt(&self, f: &mut Formatter<'_>) -> Result { Debug::fmt(&**self, f) }
}

impl<'a, T: ?Sized> From<PartMut<'a, T>> for PartRef<'a, T> {
  fn from(part: PartMut<T>) -> Self { Self { inner: part.inner, _phantom: PhantomData } }
}

impl<'a, T> From<&'a T> for PartRef<'a, T> {
  fn from(part: &'a T) -> Self { PartRef::new(part) }
}

impl<'a, T> From<&'a mut T> for PartMut<'a, T> {
  fn from(part: &'a mut T) -> Self { PartMut::new(part) }
}
