use crate::GPUBackendImpl;
use guillotiere::{AllocId, Allocation, AtlasAllocator};
use ribir_geom::{DevicePoint, DeviceRect, DeviceSize};
use ribir_painter::{image::ColorFormat, AntiAliasing};

use super::Texture;

pub const ATLAS_MAX_ITEM: DeviceSize = DeviceSize::new(512, 512);
pub const ATLAS_MIN_SIZE: DeviceSize = DeviceSize::new(1024, 1024);
pub const ATLAS_MAX_SIZE: DeviceSize = DeviceSize::new(4096, 4096);

pub(crate) struct Atlas<T: Texture> {
  pub atlas_allocator: AtlasAllocator,
  pub texture: T,
}

impl<T: Texture> Atlas<T> {
  pub fn new(format: ColorFormat, anti_aliasing: AntiAliasing, gpu_impl: &mut T::Host) -> Self
  where
    T::Host: GPUBackendImpl<Texture = T>,
  {
    let texture = gpu_impl.new_texture(ATLAS_MIN_SIZE, anti_aliasing, format);
    Self {
      texture,
      atlas_allocator: AtlasAllocator::new(ATLAS_MIN_SIZE.cast_unit()),
    }
  }

  /// Only allocate a rect in the atlas the caller should draw stull in the
  /// rect.
  pub fn allocate(&mut self, size: DeviceSize, gpu_impl: &mut T::Host) -> Option<Allocation>
  where
    T::Host: GPUBackendImpl<Texture = T>,
  {
    let alloc_size = size.to_i32().cast_unit();
    let mut alloc = self.atlas_allocator.allocate(alloc_size);

    if alloc.is_none() {
      let expand_size = (self.size() * 2).max(self.size()).min(ATLAS_MAX_SIZE);
      if expand_size != self.texture.size() {
        self.atlas_allocator.grow(expand_size.cast_unit());
        let mut new_tex = gpu_impl.new_texture(
          expand_size,
          self.texture.anti_aliasing(),
          self.texture.color_format(),
        );
        gpu_impl.copy_texture_from_texture(
          &mut new_tex,
          DevicePoint::zero(),
          &self.texture,
          &DeviceRect::from_size(self.size()),
        );

        self.texture = new_tex;
        alloc = self.atlas_allocator.allocate(alloc_size);
      }
    }

    alloc
  }

  pub fn deallocate(&mut self, id: AllocId) { self.atlas_allocator.deallocate(id); }

  pub fn size(&self) -> DeviceSize { self.texture.size() }

  pub fn clear(&mut self) { self.atlas_allocator.clear(); }
}

#[cfg(feature = "wgpu")]
#[cfg(test)]
mod tests {
  use super::*;
  use crate::{WgpuImpl, WgpuTexture};
  use futures::executor::block_on;

  #[test]
  fn atlas_grow_to_alloc() {
    let mut gpu_impl = block_on(WgpuImpl::headless());
    let mut atlas =
      Atlas::<WgpuTexture>::new(ColorFormat::Alpha8, AntiAliasing::None, &mut gpu_impl);
    let size = DeviceSize::new(ATLAS_MIN_SIZE.width + 1, 16);
    assert!(atlas.allocate(size, &mut gpu_impl).is_some());
  }
}
