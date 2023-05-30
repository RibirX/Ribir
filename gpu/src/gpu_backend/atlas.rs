use super::Texture;
use crate::GPUBackendImpl;
use guillotiere::{Allocation, AtlasAllocator};
use ribir_algo::FrameCache;
use ribir_geom::{DevicePoint, DeviceRect, DeviceSize};
use ribir_painter::{image::ColorFormat, AntiAliasing};
use slab::Slab;
use std::hash::Hash;

pub const ATLAS_MAX_ITEM: DeviceSize = DeviceSize::new(512, 512);
pub const ATLAS_MIN_SIZE: DeviceSize = DeviceSize::new(1024, 1024);
pub const ATLAS_MAX_SIZE: DeviceSize = DeviceSize::new(4096, 4096);

#[derive(Copy, Clone, Debug, PartialEq)]
enum AtlasDist {
  Atlas(Allocation),
  Extra(usize),
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct AtlasHandle<Attr> {
  pub attr: Attr,
  atlas_dist: AtlasDist,
}

pub(crate) struct Atlas<T: Texture, K, Attr> {
  atlas_allocator: AtlasAllocator,
  texture: T,
  label: &'static str,
  cache: FrameCache<K, AtlasHandle<Attr>>,
  extras: Slab<T>,
}

macro_rules! release_handle {
  ($this: ident, $handle: ident) => {
    match $handle.atlas_dist {
      AtlasDist::Atlas(alloc) => {
        $this.atlas_allocator.deallocate(alloc.id);
      }
      AtlasDist::Extra(id) => {
        $this.extras.remove(id);
      }
    }
  };
}

impl<K, Attr, T: Texture> Atlas<T, K, Attr>
where
  T::Host: GPUBackendImpl<Texture = T>,
  K: Hash + Eq,
{
  pub fn new(
    label: &'static str,
    format: ColorFormat,
    anti_aliasing: AntiAliasing,
    gpu_impl: &mut T::Host,
  ) -> Self {
    let texture = gpu_impl.new_texture(ATLAS_MIN_SIZE, anti_aliasing, format);
    Self {
      label,
      texture,
      atlas_allocator: AtlasAllocator::new(ATLAS_MIN_SIZE.cast_unit()),
      cache: FrameCache::new(),
      extras: Slab::default(),
    }
  }

  pub fn get(&mut self, key: &K) -> Option<&AtlasHandle<Attr>> { self.cache.get(key) }

  pub fn set_anti_aliasing(&mut self, anti_aliasing: AntiAliasing, host: &mut T::Host) {
    if self.texture.anti_aliasing() != anti_aliasing {
      self.texture.set_anti_aliasing(anti_aliasing, host);
      self.clear();
    }
  }

  /// Allocate a rect in the atlas the caller should draw stull in the
  /// rect. Check if a cache exist before allocate.
  pub fn allocate(
    &mut self,
    key: K,
    attr: Attr,
    size: DeviceSize,
    gpu_impl: &mut T::Host,
  ) -> AtlasHandle<Attr>
  where
    Attr: Copy,
  {
    let current_size = self.size();
    let alloc_size = size.to_i32().cast_unit();
    let mut alloc = self.atlas_allocator.allocate(alloc_size);

    if alloc.is_none() {
      let expand_size = (current_size * 2).max(current_size).min(ATLAS_MAX_SIZE);
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
          &DeviceRect::from_size(current_size),
        );

        self.texture = new_tex;
        alloc = self.atlas_allocator.allocate(alloc_size);
      }
    }

    let atlas_dist = if let Some(alloc) = alloc {
      AtlasDist::Atlas(alloc)
    } else {
      let texture = gpu_impl.new_texture(
        size,
        self.texture.anti_aliasing(),
        self.texture.color_format(),
      );
      let id = self.extras.insert(texture);
      AtlasDist::Extra(id)
    };

    let handle = AtlasHandle { attr, atlas_dist };

    if let Some(h) = self.cache.put(key, handle) {
      release_handle!(self, h);
    }

    handle
  }

  /// Get a mut reference of a texture that `id` point to. The `id` get from
  /// `AtlasHandle::tex_id`
  pub fn get_texture_mut(&mut self, id: usize) -> &mut T {
    if id == 0 {
      &mut self.texture
    } else {
      &mut self.extras[id - 1]
    }
  }

  /// Get a reference of a texture that `id` point to. The `id` get from
  /// `AtlasHandle::tex_id`
  pub fn get_texture(&self, id: usize) -> &T {
    if id == 0 {
      &self.texture
    } else {
      &self.extras[id - 1]
    }
  }

  pub fn size(&self) -> DeviceSize { self.texture.size() }

  pub fn clear(&mut self) {
    self.cache.clear();
    self.atlas_allocator.clear();
    self.extras.clear();
  }

  pub(crate) fn end_frame(&mut self) {
    self
      .cache
      .end_frame(self.label)
      .for_each(|h| release_handle!(self, h));
  }
}

impl<Attr> AtlasHandle<Attr> {
  pub fn tex_id(&self) -> usize {
    match &self.atlas_dist {
      AtlasDist::Atlas(_) => 0,
      AtlasDist::Extra(id) => *id + 1,
    }
  }

  pub(super) fn tex_rect<T, K>(&self, atlas: &Atlas<T, K, Attr>) -> DeviceRect
  where
    T: Texture,
  {
    match &self.atlas_dist {
      AtlasDist::Atlas(alloc) => alloc.rectangle.to_rect().cast_unit(),
      AtlasDist::Extra(id) => DeviceRect::from_size(atlas.extras[*id].size()),
    }
  }
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
      Atlas::<WgpuTexture, _, _>::new("_", ColorFormat::Alpha8, AntiAliasing::None, &mut gpu_impl);
    let size = DeviceSize::new(ATLAS_MIN_SIZE.width + 1, 16);
    let h = atlas.allocate(1, (), size, &mut gpu_impl);
    gpu_impl.end_frame();
    assert_eq!(h.tex_id(), 0);
  }

  #[test]
  fn resource_clear() {
    let mut wgpu = block_on(WgpuImpl::headless());
    let mut atlas =
      Atlas::<WgpuTexture, _, _>::new("_", ColorFormat::Rgba8, AntiAliasing::None, &mut wgpu);
    atlas.allocate(1, (), DeviceSize::new(32, 32), &mut wgpu);
    atlas.allocate(2, (), DeviceSize::new(4097, 16), &mut wgpu);
    atlas.end_frame();
    atlas.end_frame();
    wgpu.end_frame();

    assert!(atlas.extras.is_empty());
    assert!(atlas.atlas_allocator.is_empty());
  }
}
