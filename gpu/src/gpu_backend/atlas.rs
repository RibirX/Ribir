use crate::GPUBackendImpl;
use guillotiere::{AllocId, Allocation, AtlasAllocator};
use ribir_geom::{DevicePoint, DeviceRect, DeviceSize};
use ribir_painter::{image::ColorFormat, AntiAliasing};

use super::Texture;

pub const ATLAS_MAX_ITEM: DeviceSize = DeviceSize::new(512, 512);
pub const ATLAS_MIN_SIZE: DeviceSize = DeviceSize::new(1024, 1024);
pub const ATLAS_MAX_SIZE: DeviceSize = DeviceSize::new(4096, 4096);

pub(crate) struct Atlas<T: Texture> {
  pub texture: T,
  pub atlas_allocator: AtlasAllocator,
  need_rearrange: bool,
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
      need_rearrange: false,
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
      let expand_size = (size * 2).max(self.size()).min(ATLAS_MAX_SIZE);
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
    if alloc.is_none() && self.used_rate() < 0.7 {
      self.need_rearrange = true;
    }

    alloc
  }

  pub fn deallocate(&mut self, id: AllocId) { self.atlas_allocator.deallocate(id); }

  pub fn used_rate(&self) -> f32 {
    let mut area = 0;
    self
      .atlas_allocator
      .for_each_allocated_rectangle(|_, rect| area += rect.area());
    area as f32 / self.texture.size().area() as f32
  }

  pub fn size(&self) -> DeviceSize { self.texture.size() }

  pub fn hint_clear(&self) -> bool { self.need_rearrange }

  pub fn clear(&mut self) {
    self.need_rearrange = false;
    self.atlas_allocator.clear();
  }
}
