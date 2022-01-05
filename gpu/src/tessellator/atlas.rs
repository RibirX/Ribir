use crate::{error::Error, ColorFormat, Texture};

use super::mem_texture::MemTexture;

use algo::FrameCache;
use guillotiere::{Allocation, AtlasAllocator, ChangeList};
use painter::{DeviceSize, Image, ShallowImage};
use std::collections::HashMap;

pub(crate) struct TextureAtlas {
  texture: MemTexture<{ TextureAtlas::UNIT }>,
  atlas_allocator: AtlasAllocator,
  allocated_map: FrameCache<ShallowImage, Allocation>,
}

impl TextureAtlas {
  const UNIT: usize = 4;
  pub fn new(init_size: DeviceSize, max_size: DeviceSize) -> Self {
    let atlas_allocator = AtlasAllocator::new(init_size.to_untyped().to_i32());

    TextureAtlas {
      texture: MemTexture::new(init_size, max_size),
      allocated_map: <_>::default(),
      atlas_allocator,
    }
  }

  /// Store a image in the atlas, and return the allocation of it. And remember
  /// if `auto_grow` is true, the atlas will auto grow and _rearrange if
  /// necessary, in this case, all `Allocation` returned before invalid.
  pub fn store_image(
    &mut self,
    image: ShallowImage,
    auto_grow: bool,
  ) -> Result<&Allocation, Error> {
    if self.is_large_img_to_me(&**image) {
      return Err(Error::LargeImageAvoid);
    }

    let mut alloc = None;
    if !self.allocated_map.contains_key(&image) {
      let a = self.allocate(image.size(), auto_grow)?;
      self.texture.write_rect(
        &a.rectangle.to_u32().to_rect().cast_unit(),
        &image.pixel_bytes(),
      );
      alloc = Some(a);
    }

    let alloc = self
      .allocated_map
      .get_or_insert_with_key(image, |_| alloc.unwrap());
    Ok(alloc)
  }

  pub fn is_large_img_to_me(&self, img: &dyn Image) -> bool {
    let max = self.texture.max_size();
    return max.lower_than(img.size() * 2).any();
  }

  pub fn as_render_texture(&self, id: usize) -> Texture {
    let tex = &self.texture;
    let data = tex.is_updated().then(|| tex.as_bytes());
    Texture {
      id,
      size: tex.size(),
      data,
      format: ColorFormat::Rgba8,
    }
  }

  /// deallocate all last recently not used allocation.
  pub fn end_frame(&mut self) {
    self.allocated_map.frame_end_with(
      "Texture Atlas",
      Some(|hit: bool, alloc: &mut Allocation| {
        if !hit {
          self.atlas_allocator.deallocate(alloc.id)
        };
      }),
    );
  }

  /// Return the reference of the soft texture of the atlas, copy it to the
  /// render engine texture to use it.
  #[inline]
  pub fn texture(&self) -> &MemTexture<4> { &self.texture }

  /// A gpu command and data submitted.
  pub fn gpu_synced(&mut self) { self.texture.data_synced(); }

  #[inline]
  pub fn log_png_to(&self, path: &str) { self.texture.write_png_to(path, png::ColorType::RGBA); }

  fn allocate(&mut self, size: DeviceSize, auto_grow: bool) -> Result<Allocation, Error> {
    loop {
      if let Some(alloc) = self.atlas_allocator.allocate(size.to_untyped().to_i32()) {
        return Ok(alloc);
      } else if auto_grow {
        if !self.grow() {
          return Err(Error::TextureSpaceToLimit);
        }
      } else {
        return Err(Error::TextureFull);
      }
    }
  }

  pub fn no_cache_hit_find(&self, img: &ShallowImage) -> Option<&Allocation> {
    self.allocated_map.no_hit_get(img)
  }

