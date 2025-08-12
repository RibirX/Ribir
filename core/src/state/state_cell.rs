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
  pub(crate) fn read(&self) -> ReadRef<'_, W> {
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
    ReadRef { inner, borrow: BorrowRef { borrow }, origin_store: <_>::default() }
  }

  pub(crate) fn write(&self) -> ValueMutRef<'_, W> {
    // NOTE: Unlike BorrowRefMut::clone, new is called to create the initial
    // mutable reference, and so there must currently be no existing
    // references. Thus, while clone increments the mutable refcount, here
    // we explicitly only allow going from UNUSED to UNUSED - 1.
    if !self.is_unused() {
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

    let borrow = &self.borrow_flag;
    borrow.set(UNUSED - 1);
    let v_ref = BorrowRefMut { borrow };
    let inner = InnerPart::Ref(unsafe { NonNull::new_unchecked(self.data.get()) });
    ValueMutRef { inner, borrow: v_ref, origin_store: <_>::default() }
  }

  pub(crate) fn is_unused(&self) -> bool { self.borrow_flag.get() == UNUSED }

  pub(super) fn into_inner(self) -> W { self.data.into_inner() }
}

/// A partial reference value of a state, which should be point to the part data
/// of the state.
pub struct PartRef<'a, T: ?Sized> {
  pub(crate) inner: InnerPart<T>,
  _phantom: PhantomData<&'a T>,
}

/// A partial mutable reference value of a state, which should be point to the
/// part data of the state.
pub struct PartMut<'a, T: ?Sized> {
  pub(crate) inner: InnerPart<T>,
  _phantom: PhantomData<&'a mut T>,
}

pub(crate) enum InnerPart<T: ?Sized> {
  Ref(NonNull<T>),
  Owned(Box<T>),
}

impl<'a, T: ?Sized> PartRef<'a, T> {
  /// Create a `PartRef` from a reference.
  pub fn new(v: &T) -> Self {
    Self { inner: InnerPart::Ref(NonNull::from(v)), _phantom: PhantomData }
  }
}

impl<'a, T> PartRef<'a, T> {
  /// Create a `PartRef` from a value that is the part data of the
  /// original data. For example, `Option<&T>`, `Box`, `Arc`, `Rc`, etc.
  ///
  /// # Example
  ///
  /// ```
  /// use ribir_core::prelude::*;
  ///
  /// let vec = Stateful::new(vec![1, 2, 3]);
  /// // We get the state of the second element.
  /// // `v.get(1)` returns an `Option<&i32>`, which is valid in the vector.
  /// let elem2 = vec.part_reader(|v| {
  ///   PartRef::from_value(
  ///     // Safety: We cannot mark higher-ranked lifetimes for the closure to
  ///     // hint that we have a valid lifetime.
  ///     // See [Rust issue #58052](https://github.com/rust-lang/rust/issues/58052)
  ///     // for more details.
  ///     unsafe { std::mem::transmute::<_, Option<&i32>>(v.get(1)) },
  ///   )
  /// });
  /// ```
  pub fn from_value(value: T) -> Self {
    Self { inner: InnerPart::Owned(Box::new(value)), _phantom: PhantomData }
  }
}

impl<'a, T: ?Sized> PartMut<'a, T> {
  /// Create a `PartMut` from a mutable reference.
  pub fn new(v: &mut T) -> Self {
    Self { inner: InnerPart::Ref(NonNull::from(v)), _phantom: PhantomData }
  }
}

impl<'a, T> PartMut<'a, T> {
  /// Creates a `PartMut` from a pointer to part data in the original source.
  /// Accepts smart pointers (`Box<T>`, `Rc<T>`, `Arc<T>`),
  /// or optional references (`Option<&mut T>`) and so on.
  ///
  /// # Safety
  ///
  /// The pointer **must** directly reference the original data. Providing a
  /// value instead of a pointer will create disconnected data that won't
  /// update the source.
  ///
  /// # Example (Correct)
  ///
  /// ```rust
  /// use ribir_core::prelude::*;
  ///
  /// let mut vec = Stateful::new(vec![1, 2, 3]);
  /// let elem2 = vec.part_writer(PartialId::any(), |v| {
  ///   // Valid: v.get_mut(1) returns Option<&mut i32> (a pointer)
  ///   PartMut::from_ptr(
  ///     // Safety: We cannot mark higher-ranked lifetimes for the closure to
  ///     // hint that we have a valid lifetime.
  ///     // See [Rust issue #58052](https://github.com/rust-lang/rust/issues/58052)
  ///     // for more details.
  ///     unsafe { std::mem::transmute::<_, Option<&mut i32>>(v.get_mut(1)) },
  ///   )
  /// });
  /// ```
  ///
  /// # Example (Incorrect)
  /// ```rust
  /// use ribir_core::prelude::*;
  ///
  /// let mut vec = Stateful::new(vec![1, 2, 3]);
  /// let mut elem2 = vec.part_writer(
  ///   PartialId::any(),
  ///   // INVALID: v[1] returns i32 (a value copy)
  ///   |v| PartMut::from_ptr(v[1]), // Disconnected from original
  /// );
  ///
  /// *elem2.write() = 20; // Modifies copy, not original vec
  /// ```
  pub fn from_ptr(pointer: T) -> Self {
    Self { inner: InnerPart::Owned(Box::new(pointer)), _phantom: PhantomData }
  }
}

