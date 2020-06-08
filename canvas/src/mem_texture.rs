use super::{DevicePoint, DeviceRect, DeviceSize};

pub struct MemTexture<T: Copy + Default> {
  max_size: DeviceSize,
  array: Vec<T>,
  size: DeviceSize,
  updated: bool,
  resized: bool,
}

impl<T: Copy + Default> MemTexture<T> {
  pub fn new(size: DeviceSize, max_size: DeviceSize) -> Self {
    Self {
      size,
      max_size,
      array: vec![T::default(); size.area() as usize],
      updated: false,
      resized: true,
    }
  }

  /// Clear all data store in texture
  #[inline]
  pub fn clear(&mut self) {
    self.updated = true;
    self.array.fill(T::default());
  }

  #[inline]
  pub fn is_updated(&self) -> bool { self.updated }

  #[inline]
  pub fn is_resized(&self) -> bool { self.resized }

  #[inline]
  pub fn size(&self) -> DeviceSize { self.size }

  /// Expand the texture.
  /// Return true if expand successful, false if this texture reach the limit.
  /// Use `keep_old_data` to detect if old data should keep or throw away.
  pub fn expand_size(&mut self, keep_old_data: bool) -> bool {
    let mut size = self.size;
    if self.max_size.greater_than(size).any() {
      if size.height < self.max_size.height {
        size.height = (size.height * 2).min(self.max_size.height);
      }
      if size.width * 2 < self.max_size.width {
        size.width = (size.width * 2).min(self.max_size.width);
      }

      self.resized = true;
      self.updated = true;
      let array = std::mem::replace(&mut self.array, vec![T::default(); size.area() as usize]);
      if keep_old_data {
        let old_size = self.size;
        self.size = size;
        self.update_texture(&DeviceRect::from(old_size), array.as_slice());
      } else {
        self.size = size;
      }
      true
    } else {
      false
    }
  }

  #[inline]
  pub fn as_bytes(&self) -> &[u8] {
    let bytes = std::mem::size_of::<T>();
    unsafe {
      std::slice::from_raw_parts(
        self.array.as_slice() as *const _ as *const u8,
        self.array.len() * bytes,
      )
    }
  }

  /// Set `v` to the pixel at `pos`.
  pub fn set(&mut self, pos: &DevicePoint, v: T) {
    self[pos.y][pos.x as usize] = v;
    self.updated = true;
  }

  /// Use `data` to fill the `rect` sub range to this 2d array, `rect` should
  /// not over this 2d array's boundary.
  pub fn update_texture(&mut self, rect: &DeviceRect, data: &[T])
  where
    T: Copy,
  {
    debug_assert_eq!(rect.area() as usize, data.len());
    debug_assert!(DeviceRect::from_size(self.size).contains_rect(&rect));
    let rect = rect.to_usize();

    rect.y_range().into_iter().enumerate().for_each(|(idx, y)| {
      let offset = idx * rect.width();
      self[y as u32][rect.x_range()].copy_from_slice(&data[offset..offset + rect.width()]);
    });

    self.updated = true;
  }

  /// Call this method after synced this texture data to a real texture of
  /// render engine. Tell this mem texture know, data has sync to render engine.
  pub fn data_synced(&mut self) {
    self.resized = false;
    self.updated = false;
  }

  pub fn log(&self, name: &str) {
    let pkg_root = env!("CARGO_MANIFEST_DIR");
    let atlas_capture = format!("{}/.log/{}", pkg_root, name);

    let DeviceSize { width, height, .. } = self.size;

    let mut png_encoder = png::Encoder::new(
      std::fs::File::create(&atlas_capture).unwrap(),
      width,
      height,
    );
    png_encoder.set_depth(png::BitDepth::Eight);
    png_encoder.set_color(png::ColorType::Grayscale);
    png_encoder
      .write_header()
      .unwrap()
      .write_image_data(self.as_bytes())
      .unwrap();

    log::info!("Write a image of mem texture at: {}", &atlas_capture);
  }
}

impl<T: Copy + Default> std::ops::Index<u32> for MemTexture<T> {
  type Output = [T];
  fn index(&self, index: u32) -> &Self::Output {
    let width = self.size.width;
    let array_offset = (index * width) as usize;
    &self.array[array_offset..array_offset + width as usize]
  }
}

impl<T: Copy + Default> std::ops::IndexMut<u32> for MemTexture<T> {
  fn index_mut(&mut self, index: u32) -> &mut Self::Output {
    let width = self.size.width;
    let array_offset = (index * width) as usize;
    &mut self.array[array_offset..array_offset + width as usize]
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  #[test]
  fn update_texture() {
    let mut tex = MemTexture::new(DeviceSize::new(8, 8), DeviceSize::new(512, 512));

    tex.update_texture(&euclid::rect(0, 0, 2, 1), &[00, 01]);
    assert_eq!(&tex[0][0..4], &[00, 01, 0, 0]);

    tex.update_texture(&euclid::rect(3, 7, 2, 1), &[73, 74]);
    assert_eq!(tex[7][3], 73);

    tex.update_texture(&euclid::rect(4, 3, 2, 2), &[34, 35, 44, 45]);
    assert_eq!(&tex[3][4..], &[34, 35, 0, 0]);
    assert_eq!(&tex[4][4..], &[44, 45, 0, 0]);
    assert_eq!(tex.is_updated(), true);

    tex.data_synced();
    assert_eq!(tex.is_updated(), false);
  }

  #[test]
  fn grow() {
    let mut tex = MemTexture::new(DeviceSize::new(2, 1), DeviceSize::new(512, 512));
    tex.set(&DevicePoint::new(1, 0), 1u8);
    tex.expand_size(true);

    assert_eq!(tex.as_bytes().len(), 8);
    assert_eq!(tex.size().to_array(), [4, 2]);

    // old data should keep in the same place.
    assert_eq!(tex.as_bytes(), &[0, 1, 0, 0, 0, 0, 0, 0]);
    assert_eq!(tex.is_updated(), true);
    assert_eq!(tex.is_resized(), true);

    // grow size and throw old data away.
    tex.expand_size(false);
    assert_eq!(tex.as_bytes(), vec![0; 32].as_slice());

    tex.data_synced();
    assert_eq!(tex.is_resized(), false);
  }
}
