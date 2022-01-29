use std::{fmt::Debug, hash::Hash, rc::Rc};

use crate::DeviceSize;

pub trait Image {
  fn color_format(&self) -> ColorFormat;
  fn size(&self) -> DeviceSize;
  fn pixel_bytes(&self) -> Box<[u8]>;
}

// todo: remove? only rgba support?
pub enum ColorFormat {
  Rgba8,
}

/// A image wrap for shallow compare.
#[derive(Clone)]
pub struct ShallowImage(Rc<dyn Image>);

impl Hash for ShallowImage {
  #[inline]
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    let ptr = Rc::as_ptr(&self.0);
    ptr.hash(state);
  }
}

impl PartialEq for ShallowImage {
  #[inline]
  fn eq(&self, other: &Self) -> bool { Rc::ptr_eq(&self.0, &other.0) }
}

impl Eq for ShallowImage {}

impl Debug for ShallowImage {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let DeviceSize { width, height, .. } = self.size();
    f.debug_tuple("ShallowImage")
      .field(&format!("{width}x{height}"))
      .finish()
  }
}
impl ShallowImage {
  #[inline]
  pub fn new(img: Rc<dyn Image>) -> Self { Self(img) }
}

impl std::ops::Deref for ShallowImage {
  type Target = Rc<dyn Image>;

  #[inline]
  fn deref(&self) -> &Self::Target { &self.0 }
}
