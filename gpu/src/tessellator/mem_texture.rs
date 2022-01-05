use painter::{DeviceRect, DeviceSize};

pub struct MemTexture<const N: usize> {
  max_size: DeviceSize,
  size: DeviceSize,
  array: Box<[u8]>,
  updated: bool,
}

impl<const N: usize> MemTexture<N> {
  pub fn new(size: DeviceSize, max_size: DeviceSize) -> Self {
    Self {
      size,
      max_size,
      array: Self::alloc_mem(size),
      updated: false,
    }
  }

  #[inline]
  pub fn is_updated(&self) -> bool { self.updated }

  #[inline]
  pub fn size(&self) -> DeviceSize { self.size }

  /// Expand the texture.
  /// Return true if expand successful, false if this texture reach the limit.
  /// the old data will all drop.
  pub fn expand_size(&mut self) -> Option<Box<[u8]>> {
    let mut size = self.size;
    self.max_size.greater_than(size).any().then(|| {
      if size.height * 2 < self.max_size.height {
        size.height *= 2;
      }
      if size.width * 2 < self.max_size.width {
        size.width *= 2;
      }

      self.updated = true;
      self.size = size;
      std::mem::replace(&mut self.array, Self::alloc_mem(size))
    })
  }

  #[inline]
  pub fn as_bytes(&self) -> &[u8] {
    unsafe {
      std::slice::from_raw_parts(&self.array as *const _ as *const u8, self.array.len() * N)
    }
  }

  /// Use `data` to fill the `rect` sub range to this 2d array, `rect` should
  /// not over this 2d array's boundary.
  pub fn write_rect(&mut self, rect: &DeviceRect, data: &[u8]) {
    debug_assert_eq!(rect.area() as usize, data.len());
    debug_assert!(DeviceRect::from_size(self.size).contains_rect(rect));
    let rect = rect.to_usize();
    let row_bytes = rect.width() * N;
    rect.y_range().enumerate().for_each(|(idx, y)| {
      let offset = idx * row_bytes;
      self[y][rect.x_range()].copy_from_slice(&data[offset..offset + row_bytes]);
    });

    self.updated = true;
  }

  /// Call this method after synced this texture data to a real texture of
  /// render engine. Tell this mem texture know, data has sync to render engine.
  pub fn data_synced(&mut self) { self.updated = false; }

  // The max size the texture can grow
  pub fn max_size(&self) -> DeviceSize { self.max_size }

  pub fn write_png_to(&self, name: &str, color: png::ColorType) {
    let pkg_root = env!("CARGO_MANIFEST_DIR");
    let atlas_capture = format!("{}/.log/{}", pkg_root, name);

    let DeviceSize { width, height, .. } = self.size;

    let mut png_encoder = png::Encoder::new(
      std::fs::File::create(&atlas_capture).unwrap(),
      width,
      height,
    );
    png_encoder.set_depth(png::BitDepth::Eight);
    png_encoder.set_color(color);
    png_encoder
      .write_header()
      .unwrap()
      .write_image_data(self.as_bytes())
      .unwrap();
  }

  fn alloc_mem(size: DeviceSize) -> Box<[u8]> {
    let bytes = size.area() as usize * N;
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
  use painter::DevicePoint;

  use super::*;
  #[test]
  fn update_texture() {
    let mut tex = MemTexture::<1>::new(DeviceSize::new(8, 8), DeviceSize::new(512, 512));

    tex.write_rect(
      &DeviceRect::new(DevicePoint::new(0, 0), DeviceSize::new(2, 1)),
      &[0, 1],
    );
    assert_eq!(&tex[0][0..4], &[00, 1, 0, 0]);

    tex.write_rect(
      &DeviceRect::new(DevicePoint::new(3, 7), DeviceSize::new(2, 1)),
      &[73, 74],
    );
    assert_eq!(tex[7][3], 73);

    tex.write_rect(
      &DeviceRect::new(DevicePoint::new(4, 3), DeviceSize::new(2, 2)),
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
    let mut tex = MemTexture::<1>::new(DeviceSize::new(2, 1), DeviceSize::new(512, 512));
    tex.write_rect(&DeviceRect::new((0, 0).into(), (1, 1).into()), &[1u8]);
    tex.expand_size();

    assert_eq!(tex.as_bytes().len(), 2);
    assert_eq!(tex.size().to_array(), [2, 1]);

    assert!(tex.is_updated());

    tex.expand_size();
    assert_eq!(tex.as_bytes(), vec![0; 32].as_slice());

    tex.data_synced();
    assert!(!tex.is_updated());
  }
}
