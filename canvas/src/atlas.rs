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
    if let Some(pos) = self.indexed_colors.get(&color.as_u32()) {
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
      } else if !self.texture.expand_size(true) {
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
  pub fn texture(&mut self) -> &mut MemTexture<u32> { &mut self.texture }

  /// Clear the atlas.
  pub fn clear(&mut self) {
    self.atlas_allocator.clear();
    self.indexed_colors.clear();
    self.texture.clear();
  }

  fn add_color(&mut self, color: Color) -> DevicePoint {
    let index = self.palette_stored as u32;
    let offset = euclid::Vector2D::new(index % PALETTE_SIZE, index / PALETTE_SIZE);
    let pos = self.palette_alloc.rectangle.min.to_u32() + offset;
    let pos = DevicePoint::from_untyped(pos);
    let u_color = color.as_u32();
    self.indexed_colors.insert(u_color, pos);
    self.texture.set(&pos, u_color);
    self.palette_stored += 1;
    pos
  }

  #[inline]
  fn is_palette_fulled(&self) -> bool {
    const PALETTE_SLOTS: usize = (PALETTE_SIZE * PALETTE_SIZE) as usize;
    self.palette_stored < PALETTE_SLOTS
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  #[test]
  fn store_color_smoke() {
    let mut atlas = TextureAtlas::new();
    atlas.store_color(Color::RED).unwrap();
    atlas.store_color(Color::RED).unwrap();

    assert_eq!(
      atlas.current_palette.data()[0],
      palette::color_as_bgra(Color::RED)
    );
    assert_eq!(atlas.current_palette.data()[1], 0);
    assert_eq!(atlas.current_palette.is_fulled(), false);
    assert_eq!(atlas.texture[0][0], 0);

    atlas.new_palette();
    assert_eq!(atlas.current_palette.data()[0], 0);
    assert_eq!(atlas.texture[0][0], palette::color_as_bgra(Color::RED));
  }

  #[test]
  fn grow_texture() {
    let mut atlas = TextureAtlas::new();
    atlas.store_color(Color::RED).unwrap();

    while atlas.new_palette() {}
    let mut i = 0;
    while !atlas.current_palette.is_fulled() {
      i += 1;
      atlas.store_color(Color::from_u32(i)).unwrap();
    }

    atlas.store_color(Color::from_u32(0)).unwrap();

    assert!(atlas.is_texture_resized());
    // stored color should be kept after texture grow.
    assert_eq!(atlas.texture[0][0], palette::color_as_bgra(Color::RED));
  }
}
