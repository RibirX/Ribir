use std::hash::Hash;

use guillotiere::{Allocation, AtlasAllocator};
use ribir_algo::FrameCache;
use ribir_geom::{DeviceRect, DeviceSize};
use ribir_painter::image::ColorFormat;
use slab::Slab;

use super::Texture;
use crate::GPUBackendImpl;

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

pub(crate) struct AtlasConfig {
  label: &'static str,
  min_size: DeviceSize,
  max_size: DeviceSize,
}

pub(crate) struct Atlas<T: Texture, K, Attr> {
  config: AtlasConfig,
  atlas_allocator: AtlasAllocator,
  texture: T,
  cache: FrameCache<K, AtlasHandle<Attr>>,
  extras: Slab<T>,
  islands: Vec<AtlasHandle<Attr>>,
}

macro_rules! release_handle {
  ($this:ident, $handle:ident) => {
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
  pub fn new(config: AtlasConfig, format: ColorFormat, gpu_impl: &mut T::Host) -> Self {
    let min_size = config.min_size;
    let texture = gpu_impl.new_texture(min_size, format);
    Self {
      config,
      texture,
      atlas_allocator: AtlasAllocator::new(min_size.cast_unit()),
      cache: FrameCache::new(),
      extras: Slab::default(),
      islands: vec![],
    }
  }

  pub fn get(&mut self, key: &K) -> Option<&AtlasHandle<Attr>> { self.cache.get(key) }

  /// Allocate a rect in the atlas the caller should draw stull in the
  /// rect. Check if a cache exist before allocate.
  pub fn allocate(
    &mut self, key: K, attr: Attr, size: DeviceSize, gpu_impl: &mut T::Host,
  ) -> AtlasHandle<Attr>
  where
    Attr: Copy,
  {
    let current_size = self.size();
    let alloc_size = size.to_i32().cast_unit();
    let mut alloc = self.atlas_allocator.allocate(alloc_size);

    if alloc.is_none() {
      let expand_size = (current_size * 2)
        .max(current_size)
        .min(self.config.max_size);
      if expand_size != self.texture.size() {
        self.atlas_allocator.grow(expand_size.cast_unit());
        let mut new_tex = gpu_impl.new_texture(expand_size, self.texture.color_format());
        // Copy old texture to new texture item by item, not copy whole texture. Because
        // the new texture will overlap with the old texture. And we promise to the
        // gpu backend implementation that our operations not overlap in one texture in
        // one frame. So the implementation can batch and reorder the operations to
        // improve the performance.
        self
          .atlas_allocator
          .for_each_allocated_rectangle(|_, rect| {
            gpu_impl.copy_texture_from_texture(
              &mut new_tex,
              rect.min.cast_unit(),
              &self.texture,
              &rect.to_rect().cast_unit(),
            );
          });

        self.texture = new_tex;
        alloc = self.atlas_allocator.allocate(alloc_size);
      }
    }

    let atlas_dist = if let Some(alloc) = alloc {
      AtlasDist::Atlas(alloc)
    } else {
      let texture = gpu_impl.new_texture(size, self.texture.color_format());
      let id = self.extras.insert(texture);
      AtlasDist::Extra(id)
    };

    let handle = AtlasHandle { attr, atlas_dist };

    if let Some(h) = self.cache.put(key, handle) {
      // Hold the old handle until the frame end, because it's maybe used by other
      // commands.
      self.islands.push(h);
    }

    handle
  }

  /// Get a mut reference of a texture that `id` point to. The `id` get from
  /// `AtlasHandle::tex_id`
  pub fn get_texture_mut(&mut self, id: usize) -> &mut T {
    if id == 0 { &mut self.texture } else { &mut self.extras[id - 1] }
  }

  /// Get a reference of a texture that `id` point to. The `id` get from
  /// `AtlasHandle::tex_id`
  pub fn get_texture(&self, id: usize) -> &T {
    if id == 0 { &self.texture } else { &self.extras[id - 1] }
  }

  pub fn size(&self) -> DeviceSize { self.texture.size() }

  /// The max size of the atlas can be.
  pub fn max_size(&self) -> DeviceSize { self.config.max_size }

  pub fn is_good_size_to_alloc(&self, size: DeviceSize) -> bool {
    (!size.greater_than(self.config.max_size).any())
      && size.area() <= self.config.max_size.area() / 4
  }

  pub(crate) fn end_frame(&mut self) {
    self
      .cache
      .end_frame(self.config.label)
      .for_each(|h| release_handle!(self, h));
    self
      .islands
      .drain(..)
      .for_each(|h| release_handle!(self, h))
  }
}

impl AtlasConfig {
  pub fn new(label: &'static str, max_size: DeviceSize) -> Self {
    Self { label, min_size: max_size / 4, max_size }
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
  use futures::executor::block_on;

  use super::*;
  use crate::{WgpuImpl, WgpuTexture};
  #[test]
  fn atlas_grow_to_alloc() {
    let mut gpu_impl = block_on(WgpuImpl::headless());
    let mut atlas = Atlas::<WgpuTexture, _, _>::new(
      AtlasConfig::new("", DeviceSize::new(4096, 4096)),
      ColorFormat::Alpha8,
      &mut gpu_impl,
    );

    let size = DeviceSize::new(atlas.config.min_size.width + 1, 16);
    let h = atlas.allocate(1, (), size, &mut gpu_impl);
    gpu_impl.end_frame();
    assert_eq!(h.tex_id(), 0);
  }

  #[test]
  fn resource_clear() {
    let mut wgpu = block_on(WgpuImpl::headless());
    let size = wgpu.limits().texture_size;
    let mut atlas =
      Atlas::<WgpuTexture, _, _>::new(AtlasConfig::new("", size), ColorFormat::Rgba8, &mut wgpu);
    atlas.allocate(1, (), DeviceSize::new(32, 32), &mut wgpu);
    atlas.allocate(2, (), size, &mut wgpu);
    atlas.end_frame();
    atlas.end_frame();
    wgpu.end_frame();

    assert!(atlas.extras.is_empty());
    assert!(atlas.atlas_allocator.is_empty());
  }

  #[test]
  fn fix_scale_path_cache_miss() {
    let mut wgpu = block_on(WgpuImpl::headless());
    let mut atlas = Atlas::<WgpuTexture, _, _>::new(
      AtlasConfig::new("", DeviceSize::new(4096, 4096)),
      ColorFormat::Rgba8,
      &mut wgpu,
    );
    atlas.allocate(1, (), DeviceSize::new(32, 32), &mut wgpu);
    atlas.allocate(1, (), DeviceSize::new(512, 512), &mut wgpu); // before the frame end, two allocation for key(1) should keep.
    let mut alloc_count = 0;
    atlas
      .atlas_allocator
      .for_each_allocated_rectangle(|_, _| alloc_count += 1);
    assert_eq!(alloc_count, 2);

    atlas.end_frame();

    // after end frame, the smaller allocation of the keep should be release.
    alloc_count = 0;
    atlas
      .atlas_allocator
      .for_each_allocated_rectangle(|_, _| alloc_count += 1);
    assert_eq!(alloc_count, 1);
  }

  #[test]
  fn fix_atlas_expand_overlap() {
    let mut wgpu = block_on(WgpuImpl::headless());
    let mut atlas = Atlas::<WgpuTexture, _, _>::new(
      AtlasConfig::new("", DeviceSize::new(4096, 4096)),
      ColorFormat::Alpha8,
      &mut wgpu,
    );
    let icon = DeviceSize::new(32, 32);
    atlas.allocate(1, (), icon, &mut wgpu);

    atlas
      .texture
      .write_data(&DeviceRect::from_size(icon), &[1; 32 * 32], &mut wgpu);

    let min_size = atlas.config.min_size;
    // force atlas to expand
    let h = atlas.allocate(2, (), min_size, &mut wgpu);
    let second_rect = h.tex_rect(&atlas);
    let second_area: usize = (min_size.width * min_size.height) as usize;
    atlas
      .texture
      .write_data(&second_rect, &vec![2; second_area], &mut wgpu);
    let img = atlas
      .texture
      .copy_as_image(&DeviceRect::from_size(atlas.size()), &mut wgpu);

    wgpu.end_frame();
    let img = block_on(img).unwrap();

    // check sum of the texture.
    assert_eq!(
      img
        .pixel_bytes()
        .iter()
        .map(|v| *v as usize)
        .sum::<usize>(),
      icon.area() as usize + second_area * 2
    )
  }
}
