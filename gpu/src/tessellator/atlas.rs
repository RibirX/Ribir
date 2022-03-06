use crate::{error::Error, ColorFormat, Texture};

use super::mem_texture::{MemTexture, Rect, Size};

use algo::FrameCache;
use guillotiere::{Allocation, AtlasAllocator, ChangeList};
use painter::{PixelImage, ShallowImage};
use std::collections::HashMap;

pub(crate) struct TextureAtlas {
  texture: MemTexture<{ TextureAtlas::UNIT }>,
  atlas_allocator: AtlasAllocator,
  allocated_map: FrameCache<ShallowImage, Allocation>,
}

impl TextureAtlas {
  const UNIT: usize = 4;
  pub fn new(init_size: Size, max_size: Size) -> Self {
    let atlas_allocator = AtlasAllocator::new(init_size.to_untyped().to_i32());

    TextureAtlas {
      texture: MemTexture::new(init_size, max_size),
      allocated_map: <_>::default(),
      atlas_allocator,
    }
  }

  /// Store a image in the atlas, and return the allocation of it.
  pub fn store_image(&mut self, image: &ShallowImage) -> Result<Rect, Error> {
    if self.is_large_img_to_me(image.as_ref()) {
      return Err(Error::LargeImageAvoid);
    }

    fn alloc_rect(alloc: &Allocation) -> Rect {
      let rect = alloc.rectangle.to_rect();
      guillotiere::euclid::rect(
        rect.min_x() as u16,
        rect.min_y() as u16,
        rect.width() as u16,
        rect.height() as u16,
      )
    }
    let mut alloc = None;
    if !self.allocated_map.contains_key(&image) {
      let (w, h) = image.size();
      let a = self.allocate(w, h)?;

      self
        .texture
        .write_rect(&alloc_rect(&a), &image.pixel_bytes());
      alloc = Some(a);
    }

    let alloc = self
      .allocated_map
      .get_or_insert_with(&image, || alloc.unwrap());
    Ok(alloc_rect(&alloc))
  }

  pub fn is_large_img_to_me(&self, img: &PixelImage) -> bool {
    let max = self.texture.max_size();
    let (img_w, img_h) = img.size();
    max.width < img_w * 2 || max.height < img_h * 2
  }

  pub fn as_render_texture(&self, id: usize) -> Texture {
    let tex = &self.texture;
    let data = tex.is_updated().then(|| tex.as_bytes());
    Texture {
      id,
      size: tex.size().into(),
      data,
      format: ColorFormat::Rgba8,
    }
  }

  /// deallocate all last recently not used allocation.
  pub fn end_frame(&mut self) {
    let removed = self.allocated_map.frame_end_with(
      "Texture Atlas",
      Some(|hit: bool, alloc: &mut Allocation| {
        if !hit {
          self.atlas_allocator.deallocate(alloc.id)
        };
      }),
    );
    if removed > 0 {
      self.rearrange()
    }
  }

  /// Return the reference of the soft texture of the atlas, copy it to the
  /// render engine texture to use it.
  #[inline]
  pub fn texture(&self) -> &MemTexture<4> { &self.texture }

  /// A gpu command and data submitted.
  pub fn gpu_synced(&mut self) { self.texture.data_synced(); }

  pub fn clear(&mut self) {
    self.atlas_allocator.clear();
    self.allocated_map.clear();
  }

  fn allocate(&mut self, width: u16, height: u16) -> Result<Allocation, Error> {
    loop {
      if let Some(alloc) = self
        .atlas_allocator
        .allocate((width as i32, height as i32).into())
      {
        break Ok(alloc);
      }
      if !self.grow() {
        break Err(Error::TextureSpaceLimit);
      }
    }
  }

  fn grow(&mut self) -> bool {
    let expended = self.texture.expand_size();
    if expended {
      let new_size = self.texture().size().to_i32().to_untyped();
      self.atlas_allocator.grow(new_size);
    }
    expended
  }

  fn rearrange(&mut self) {
    let ChangeList { changes, failures } = self.atlas_allocator.rearrange();
    assert!(
      failures.is_empty(),
      "grow atlas and rearrange should not failed."
    );
    if changes.is_empty() {
      return;
    }

    let old = self.texture.as_bytes().to_owned();
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
  }
}

#[cfg(test)]
pub mod tests {
  use painter::Color;
  use std::borrow::Cow;

  use super::*;

  const MAX_SIZE: DeviceSize = DeviceSize::new(1024, 1024);

  pub fn color_image(color: Color, size: DeviceSize) -> ShallowImage {
    let data = std::iter::repeat(color.into_raw())
      .take(size.area() as usize)
      .flatten()
      .collect::<Vec<_>>();

    let img = PixelImage::new(Cow::Owned(data), size, ColorFormat::Rgba8);
    ShallowImage::new(img)
  }

  #[test]
  fn grow_alloc_keep() {
    let mut atlas = TextureAtlas::new(DeviceSize::new(64, 64), MAX_SIZE);
    let red_img = color_image(Color::RED, DeviceSize::new(32, 32));
    let red_alloc = *atlas.store_image(&red_img).unwrap();

    assert_eq!(red_alloc.rectangle.min.to_array(), [0, 0]);

    // same image should have same position in atlas
    assert_eq!(&red_alloc, atlas.store_image(&red_img).unwrap());
    color_img_check(&atlas, &red_alloc, Color::RED);

    let yellow_img = color_image(Color::YELLOW, DeviceSize::new(64, 64));
    let yellow_alloc = *atlas.store_image(&yellow_img).unwrap();

    // the color should keep after atlas rearrange
    color_img_check(&atlas, &red_alloc, Color::RED);
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
