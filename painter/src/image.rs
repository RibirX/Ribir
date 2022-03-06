use std::{borrow::Cow, fmt::Debug, hash::Hash, rc::Rc};

#[derive(Clone, Copy)]
pub enum ColorFormat {
  Rgba8,
}

impl ColorFormat {
  /// return have many bytes per pixel need
  pub const fn pixel_per_bytes(&self) -> u8 {
    match self {
      ColorFormat::Rgba8 => 4,
    }
  }
}

pub struct PixelImage {
  data: Cow<'static, [u8]>,
  size: (u16, u16),
  format: ColorFormat,
}

impl PixelImage {
  #[inline]
  pub fn new(data: Cow<'static, [u8]>, width: u16, height: u16, format: ColorFormat) -> Self {
    PixelImage { data, size: (width, height), format }
  }
  #[inline]
  pub fn color_format(&self) -> ColorFormat { self.format }
  #[inline]
  pub fn size(&self) -> (u16, u16) { self.size }
  #[inline]
  pub fn pixel_bytes(&self) -> &[u8] { &self.data }
}

/// A image wrap for shallow compare.
#[derive(Clone)]
pub struct ShallowImage(Rc<PixelImage>);

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
    let (width, height) = self.size;
    f.debug_tuple("ShallowImage")
      .field(&format!("{width}x{height}"))
      .finish()
  }
}

impl ShallowImage {
  #[inline]
  pub fn new(img: PixelImage) -> Self { Self(Rc::new(img)) }
}

impl std::ops::Deref for ShallowImage {
  type Target = Rc<PixelImage>;

  #[inline]
  fn deref(&self) -> &Self::Target { &self.0 }
}
