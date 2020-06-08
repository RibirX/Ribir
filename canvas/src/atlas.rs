use super::{error::CanvasError, mem_texture::MemTexture, Color, DevicePoint, DeviceSize};
use guillotiere::*;

const PALETTE_SIZE: u32 = DEFAULT_OPTIONS.small_size_threshold as u32;

pub struct TextureAtlas {
  texture: MemTexture<u32>,
  atlas_allocator: AtlasAllocator,
  indexed_colors: std::collections::HashMap<u32, DevicePoint>,
  palette_stored: usize,
  palette_alloc: Allocation,
}

impl TextureAtlas {
  pub fn new(init_size: DeviceSize, max_size: DeviceSize) -> Self {
    let mut atlas_allocator = AtlasAllocator::new(init_size.to_untyped().to_i32());

    let palette_alloc = atlas_allocator
      .allocate(Size::new(PALETTE_SIZE as i32, PALETTE_SIZE as i32))
      .unwrap();

    TextureAtlas {
      texture: MemTexture::new(init_size, max_size),
      indexed_colors: <_>::default(),
      palette_stored: 0,
      atlas_allocator,
      palette_alloc,
    }
  }

  /// Store the `color` in, return the position in the texture of the color was.
  pub fn store_color(&mut self, color: Color) -> Result<DevicePoint, CanvasError> {
    let color = color.as_u32();
    if let Some(pos) = self.indexed_colors.get(&color) {
      return Ok(*pos);
    }
    if !self.is_palette_fulled() {
      let pos = self.add_color(color);
      return Ok(pos);
    }

    let allocated_new_palette = loop {
      if let Some(alloc) = self
        .atlas_allocator
        .allocate(Size::new(PALETTE_SIZE as i32, PALETTE_SIZE as i32))
      {
        self.palette_alloc = alloc;
        self.palette_stored = 0;
        break true;
      } else if self.texture.expand_size(true) {
        self
          .atlas_allocator
          .grow(self.texture().size().to_i32().to_untyped());
      } else {
        break false;
      }
    };

    if allocated_new_palette {
      Ok(self.add_color(color))
    } else {
      Err(CanvasError::TextureSpaceNotEnough)
    }
  }

  /// Return the reference of the soft texture of the atlas, copy it to the
  /// render engine texture to use it.
  #[inline]
  pub fn texture(&self) -> &MemTexture<u32> { &self.texture }

  #[inline]
  pub fn texture_mut(&mut self) -> &mut MemTexture<u32> { &mut self.texture }

  /// Clear the atlas.
  pub fn clear(&mut self) {
    self.atlas_allocator.clear();
    self.indexed_colors.clear();
    self.texture.clear();
  }

  fn add_color(&mut self, color: u32) -> DevicePoint {
    let index = self.palette_stored as u32;
    let offset = euclid::Vector2D::new(index % PALETTE_SIZE, index / PALETTE_SIZE);
    let pos = self.palette_alloc.rectangle.min.to_u32() + offset;
    let pos = DevicePoint::from_untyped(pos);

    self.indexed_colors.insert(color, pos);
    self.texture.set(&pos, color);
    self.palette_stored += 1;
    pos
  }

  #[inline]
  fn is_palette_fulled(&self) -> bool {
    const PALETTE_SLOTS: usize = (PALETTE_SIZE * PALETTE_SIZE) as usize;
    self.palette_stored >= PALETTE_SLOTS
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  const INIT_SIZE: DeviceSize = DeviceSize::new(32, 32);
  const MAX_SIZE: DeviceSize = DeviceSize::new(1024, 1024);

  #[test]
  fn store_color_smoke() {
    let mut atlas = TextureAtlas::new(INIT_SIZE, MAX_SIZE);
    let r1 = atlas.store_color(Color::RED).unwrap();
    let r2 = atlas.store_color(Color::RED).unwrap();
    assert_eq!(r1, r2);
    assert!(atlas.texture().is_updated());

    (0..512).for_each(|i| {
      atlas.store_color(Color::from_u32(i)).unwrap();
    });
    assert_eq!(atlas.texture()[0][0], Color::RED.as_u32());
    assert!(atlas.texture().is_updated());
    assert!(atlas.texture().is_resized());
  }
}
