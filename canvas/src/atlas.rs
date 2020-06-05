use super::{array_2d::Array2D, surface::Texture, Color, DevicePoint, DeviceSize};
use guillotiere::*;
mod palette;

pub struct TextureAtlas {
  texture_updated: bool,
  texture_resized: bool,
  texture: Array2D<u32>,
  atlas_allocator: AtlasAllocator,
  indexed_colors: std::collections::HashMap<u32, DevicePoint>,
  current_palette: palette::Palette,
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
      .allocate(Size::new(
        palette::PALETTE_SIZE as i32,
        palette::PALETTE_SIZE as i32,
      ))
      .unwrap();

    TextureAtlas {
      texture: Array2D::fill_from(INIT, INIT, 0),
      texture_resized: false,
      texture_updated: false,
      indexed_colors: <_>::default(),
      current_palette: <_>::default(),
      atlas_allocator,
      palette_alloc,
    }
  }

  /// Store the `color` in, return the position in the texture of the color was.
  pub fn store_color(&mut self, color: Color) -> Result<DevicePoint, AtlasStoreErr> {
    if let Some(pos) = self.indexed_colors.get(&color.as_u32()) {
      return Ok(*pos);
    }
    if !self.current_palette.is_fulled() {
      let pos = self.add_color(color);
      return Ok(pos);
    }

    let allocated_new_palette = loop {
      if !self.new_palette() {
        let mut size = self.texture.size();
        if size.height * 2 <= Texture::MAX_DIMENSION {
          size.height *= 2;
          self.grow_texture(size);
        } else if size.width < Texture::MAX_DIMENSION {
          size.width *= 2;
          self.grow_texture(size);
        } else {
          break false;
        }
      } else {
        break true;
      }
    };

    if allocated_new_palette {
      Ok(self.add_color(color))
    } else {
      Err(AtlasStoreErr::SpaceNotEnough)
    }
  }

  #[inline]
  pub fn is_texture_resized(&self) -> bool { self.texture_resized }

  #[inline]
  pub fn size(&self) -> DeviceSize { self.texture.size() }

  /// Flush all data to the texture and ready to commit to gpu.
  /// Call this function before commit drawing to gpu.
  pub fn flush_cache(
    &mut self,
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    texture: &wgpu::Texture,
  ) {
    if self.texture_updated {
      self.texture_updated = false;
      self.texture_resized = false;

      if self.current_palette.stored_color_size() > 0 {
        self.save_current_palette();
      }

      let DeviceSize { width, height, .. } = self.texture.size();
      let buffer =
        device.create_buffer_with_data(self.texture.raw_data(), wgpu::BufferUsage::COPY_SRC);

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
    self.current_palette.clear();
    self.indexed_colors.clear();
    self.texture.fill(0);
  }

  fn grow_texture(&mut self, size: DeviceSize) {
    self.texture_resized = true;
    self.texture_updated = true;
    self.atlas_allocator.grow(size.to_i32().to_untyped());
    let mut new_tex = Array2D::fill_from(size.height, size.width, 0);
    new_tex.copy_from_slice(0, 0, self.texture.columns(), self.texture.data());
    self.texture = new_tex;
  }

  fn add_color(&mut self, color: Color) -> DevicePoint {
    self.texture_updated = true;
    let key = color.as_u32();
    let offset = self.current_palette.add_color(color);
    let pos = self.palette_alloc.rectangle.min + offset;
    let pos = DevicePoint::new(pos.x as u32, pos.y as u32);
    self.indexed_colors.insert(key, pos);
    pos
  }

  fn new_palette(&mut self) -> bool {
    self
      .atlas_allocator
      .allocate(Size::new(
        palette::PALETTE_SIZE as i32,
        palette::PALETTE_SIZE as i32,
      ))
      .and_then(|alloc| {
        self.save_current_palette();
        self.current_palette = palette::Palette::default();
        self.palette_alloc = alloc;
        Some(true)
      })
      .is_some()
  }

  fn save_current_palette(&mut self) {
    let palette_pos = self.palette_alloc.rectangle.min.to_u32();
    self.texture.copy_from_slice(
      palette_pos.y,
      palette_pos.x,
      palette::PALETTE_SIZE,
      self.current_palette.data(),
    );
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
