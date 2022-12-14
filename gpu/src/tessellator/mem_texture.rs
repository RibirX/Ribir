use lyon_tessellation::geom::euclid::UnknownUnit;
use zerocopy::AsBytes;
pub type Rect = lyon_tessellation::geom::euclid::Rect<u16, UnknownUnit>;
pub type Size = lyon_tessellation::geom::Size<u16>;
pub struct MemTexture<const N: usize> {
  max_size: Size,
  size: Size,
  array: Box<[u8]>,
  updated: bool,
}

impl<const N: usize> MemTexture<N> {
  pub fn new(size: Size, max_size: Size) -> Self {
    Self {
      size,
      max_size,
      array: Self::alloc_mem(size.width, size.height),
      updated: false,
    }
  }

  #[inline]
  pub fn is_updated(&self) -> bool { self.updated }

  #[inline]
  pub fn size(&self) -> Size { self.size }

  /// Expand the texture.
  /// Return true if expand successful, false if this texture reach the limit.
  /// the old data will all drop.
  pub fn expand_size(&mut self) -> bool {
    let old_size = self.size;
    let success = self.max_size.greater_than(old_size).any();
    if success {
      if old_size.height * 2 < self.max_size.height {
        self.size.height *= 2;
      }
      if old_size.width * 2 < self.max_size.width {
        self.size.width *= 2;
      }

      let old = std::mem::replace(
        &mut self.array,
        Self::alloc_mem(self.size.width, self.size.height),
      );
      self.write_rect(&Rect::from_size(old_size), &old);
      self.updated = true;
    }
    success
  }

  #[inline]
  pub fn as_bytes(&self) -> &[u8] { self.array.as_bytes() }

  /// Use `data` to fill the `rect` sub range to this 2d array, `rect` should
  /// not over this 2d array's boundary.
  pub fn write_rect(&mut self, rect: &Rect, data: &[u8]) {
    debug_assert_eq!(
      rect.width() as usize * rect.height() as usize * N,
      data.len()
    );
    debug_assert!(Rect::from_size(self.size).contains_rect(rect));
    let rect = rect.to_usize();
    let row_bytes = rect.width() * N;
    let x_range = rect.min_x() * N..rect.max_x() * N;
    rect.y_range().enumerate().for_each(|(idx, y)| {
      let offset = idx * row_bytes;
      self[y][x_range.clone()].copy_from_slice(&data[offset..offset + row_bytes]);
    });

    self.updated = true;
  }

  /// Call this method after synced this texture data to a real texture of
  /// render engine. Tell this mem texture know, data has sync to render engine.
  pub fn data_synced(&mut self) { self.updated = false; }

  // The max size the texture can grow
  pub fn max_size(&self) -> Size { self.max_size }

  fn alloc_mem(width: u16, height: u16) -> Box<[u8]> {
    let bytes = width as usize * height as usize * N;
    vec![0; bytes].into_boxed_slice()
  }
}

impl<const N: usize> std::ops::Index<usize> for MemTexture<N> {
  type Output = [u8];
  fn index(&self, index: usize) -> &Self::Output {
    let row_bytes = self.size.width as usize * N;
    let array_offset = index * row_bytes;
    &self.array[array_offset..array_offset + row_bytes]
  }
}

impl<const N: usize> std::ops::IndexMut<usize> for MemTexture<N> {
  fn index_mut(&mut self, index: usize) -> &mut Self::Output {
    let row_bytes = self.size.width as usize * N;
    let array_offset = index * row_bytes;
    &mut self.array[array_offset..array_offset + row_bytes]
  }
}

#[cfg(test)]
mod tests {

  use super::*;
  #[test]
  fn update_texture() {
    let mut tex = MemTexture::<1>::new(Size::new(8, 8), Size::new(512, 512));

    tex.write_rect(&Rect::new((0, 0).into(), Size::new(2, 1)), &[0, 1]);
    assert_eq!(&tex[0][0..4], &[00, 1, 0, 0]);

    tex.write_rect(&Rect::new((3, 7).into(), Size::new(2, 1)), &[73, 74]);
    assert_eq!(tex[7][3], 73);

    tex.write_rect(
      &Rect::new((4, 3).into(), Size::new(2, 2)),
      &[34, 35, 44, 45],
    );
    assert_eq!(&tex[3][4..], &[34, 35, 0, 0]);
    assert_eq!(&tex[4][4..], &[44, 45, 0, 0]);
    assert!(tex.is_updated());

    tex.data_synced();
    assert!(!tex.is_updated());
  }

  #[test]
  fn grow() {
    let mut tex = MemTexture::<1>::new(Size::new(2, 1), Size::new(512, 512));
    tex.write_rect(&Rect::new((0, 0).into(), (1, 1).into()), &[1u8]);

    assert_eq!(tex.as_bytes().len(), 2);
    assert_eq!(tex.size().to_array(), [2, 1]);

    tex.expand_size();
    assert_eq!(tex.as_bytes(), vec![1, 0, 0, 0, 0, 0, 0, 0].as_slice());
    assert!(tex.is_updated());

    tex.expand_size();
    let mut data = vec![0; 32];
    data[0] = 1;

    assert_eq!(tex.as_bytes(), data.as_slice());

    tex.data_synced();
    assert!(!tex.is_updated());
  }
}
