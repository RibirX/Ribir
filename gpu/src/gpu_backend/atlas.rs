use std::hash::Hash;

use guillotiere::{Allocation, AtlasAllocator};
use ribir_painter::ColorFormat;
use ribir_types::{DeviceRect, DeviceSize};
use slab::Slab;

use super::Texture;
use crate::GPUBackendImpl;

#[derive(Copy, Clone, Debug, PartialEq)]
pub(super) enum AtlasDist {
  Atlas(Allocation),
  Extra(usize),
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct AtlasHandle {
  pub scale: f32,
  pub dist: AtlasDist,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct AtlasEntry {
  pub handle: AtlasHandle,
  pub last_access_frame: u64,
}

pub(crate) struct AtlasConfig {
  label: &'static str,
  min_size: DeviceSize,
  max_size: DeviceSize,
}

pub(crate) struct Atlas<K, T: Texture> {
  config: AtlasConfig,
  atlas_allocator: AtlasAllocator,
  texture: T,
  cache: ahash::HashMap<K, AtlasEntry>,
  /// Extra textures which store only single allocation.
  extras: Slab<T>,
  current_frame: u64,
  allocated_area: i32,
  last_alloc_failed: bool,
  /// All allocations in the current frame and not cached.
  islands: ahash::HashSet<AtlasDist>,
}

impl<K, T: Texture> Atlas<K, T>
where
  K: Hash + Eq + Clone,
  T::Host: GPUBackendImpl<Texture = T>,
{
  pub const MILD_THRESHOLD: f32 = 0.6;
  pub const AGGRESSIVE_THRESHOLD: f32 = 0.8;
  pub const MILD_TTL: u64 = 60;
  pub const EXTRA_TTL: u64 = 5;

  pub fn new(config: AtlasConfig, format: ColorFormat, gpu_impl: &mut T::Host) -> Self {
    let min_size = config.min_size;
    let texture = gpu_impl.new_texture(min_size, format);
    Self {
      config,
      texture,
      atlas_allocator: AtlasAllocator::new(min_size.cast_unit()),
      cache: ahash::HashMap::default(),
      extras: Slab::default(),
      current_frame: 0,
      allocated_area: 0,
      last_alloc_failed: false,
      islands: <_>::default(),
    }
  }

  pub fn get(&mut self, key: &K, scale: f32) -> Option<&AtlasHandle> {
    let current_frame = self.current_frame;
    self.cache.get_mut(key).and_then(|entry| {
      if entry.handle.scale >= scale * 0.95 {
        entry.last_access_frame = current_frame;
        Some(&entry.handle)
      } else {
        None
      }
    })
  }

  /// Cache a handle to the atlas. If the key already exists, the old handle
  /// will be replaced
  pub fn cache(&mut self, key: K, scale: f32, dist: AtlasDist) -> AtlasHandle {
    let handle = AtlasHandle { scale, dist };

    if self.islands.contains(&dist) {
      self.islands.remove(&dist);
    }

    if let Some(old_entry) = self
      .cache
      .insert(key, AtlasEntry { handle, last_access_frame: self.current_frame })
    {
      self.islands.insert(old_entry.handle.dist);
    }

    handle
  }

  /// Return the handle of cached resource. If the resource is not cached,
  /// allocate it and call `init` to initialize the texture.
  pub fn get_or_cache(
    &mut self, key: K, scale: f32, size: DeviceSize, gpu: &mut T::Host,
    init: impl FnOnce(&DeviceRect, &mut T, &mut T::Host),
  ) -> AtlasHandle {
    if let Some(h) = self.get(&key, scale) {
      return *h;
    }

    let dist = self.allocate(size, gpu);
    let h = self.cache(key, scale, dist);
    init(&h.tex_rect(self), self.get_texture_mut(h.tex_id()), gpu);

    h
  }
  /// Allocate a rect in the atlas
  pub fn allocate(&mut self, size: DeviceSize, gpu_impl: &mut T::Host) -> AtlasDist {
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

    let dist = if let Some(alloc) = alloc {
      self.allocated_area += size.area();
      AtlasDist::Atlas(alloc)
    } else {
      self.last_alloc_failed = true;
      let texture = gpu_impl.new_texture(size, self.texture.color_format());
      let id = self.extras.insert(texture);
      AtlasDist::Extra(id)
    };
    self.islands.insert(dist);

    dist
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

  pub(crate) fn end_frame(&mut self) { self.end_frame_with(|_| {}) }

  pub(crate) fn end_frame_with(&mut self, mut on_deallocate: impl FnMut(DeviceRect)) {
    let capacity_ratio = self.allocated_area as f32 / self.size().area() as f32;
    let is_aggressive = self.last_alloc_failed || capacity_ratio >= Self::AGGRESSIVE_THRESHOLD;
    let is_mild = capacity_ratio >= Self::MILD_THRESHOLD;

    tracing::info!("Atlas[{}]: cache percent is {:.1}%", self.config.label, capacity_ratio);

    let current_frame = self.current_frame;
    let mut to_remove_keys = Vec::new();

    for (key, entry) in self.cache.iter() {
      let idle_frames = current_frame.saturating_sub(entry.last_access_frame);
      let should_evict = match entry.handle.dist {
        AtlasDist::Atlas(_) => {
          (is_aggressive && idle_frames > 0) || (is_mild && idle_frames >= Self::MILD_TTL)
        }
        AtlasDist::Extra(_) => idle_frames >= Self::EXTRA_TTL,
      };

      if should_evict {
        to_remove_keys.push(key.clone());
      }
    }

    let mut remove_dist = |dist: AtlasDist| match dist {
      AtlasDist::Atlas(alloc) => {
        on_deallocate(alloc.rectangle.to_rect().cast_unit());
        self.atlas_allocator.deallocate(alloc.id);
        self.allocated_area -= alloc.rectangle.area();
      }
      AtlasDist::Extra(id) => {
        self.extras.remove(id);
      }
    };

    for key in to_remove_keys {
      if let Some(entry) = self.cache.remove(&key) {
        remove_dist(entry.handle.dist);
      }
    }

    for dist in self.islands.drain() {
      remove_dist(dist);
    }

    self.last_alloc_failed = false;
    self.current_frame += 1;
  }
}

impl AtlasConfig {
  pub fn new(label: &'static str, max_size: DeviceSize) -> Self {
    Self { label, min_size: max_size / 8, max_size }
  }
}

impl AtlasDist {
  pub fn tex_id(&self) -> usize {
    match self {
      AtlasDist::Atlas(_) => 0,
      AtlasDist::Extra(id) => *id + 1,
    }
  }

  pub(super) fn tex_rect<K, T>(&self, atlas: &Atlas<K, T>) -> DeviceRect
  where
    T: Texture,
  {
    match self {
      AtlasDist::Atlas(alloc) => alloc.rectangle.to_rect().cast_unit(),
      AtlasDist::Extra(id) => DeviceRect::from_size(atlas.extras[*id].size()),
    }
  }
}

impl AtlasHandle {
  pub fn tex_id(&self) -> usize {
    match &self.dist {
      AtlasDist::Atlas(_) => 0,
      AtlasDist::Extra(id) => *id + 1,
    }
  }

  pub(super) fn tex_rect<K, T>(&self, atlas: &Atlas<K, T>) -> DeviceRect
  where
    T: Texture,
  {
    match &self.dist {
      AtlasDist::Atlas(alloc) => alloc.rectangle.to_rect().cast_unit(),
      AtlasDist::Extra(id) => DeviceRect::from_size(atlas.extras[*id].size()),
    }
  }
}

impl Hash for AtlasDist {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    match self {
      // hash id enough, because the id is unique.
      AtlasDist::Atlas(alloc) => alloc.id.hash(state),
      AtlasDist::Extra(id) => id.hash(state),
    }
  }
}

impl Eq for AtlasDist {}

#[cfg(feature = "wgpu")]
#[cfg(test)]
mod tests {
  use std::any::Any;

  use futures::executor::block_on;
  use ribir_algo::Resource;

  use super::*;
  use crate::{WgpuImpl, WgpuTexture};

  #[test]
  fn resource_hit() {
    let mut gpu = block_on(WgpuImpl::headless());
    let size = gpu.limits().texture_size;
    let mut atlas = Atlas::<Resource<dyn Any>, WgpuTexture>::new(
      AtlasConfig::new("", size),
      ColorFormat::Rgba8,
      &mut gpu,
    );
    let resource = Resource::new(1);
    let h1 = atlas.get_or_cache(resource.clone().into_any(), 1., size, &mut gpu, |_, _, _| {});
    let h2 = atlas.get_or_cache(resource.clone().into_any(), 0.8, size, &mut gpu, |_, _, _| {});
    let h3 = atlas.get_or_cache(resource.clone().into_any(), 2., size, &mut gpu, |_, _, _| {});
    let h4 = atlas.get_or_cache(resource.clone().into_any(), 1., size, &mut gpu, |_, _, _| {});

    assert_eq!(h1, h2);
    assert_ne!(h2, h3);
    assert_eq!(h4, h3);
  }

  #[test]
  fn atlas_grow_to_alloc() {
    let mut gpu_impl = block_on(WgpuImpl::headless());
    let mut atlas = Atlas::<Resource<dyn Any>, WgpuTexture>::new(
      AtlasConfig::new("", DeviceSize::new(4096, 4096)),
      ColorFormat::Alpha8,
      &mut gpu_impl,
    );

    let size = DeviceSize::new(atlas.config.min_size.width + 1, 16);
    let dist = atlas.allocate(size, &mut gpu_impl);
    atlas.cache(Resource::new(1).into_any(), 1., dist);

    gpu_impl.end_frame();
    assert_eq!(dist.tex_id(), 0);
  }

  #[test]
  fn resource_clear() {
    let mut wgpu = block_on(WgpuImpl::headless());
    let size = wgpu.limits().texture_size;
    let mut atlas = Atlas::<Resource<dyn Any>, WgpuTexture>::new(
      AtlasConfig::new("", size),
      ColorFormat::Rgba8,
      &mut wgpu,
    );
    let dist = atlas.allocate(DeviceSize::new(32, 32), &mut wgpu);
    atlas.cache(Resource::new(1).into_any(), 1., dist);

    // end frame 0.
    atlas.end_frame();

    // in frame 1, cause an allocation to fail in Atlas, forcing Extra and
    // last_alloc_failed = true
    atlas.allocate(size, &mut wgpu);

    // advance 5 frames to clear Extra as well.
    for _ in 0..5 {
      atlas.end_frame();
    }
    wgpu.end_frame();

    assert!(atlas.extras.is_empty());
    assert!(atlas.atlas_allocator.is_empty());
  }

  #[test]
  fn fix_scale_path_cache_miss() {
    let mut wgpu = block_on(WgpuImpl::headless());
    let mut atlas = Atlas::<Resource<dyn Any>, WgpuTexture>::new(
      AtlasConfig::new("", DeviceSize::new(4096, 4096)),
      ColorFormat::Rgba8,
      &mut wgpu,
    );
    let key = Resource::new(1).into_any();
    let dist = atlas.allocate(DeviceSize::new(32, 32), &mut wgpu);
    atlas.cache(key.clone(), 1., dist);
    let dist = atlas.allocate(DeviceSize::new(512, 512), &mut wgpu);
    // before the frame end, two allocation for key should keep.
    atlas.cache(key, 1., dist);

    let mut alloc_count = 0;
    atlas
      .atlas_allocator
      .for_each_allocated_rectangle(|_, _| alloc_count += 1);
    assert_eq!(alloc_count, 2);

    atlas.end_frame(); // Frame 0, islands drained, 2 allocations in allocator

    // Advance 60 frames to trigger eviction of the replaced old_entry in islands
    // logic now moved. Wait, the replaced entry is in `cache`? No, it's NOT in
    // `cache`, but it IS in `islands` from line 87 So `end_frame` at line 406
    // already deallocates it from allocator!

    // Oh, `islands` drained means it is deallocated. Let's check alloc_count.
    alloc_count = 0;
    atlas
      .atlas_allocator
      .for_each_allocated_rectangle(|_, _| alloc_count += 1);
    assert_eq!(alloc_count, 1);
  }

  #[test]
  fn fix_atlas_expand_overlap() {
    let mut wgpu = block_on(WgpuImpl::headless());
    let mut atlas = Atlas::<Resource<dyn Any>, WgpuTexture>::new(
      AtlasConfig::new("", DeviceSize::new(4096, 4096)),
      ColorFormat::Alpha8,
      &mut wgpu,
    );
    let icon = DeviceSize::new(32, 32);
    atlas.allocate(icon, &mut wgpu);

    atlas
      .texture
      .write_data(&DeviceRect::from_size(icon), &[1; 32 * 32], &mut wgpu);

    let min_size = atlas.config.min_size;
    // force atlas to expand
    let h = atlas.allocate(min_size, &mut wgpu);
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

  #[test]
  fn test_capacity_aware_gc() {
    let mut wgpu = block_on(WgpuImpl::headless());
    let mut atlas = Atlas::<Resource<dyn Any>, WgpuTexture>::new(
      AtlasConfig::new("", DeviceSize::new(4096, 4096)),
      ColorFormat::Rgba8,
      &mut wgpu,
    );

    // allocate an item directly that occupies > 60% of min_size (which is 512x512)
    // 512 * 512 = 262144 area. 60% is 157286. 400x400 is 160000.
    let dist1 = atlas.allocate(DeviceSize::new(400, 400), &mut wgpu);
    let key1 = Resource::new(1).into_any();
    atlas.cache(key1.clone(), 1., dist1);

    // Initial end_frame, capacity > 60%, so mild GC is armed.
    // However, idle_frame = 0, so it gets kept.
    atlas.end_frame();
    assert_eq!(atlas.cache.len(), 1);

    // Still kept before MILD_TTL frames.
    for _ in 0..Atlas::<Resource<dyn Any>, WgpuTexture>::MILD_TTL - 2 {
      atlas.end_frame();
    }
    assert_eq!(atlas.cache.len(), 1);

    // Reached MILD_TTL, it gets reclaimed by mild GC.
    atlas.end_frame();
    atlas.end_frame();
    assert_eq!(atlas.cache.len(), 0);

    wgpu.end_frame();
  }
}