pub struct ReadRef<'a, T: ?Sized> {
  pub(crate) inner: InnerPart<T>,
  pub(crate) borrow: BorrowRef<'a>,
  pub(crate) origin_store: OriginPartStore<'a>,
}

pub(crate) struct ValueMutRef<'a, T: ?Sized> {
  pub(crate) inner: InnerPart<T>,
  pub(crate) borrow: BorrowRefMut<'a>,
  pub(crate) origin_store: OriginPartStore<'a>,
}

#[derive(Clone)]
pub(crate) struct BorrowRefMut<'b> {
  borrow: &'b Cell<BorrowFlag>,
}

pub(crate) struct BorrowRef<'b> {
  borrow: &'b Cell<BorrowFlag>,
}
/// A type used to store the temporary data of a part state for the `ReadRef` or
/// `ValueMutRef` so that it can be kept until the end of the `ReadRef` or
/// `ValueMutRef`.
#[derive(Default)]
pub(crate) struct OriginPartStore<'a>(Option<Box<dyn FnOnce() + 'a>>);

impl<'a> OriginPartStore<'a> {
  pub(crate) fn add<T: ?Sized + 'a>(&mut self, origin: InnerPart<T>) {
    let InnerPart::Owned(data) = origin else { return };
    if let Some(free) = self.0.take() {
      self.0 = Some(Box::new(move || {
        free();
        drop(data);
      }));
    } else {
      self.0 = Some(Box::new(move || drop(data)));
    }
  }
}

impl<T: ?Sized> Deref for InnerPart<T> {
  type Target = T;
  fn deref(&self) -> &Self::Target {
    match self {
      InnerPart::Ref(ptr) => unsafe { ptr.as_ref() },
      InnerPart::Owned(data) => data,
    }
  }
}

impl<'a> Drop for OriginPartStore<'a> {
  #[inline]
  fn drop(&mut self) {
    if let Some(free) = self.0.take() {
      free();
    }
  }
}

impl<T: ?Sized> DerefMut for InnerPart<T> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    match self {
      InnerPart::Ref(ptr) => unsafe { ptr.as_mut() },
      InnerPart::Owned(data) => data,
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

impl<'a, V: ?Sized + 'a> ReadRef<'a, V> {
  /// Make a new `ReadRef` by mapping the value of the current `ReadRef`.
  pub fn map<U: ?Sized>(r: ReadRef<'a, V>, f: impl FnOnce(&V) -> PartRef<U>) -> ReadRef<'a, U> {
    let Self { inner, borrow, origin_store: mut orig_store } = r;
    let part = f(&inner).inner;
    orig_store.add(inner);
    ReadRef { inner: part, borrow, origin_store: orig_store }
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
    match part_map(&orig.inner).map(|v| v.inner) {
      Some(part) => {
        let Self { inner, borrow, origin_store: mut orig_store } = orig;
        orig_store.add(inner);
        Ok(ReadRef { inner: part, borrow, origin_store: orig_store })
      }
      None => Err(orig),
    }
  }

  pub(crate) fn mut_as_ref_map<U: ?Sized>(
    orig: ReadRef<'a, V>, f: impl FnOnce(&mut V) -> PartMut<U>,
  ) -> ReadRef<'a, U> {
    let ReadRef { mut inner, borrow, origin_store: mut orig_store } = orig;
    let value = f(&mut inner).inner;
    orig_store.add(inner);
    ReadRef { inner: value, borrow, origin_store: orig_store }
  }
}

impl<'a, V: ?Sized + 'a> ValueMutRef<'a, V> {
  pub(crate) fn map<U: ?Sized>(self, f: impl FnOnce(&mut V) -> PartMut<U>) -> ValueMutRef<'a, U> {
    let ValueMutRef { mut inner, origin_store: mut orig_store, borrow } = self;
    let value = f(&mut inner).inner;
    orig_store.add(inner);
    ValueMutRef { inner: value, origin_store: orig_store, borrow }
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
