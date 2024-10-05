use std::{cell::Cell, thread::ThreadId};

/// This wrapper ensures that the data can only be accessed in the thread that
/// initially utilizes it.
///
/// Any attempt to access it from another thread will result in a panic.
pub struct LocalSender<T> {
  data: *mut T,
  id: Cell<ThreadId>,
}

impl<T> LocalSender<T> {
  pub fn new(data: T) -> Self {
    let id = Cell::new(std::thread::current().id());
    Self { data: Box::into_raw(Box::new(data)), id }
  }

  pub fn reset(&self) { self.id.set(std::thread::current().id()); }

  pub fn take(&self)
  where
    T: Default,
  {
    unsafe { self.data.replace(<_>::default()) };
  }

  #[track_caller]
  fn assert_same_thread(&self) {
    assert_eq!(
      self.id.get(),
      std::thread::current().id(),
      "Access is only supported within the same thread."
    );
  }
}

impl<T> std::ops::Deref for LocalSender<T> {
  type Target = T;

  fn deref(&self) -> &Self::Target {
    self.assert_same_thread();
    unsafe { &*self.data }
  }
}

impl<T> std::ops::DerefMut for LocalSender<T> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    self.assert_same_thread();
    unsafe { &mut *self.data }
  }
}

impl<T> Drop for LocalSender<T> {
  fn drop(&mut self) {
    self.assert_same_thread();
    let _drop = unsafe { Box::from_raw(self.data) };
  }
}

unsafe impl<T> Send for LocalSender<T> {}
unsafe impl<T> Sync for LocalSender<T> {}
