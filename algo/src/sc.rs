use std::{
  any::Any,
  cell::Cell,
  fmt::{Debug, Display, Formatter, Pointer},
  ptr::NonNull,
};

/// A single-thread smart pointer with strong reference count only.
/// This is a simplified version of `std::rc::Sc` with the weak reference count.
/// Use it when you are sure that there is no cycle in the reference graph or in
/// a inner resource manage that will break the cycle by itself.
pub struct Sc<T: ?Sized>(NonNull<ScBox<T>>);

struct ScBox<T: ?Sized> {
  ref_cnt: Cell<usize>,
  value: T,
}

impl<T> Sc<T> {
  /// Constructs a new `Sc<T>`.
  ///
  /// # Examples
  ///
  /// ```
  /// use ribir_algo::Sc;
  ///
  /// let five = Sc::new(5);
  /// ```
  #[inline]
  pub fn new(value: T) -> Self {
    Self::from_inner(Box::leak(Box::new(ScBox { ref_cnt: Cell::new(1), value })).into())
  }

  /// Returns the inner value, if the `Sc` has exactly one strong reference.
  ///
  /// Otherwise, an [`Err`] is returned with the same `Sc` that was
  /// passed in.
  ///
  /// This will succeed even if there are outstanding weak references.
  ///
  /// # Examples
  ///
  /// ```
  /// use ribir_algo::Sc;
  ///
  /// let x = Sc::new(3);
  /// assert_eq!(Sc::try_unwrap(x), Ok(3));
  ///
  /// let x = Sc::new(4);
  /// let _y = Sc::clone(&x);
  /// assert_eq!(*Sc::try_unwrap(x).unwrap_err(), 4);
  /// ```
  #[inline]
  pub fn try_unwrap(this: Self) -> Result<T, Self> {
    if Sc::ref_count(&this) == 1 {
      unsafe {
        let val = std::ptr::read(&*this); // copy the contained object

        // avoid to call `drop` but release the memory.
        let layout = std::alloc::Layout::for_value(this.0.as_ref());
        let ptr = this.0.as_ptr();
        std::mem::forget(this);
        std::alloc::dealloc(ptr as *mut _, layout);

        Ok(val)
      }
    } else {
      Err(this)
    }
  }
}

impl Sc<dyn Any> {
  ///
  /// todo: prefer implement `CoerceUnsized` if it stable.
  #[inline]
  pub fn new_any<T: Any>(value: T) -> Self {
    let inner: Box<ScBox<dyn Any>> = Box::new(ScBox { ref_cnt: Cell::new(1), value });
    Self::from_inner(Box::leak(inner).into())
  }
}

impl<T: ?Sized> Sc<T> {
  // Gets the number of pointers to this allocation.
  ///
  /// # Examples
  ///
  /// ```
  /// use ribir_algo::Sc;
  ///
  /// let five = Sc::new(5);
  /// let _also_five = Sc::clone(&five);
  ///
  /// assert_eq!(2, Sc::ref_count(&five));
  /// ```
  #[inline]
  pub fn ref_count(&self) -> usize { self.inner().ref_cnt() }

  /// Returns `true` if the two `Sc`s point to the same allocation in a vein
  /// similar to [`ptr::eq`]. See [that function][`ptr::eq`] for caveats when
  /// comparing `dyn Trait` pointers.
  ///
  /// # Examples
  ///
  /// ```
  /// use ribir_algo::Sc;
  ///
  /// let five = Sc::new(5);
  /// let same_five = Sc::clone(&five);
  /// let other_five = Sc::new(5);
  ///
  /// assert!(Sc::ptr_eq(&five, &same_five));
  /// assert!(!Sc::ptr_eq(&five, &other_five));
  /// ```
  pub fn ptr_eq(this: &Self, other: &Self) -> bool {
    std::ptr::addr_eq(this.0.as_ptr(), other.0.as_ptr())
  }

  fn from_inner(ptr: NonNull<ScBox<T>>) -> Self { Self(ptr) }

  fn inner(&self) -> &ScBox<T> {
    // Safety: we're guaranteed that the inner pointer is valid when the `Sc` is
    // alive
    unsafe { self.0.as_ref() }
  }
}

impl Sc<dyn Any> {
  /// Attempt to downcast the `Sc<dyn Any>` to a concrete type.
  ///
  /// # Examples
  ///
  /// ```
  /// use std::any::Any;
  ///
  /// use ribir_algo::Sc;
  ///
  /// fn print_if_string(value: Sc<dyn Any>) {
  ///   if let Ok(string) = value.downcast::<String>() {
  ///     println!("String ({}): {}", string.len(), string);
  ///   }
  /// }
  ///
  /// let my_string = "Hello World".to_string();
  /// print_if_string(Sc::new_any(my_string));
  /// print_if_string(Sc::new_any(0i8));
  /// ```
  pub fn downcast<T: Any>(self) -> Result<Sc<T>, Sc<dyn Any>> {
    if (*self).is::<T>() {
      let ptr = self.0.cast::<ScBox<T>>();
      std::mem::forget(self);
      Ok(Sc::from_inner(ptr))
    } else {
      Err(self)
    }
  }
}

impl<T: ?Sized> ScBox<T> {
  fn inc(&self) { self.ref_cnt.set(self.ref_cnt.get() + 1); }
  fn dec(&self) { self.ref_cnt.set(self.ref_cnt.get() - 1) }
  fn ref_cnt(&self) -> usize { self.ref_cnt.get() }
}

impl<T: ?Sized> std::ops::Deref for Sc<T> {
  type Target = T;
  #[inline]
  fn deref(&self) -> &Self::Target { &self.inner().value }
}

impl<T: ?Sized> Drop for Sc<T> {
  fn drop(&mut self) {
    self.inner().dec();
    if self.inner().ref_cnt() == 0 {
      unsafe {
        let layout = std::alloc::Layout::for_value(self.0.as_ref());
        let ptr = self.0.as_ptr();
        std::ptr::drop_in_place(ptr);
        std::alloc::dealloc(ptr as *mut _, layout)
      }
    }
  }
}

impl<T> Clone for Sc<T> {
  #[inline]
  fn clone(&self) -> Self {
    self.inner().inc();
    Self(self.0)
  }
}

impl<T: ?Sized + Display> Display for Sc<T> {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { Display::fmt(&**self, f) }
}

impl<T: ?Sized + Debug> Debug for Sc<T> {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { Debug::fmt(&**self, f) }
}

impl<T: ?Sized> Pointer for Sc<T> {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    Pointer::fmt(&(&**self as *const T), f)
  }
}

impl<T: Default> Default for Sc<T> {
  #[inline]
  fn default() -> Sc<T> { Sc::new(Default::default()) }
}

impl<T: ?Sized + PartialEq> PartialEq for Sc<T> {
  #[inline]
  fn eq(&self, other: &Sc<T>) -> bool { **self == **other }
}

#[cfg(test)]
mod tests {
  use super::*;
  #[test]
  fn test_sc() {
    let a = Sc::new(1);
    assert_eq!(Sc::ref_count(&a), 1);
    let b = Sc::clone(&a);
    assert_eq!(Sc::ref_count(&b), 2);
    drop(a);
    assert_eq!(Sc::ref_count(&b), 1);
    drop(b);
  }
}
