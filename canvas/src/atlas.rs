use super::{mem_texture::MemTexture, surface::Texture, Color, DevicePoint, DeviceSize};
use guillotiere::*;

const PALETTE_SIZE: u32 = DEFAULT_OPTIONS.small_size_threshold as u32;

pub struct TextureAtlas {
  texture: MemTexture<u32>,
  atlas_allocator: AtlasAllocator,
  indexed_colors: std::collections::HashMap<u32, DevicePoint>,
  palette_stored: usize,
  palette_alloc: Allocation,
}

#[derive(Debug)]
pub enum AtlasStoreErr {
  /// atlas is too full to store the texture, buf the texture is good for store
  /// in the atlas if it's not store too many others.
  SpaceNotEnough,
  /// The texture you want to store in the atlas is too large, you should not
  /// try to store it again.
  OverTheMaxLimit,
}

impl TextureAtlas {
  pub fn new() -> Self {
    const INIT: u32 = Texture::INIT_DIMENSION;
    let size = DeviceSize::new(INIT, INIT);
    let mut atlas_allocator = AtlasAllocator::new(size.cast_unit().to_i32());

    let palette_alloc = atlas_allocator
      .allocate(Size::new(PALETTE_SIZE as i32, PALETTE_SIZE as i32))
      .unwrap();

    TextureAtlas {
      texture: MemTexture::new(DeviceSize::new(INIT, INIT)),
      indexed_colors: <_>::default(),
      palette_stored: 0,
      atlas_allocator,
      palette_alloc,
    }
  }

  /// Store the `color` in, return the position in the texture of the color was.
  pub fn store_color(&mut self, color: Color) -> Result<DevicePoint, AtlasStoreErr> {
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
      } else {
        let mut size = *self.texture.size();
        if size.height * 2 <= Texture::MAX_DIMENSION {
          size.height *= 2;
          self.texture.grow_size(size, true);
        } else if size.width < Texture::MAX_DIMENSION {
          size.width *= 2;
          self.texture.grow_size(size, true);
        } else {
          break false;
        }
      }
    };

    if allocated_new_palette {
      Ok(self.add_color(color))
    } else {
      Err(AtlasStoreErr::SpaceNotEnough)
    }
  }

  /// Return the reference of the soft texture of the atlas, copy it to the
  /// render engine texture to use it.
  #[inline]
  pub fn texture(&self) -> &MemTexture<u32> { &self.texture }

  /// Flush all data to the texture and ready to commit to gpu.
  /// Call this function before commit drawing to gpu.
  pub fn flush_cache(
    &mut self,
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    texture: &wgpu::Texture,
  ) {
    if self.texture.is_updated() {
      let DeviceSize { width, height, .. } = *self.texture.size();
      let buffer =
        device.create_buffer_with_data(self.texture.as_bytes(), wgpu::BufferUsage::COPY_SRC);

      encoder.copy_buffer_to_texture(
        wgpu::BufferCopyView {
          buffer: &buffer,
          layout: wgpu::TextureDataLayout {
            offset: 0,
            bytes_per_row: width * std::mem::size_of::<u32>() as u32,
            rows_per_image: height,
          },
        },
        wgpu::TextureCopyView {
          texture,
          mip_level: 0,
          origin: wgpu::Origin3d::ZERO,
        },
        wgpu::Extent3d {
          width,
          height,
          depth: 1,
        },
      )
    }
  }

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