  fn grow(&mut self) -> bool {
    self.texture.expand_size().map_or(false, |old| {
      let new_size = self.texture().size().to_i32().to_untyped();
      let ChangeList { changes, failures } = self.atlas_allocator.resize_and_rearrange(new_size);
      assert!(
        failures.is_empty(),
        "grow atlas and rearrange should not failed."
      );
      changes.iter().for_each(|c| {
        let old_rect = c.old.rectangle.to_usize();
        let new_rect = c.new.rectangle.to_usize();
        let rect_row_bytes = old_rect.width() * TextureAtlas::UNIT;

        old_rect
          .y_range()
          .zip(new_rect.y_range())
          .for_each(|(old_y, new_y)| {
            let old_offset = old_rect.min.x * TextureAtlas::UNIT + old_y * rect_row_bytes;
            let new_offset = new_rect.min.x * TextureAtlas::UNIT + new_y * rect_row_bytes;
            self.texture[new_y][new_offset..new_offset + rect_row_bytes]
              .copy_from_slice(&old[old_offset..old_offset + rect_row_bytes]);
          });
      });

      // update the allocated map
      let id_map = changes
        .iter()
        .map(|c| (c.old.id, c.new))
        .collect::<HashMap<_, _>>();
      self.allocated_map.values_mut().for_each(|alloc| {
        if let Some(new_alloc) = id_map.get(&alloc.id) {
          *alloc = *new_alloc;
        }
      });
      true
    })
  }
}

#[cfg(test)]
pub mod tests {
  use std::rc::Rc;

  use painter::Color;

  use super::*;

  const MAX_SIZE: DeviceSize = DeviceSize::new(1024, 1024);

  #[derive(Clone, Debug)]
  pub struct PureColorImage {
    pub size: DeviceSize,
    pub color: Color,
  }

  impl Image for PureColorImage {
    fn pixel_bytes(&self) -> Box<[u8]> {
      let vec = vec![self.color.clone().into_raw(); self.size.area() as usize].into_boxed_slice();
      unsafe { std::mem::transmute(vec) }
    }

    fn size(&self) -> DeviceSize { self.size }

    fn color_format(&self) -> ColorFormat { ColorFormat::Rgba8 }
  }

  impl PureColorImage {
    pub fn new(color: Color, size: DeviceSize) -> Self { Self { size, color } }

    pub fn shallow_img(color: Color, size: DeviceSize) -> ShallowImage {
      ShallowImage::new(Rc::new(Box::new(Self::new(color, size))))
    }
  }

  #[test]
  fn grow_alloc_keep() {
    let mut atlas = TextureAtlas::new(DeviceSize::new(64, 64), MAX_SIZE);
    let red_img = PureColorImage::shallow_img(Color::RED, DeviceSize::new(32, 32));
    let red_alloc = *atlas.store_image(red_img.clone(), false).unwrap();

    assert_eq!(red_alloc.rectangle.min.to_array(), [0, 0]);

    // same image should have same position in atlas
    assert_eq!(
      &red_alloc,
      atlas.store_image(red_img.clone(), false).unwrap()
    );
    color_img_check(&atlas, &red_alloc, Color::RED);

    let yellow_img = PureColorImage::shallow_img(Color::YELLOW, DeviceSize::new(64, 64));
    let yellow_alloc = *atlas.store_image(yellow_img, true).unwrap();

    let red_alloc = atlas.no_cache_hit_find(&red_img).unwrap();
    // the color should keep after atlas rearrange
    color_img_check(&atlas, red_alloc, Color::RED);
    color_img_check(&atlas, &yellow_alloc, Color::YELLOW);
  }

  fn color_img_check(atlas: &TextureAtlas, alloc: &Allocation, color: Color) {
    const UNIT: usize = TextureAtlas::UNIT;
    let rect = alloc.rectangle.to_usize();
    let color_data = color.into_raw();
    rect.y_range().for_each(|y| {
      rect.x_range().for_each(|x| {
        assert_eq!(atlas.texture[y][UNIT * x..UNIT * (x + 1)], color_data);
      })
    })
  }
}
