use std::hash::Hash;

use guillotiere::{Allocation, AtlasAllocator};
use ribir_algo::FrameCache;
use ribir_painter::{DeviceRect, DeviceSize, TextureX};

use super::utils::allocation_to_rect;

pub struct Atlas<K, T: TextureX> {
  texture: T,
  atlas_allocator: AtlasAllocator,
  allocated_map: FrameCache<K, Allocation>,
  alloc_miss: bool,
}

impl<K: PartialEq + Eq + Hash, T: TextureX> Atlas<K, T> {
  pub fn new(texture: T) -> Self {
    let size = texture.size();
    Atlas {
      texture,
      allocated_map: <_>::default(),
      atlas_allocator: AtlasAllocator::new(size.to_i32().cast_unit()),
      alloc_miss: false,
    }
  }

  /// Get the rect of `key` store in the atlas, otherwise allocate area and call
  /// the callback to get data to write to texture.
  pub fn store(
    &mut self,
    key: K,
    size: DeviceSize,
    get_data: impl FnOnce(&K) -> Option<&[u8]>,
  ) -> Option<DeviceRect> {
    if let Some(alloc) = self.allocated_map.get(&key) {
      return Some(allocation_to_rect(alloc));
    } else {
      let alloc = self.atlas_allocator.allocate(size.to_i32().cast_unit());
      if let Some(alloc) = alloc {
        let rect = allocation_to_rect(&alloc);
        if let Some(data) = get_data(&key) {
          self.texture.write_data_to(rect, data);
        }
        self.allocated_map.insert(key, alloc);
        Some(rect)
      } else {
        self.alloc_miss = true;
        None
      }
    }
  }

  /// deallocate all last recently not used allocation.
  pub fn end_frame(&mut self) {
    let removed = self.allocated_map.frame_end_with(
      "Atlas",
      Some(|hit: bool, alloc: &mut Allocation| {
        if !hit {
          self.atlas_allocator.deallocate(alloc.id)
        };
      }),
    );
    if self.alloc_miss {
      todo!("rearrange")
    }

    self.alloc_miss = false;
  }
}
