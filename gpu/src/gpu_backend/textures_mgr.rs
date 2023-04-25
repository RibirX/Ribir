use super::atlas::Atlas;
use super::Texture;
use crate::{gpu_backend::atlas::ATLAS_MAX_ITEM, GPUBackendImpl};
use guillotiere::AllocId;
use rayon::{prelude::ParallelIterator, slice::ParallelSlice};
use ribir_algo::{FrameCache, ShareResource};
use ribir_painter::{image::ColorFormat, DeviceRect, DeviceSize, PaintPath, Path, PixelImage};
use ribir_painter::{DeviceVector, Vertex, VertexBuffers};
use slab::Slab;
use std::ops::Range;
const TOLERANCE: f32 = 0.1;
const PAR_CHUNKS_SIZE: usize = 128;

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Copy)]
pub(super) enum TextureID {
  AlphaAtlas,
  RgbaAtlas,
  Extra(u32),
}

pub(super) struct TexturesMgr<T: Texture> {
  alpha_atlas: Atlas<T>,
  rgba_atlas: Atlas<T>,
  extra_textures: Slab<T>,
  path_cache: FrameCache<unsafe_path::CachePath, TextureDist>,
  resource_cache: FrameCache<*const (), TextureDist>,
  uncached: Vec<TextureDist>,
  fill_task: Vec<FillTask>,
  fill_task_buffers: VertexBuffers<()>,
}

struct FillTask {
  id: TextureID,
  offset: DeviceVector,
  path: Path,
  clip_rect: Option<DeviceRect>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct TextureSlice {
  pub(super) tex_id: TextureID,
  pub(super) rect: DeviceRect,
}

macro_rules! id_to_texture {
  ($mgr: ident, $id: expr) => {
    match $id {
      TextureID::AlphaAtlas => &mut $mgr.alpha_atlas.texture,
      TextureID::RgbaAtlas => &mut $mgr.rgba_atlas.texture,
      TextureID::Extra(id) => &mut $mgr.extra_textures[id as usize],
    }
  };
}

impl<T: Texture> TexturesMgr<T>
where
  T::Host: GPUBackendImpl<Texture = T>,
{
  pub(super) fn new(gpu_impl: &mut T::Host) -> Self {
    Self {
      alpha_atlas: Atlas::new(ColorFormat::Alpha8, gpu_impl),
      rgba_atlas: Atlas::new(ColorFormat::Rgba8, gpu_impl),
      extra_textures: <_>::default(),
      path_cache: <_>::default(),
      resource_cache: <_>::default(),
      uncached: <_>::default(),
      fill_task: <_>::default(),
      fill_task_buffers: <_>::default(),
    }
  }

  /// Store an alpha path in texture and return the texture.
  pub(super) fn store_alpha_path(
    &mut self,
    path: PaintPath,
    gpu_impl: &mut T::Host,
  ) -> TextureSlice {
    if let Some(dist) = self.path_cache.get((&path.path).into()) {
      return self.text_dist_to_rect(*dist);
    }
    let tex_dist = self.inner_alloc(ColorFormat::Alpha8, path.bounds.size, gpu_impl);
    self.path_cache.insert(path.path.clone().into(), tex_dist);
    let texture = self.text_dist_to_rect(tex_dist);
    self.fill_task.push(FillTask {
      id: tex_dist.texture_id(),
      offset: texture.rect.origin.to_vector(),
      path: path.path,
      clip_rect: None,
    });
    texture
  }

  pub(super) fn alloc_path_without_cache(
    &mut self,
    intersect_view: DeviceRect,
    path: PaintPath,
    gpu_impl: &mut T::Host,
  ) -> TextureSlice {
    let tex_dist = self.inner_alloc(ColorFormat::Alpha8, intersect_view.size, gpu_impl);
    self.uncached.push(tex_dist);
    let texture = self.text_dist_to_rect(tex_dist);
    let offset = texture.rect.origin + (path.bounds.origin - intersect_view.origin);
    let task = FillTask {
      id: tex_dist.texture_id(),
      offset: offset.to_vector(),
      path: path.path,
      clip_rect: Some(texture.rect),
    };
    self.fill_task.push(task);
    texture
  }

  pub(super) fn store_image(
    &mut self,
    img: &ShareResource<PixelImage>,
    gpu_impl: &mut T::Host,
  ) -> TextureSlice {
    if let Some(dist) = self.resource_cache.get(&ShareResource::as_ptr(img)) {
      return self.text_dist_to_rect(*dist);
    }

    let size = DeviceSize::new(img.width() as i32, img.height() as i32);
    let tex_dist = self.inner_alloc(img.color_format(), size, gpu_impl);
    let tex_rect = self.text_dist_to_rect(tex_dist);

    let texture = self.texture_mut(tex_rect.tex_id);
    texture.write_data(&tex_rect.rect, img.pixel_bytes(), gpu_impl);

    self
      .resource_cache
      .insert(ShareResource::as_ptr(img), tex_dist);

    tex_rect
  }

  pub(super) fn alloc(
    &mut self,
    size: DeviceSize,
    format: ColorFormat,
    gpu_impl: &mut T::Host,
  ) -> TextureSlice {
    let tex_dist = self.inner_alloc(format, size, gpu_impl);
    self.uncached.push(tex_dist);
    self.text_dist_to_rect(tex_dist)
  }

  fn inner_alloc(
    &mut self,
    format: ColorFormat,
    size: DeviceSize,
    gpu_impl: &mut T::Host,
  ) -> TextureDist {
    if valid_atlas_item(&size) {
      let (tex_id, alloc) = match format {
        ColorFormat::Rgba8 => (
          TextureID::RgbaAtlas,
          self.rgba_atlas.allocate(size, gpu_impl),
        ),
        ColorFormat::Alpha8 => (
          TextureID::AlphaAtlas,
          self.alpha_atlas.allocate(size, gpu_impl),
        ),
      };
      if let Some(alloc) = alloc {
        let rect = alloc.rectangle.to_rect().cast_unit();
        let tex_rect = TextureSlice { tex_id, rect };
        return TextureDist::Atlas { alloc_id: alloc.id, tex_rect };
      }
    }
    let texture = gpu_impl.new_texture(size, format);
    let id = self.extra_textures.insert(texture);
    TextureDist::Extra(id as u32)
  }

  pub(super) fn texture_mut(&mut self, tex_id: TextureID) -> &mut T { id_to_texture!(self, tex_id) }

  pub(super) fn texture(&self, tex_id: TextureID) -> &T {
    match tex_id {
      TextureID::AlphaAtlas => &self.alpha_atlas.texture,
      TextureID::RgbaAtlas => &self.rgba_atlas.texture,
      TextureID::Extra(id) => &self.extra_textures[id as usize],
    }
  }

  fn text_dist_to_rect(&self, dist: TextureDist) -> TextureSlice {
    match dist {
      TextureDist::Atlas { tex_rect, .. } => tex_rect,
      TextureDist::Extra(id) => TextureSlice {
        tex_id: TextureID::Extra(id),
        rect: DeviceRect::from_size(self.extra_textures[id as usize].size()),
      },
    }
  }

  pub(crate) fn submit(&mut self, gpu_impl: &mut impl GPUBackendImpl<Texture = T>) {
    if self.fill_task.is_empty() {
      return;
    }

    self
      .fill_task
      .sort_by(|a, b| b.clip_rect.is_some().cmp(&a.clip_rect.is_some()));

    fn tess_tasks(
      tasks: &[FillTask],
      buffer: &mut VertexBuffers<()>,
    ) -> Vec<(TextureID, Option<DeviceRect>, Range<u32>)> {
      tasks
        .iter()
        .map(|FillTask { id, offset, path, clip_rect }| {
          let start = buffer.indices.len() as u32;

          let offset = offset.to_f32().cast_unit();
          path.tessellate(TOLERANCE, buffer, |pos| Vertex {
            attr: (),
            pos: (pos + offset).to_array(),
          });
          (*id, *clip_rect, start..buffer.indices.len() as u32)
        })
        .collect()
    }

    let draw_rgs = if self.fill_task.len() < PAR_CHUNKS_SIZE {
      tess_tasks(&self.fill_task, &mut self.fill_task_buffers)
    } else {
      let res = self
        .fill_task
        .par_chunks(PAR_CHUNKS_SIZE)
        .map(|tasks| {
          let mut buffers = VertexBuffers::default();
          let rgs = tess_tasks(tasks, &mut buffers);
          (buffers, rgs)
        })
        .collect::<Vec<_>>();
      res
        .into_iter()
        .fold(vec![], |mut draw_rgs, (buffers, rgs)| {
          extend_buffer(&mut self.fill_task_buffers, buffers);
          draw_rgs.extend(rgs.into_iter());
          draw_rgs
        })
    };
    gpu_impl.load_alpha_vertices(&self.fill_task_buffers);

    let mut idx = 0;
    loop {
      if idx >= draw_rgs.len() {
        break;
      }

      let (tex_id, Some(clip_rect), rg) = &draw_rgs[idx] else { break; };
      let texture = id_to_texture!(self, *tex_id);
      gpu_impl.draw_alpha_triangles_with_scissor(rg, texture, *clip_rect);
      idx += 1;
    }

    loop {
      if idx >= draw_rgs.len() {
        break;
      }
      let (tex_id,None, rg) = &draw_rgs[idx] else { unreachable!(); };
      let next = draw_rgs[idx..]
        .iter()
        .position(|(next, _, _)| tex_id != next);

      let texture = id_to_texture!(self, *tex_id);
      if let Some(mut next) = next {
        next += idx;
        let (_, _, end) = &draw_rgs[next];
        let indices = rg.start..end.start;
        gpu_impl.draw_alpha_triangles(&indices, texture);

        idx = next;
      } else {
        let indices = rg.start..self.fill_task_buffers.indices.len() as u32;
        gpu_impl.draw_alpha_triangles(&indices, texture);
        idx = draw_rgs.len();
      }
    }

    self.fill_task.clear();
    self.fill_task_buffers.vertices.clear();
  }

  pub(crate) fn end_frame(&mut self) {
    self.uncached.drain(..).for_each(|u| match u {
      TextureDist::Atlas { alloc_id, tex_rect } => match tex_rect.tex_id {
        TextureID::AlphaAtlas => {
          self.alpha_atlas.deallocate(alloc_id);
        }
        TextureID::RgbaAtlas => {
          self.rgba_atlas.deallocate(alloc_id);
        }
        TextureID::Extra(_) => unreachable!(),
      },
      TextureDist::Extra(id) => {
        self.extra_textures.try_remove(id as usize);
      }
    });
    self.resource_cache.end_frame("image atlas");
    self.path_cache.end_frame("path atlas");
    if self.rgba_atlas.hint_clear() {
      self.rgba_atlas.clear();
    }
    if self.alpha_atlas.hint_clear() {
      self.alpha_atlas.clear();
    }
  }
}

#[derive(Debug, Clone, Copy)]
enum TextureDist {
  Atlas {
    alloc_id: AllocId,
    tex_rect: TextureSlice,
  },
  Extra(u32),
}

impl TextureDist {
  fn texture_id(&self) -> TextureID {
    match self {
      TextureDist::Atlas { tex_rect, .. } => tex_rect.tex_id,
      TextureDist::Extra(id) => TextureID::Extra(*id),
    }
  }
}

pub(crate) fn valid_atlas_item(size: &DeviceSize) -> bool { size.lower_than(ATLAS_MAX_ITEM).any() }

// Use a fast but not precise way to hash our path.
mod unsafe_path {
  use ribir_painter::{DevicePoint, Path, Point};
  use std::hash::Hash;

  #[derive(Hash, PartialEq, Eq, Clone)]
  enum Verb {
    _LineTo,
    _QuadraticTo,
    _CubicTo,
    _Begin,
    _Close,
    _End,
  }

  pub(crate) struct CachePath {
    points: Box<[Point]>,
    verbs: Box<[Verb]>,
    num_attributes: usize,
  }

  impl From<Path> for CachePath {
    fn from(value: Path) -> Self { unsafe { std::mem::transmute(value) } }
  }

  impl From<&Path> for &CachePath {
    fn from(value: &Path) -> Self { unsafe { std::mem::transmute(value) } }
  }

  //  keep 0.1 device pixel precision
  fn precision_point(p: &Point) -> DevicePoint { (*p * 10.).to_i32().cast_unit() }

  impl Hash for CachePath {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
      self
        .points
        .iter()
        .for_each(|p| precision_point(p).hash(state));

      self.verbs.hash(state);
      self.num_attributes.hash(state);
    }
  }

  impl PartialEq for CachePath {
    fn eq(&self, other: &Self) -> bool {
      self
        .points
        .iter()
        .map(precision_point)
        .eq(other.points.iter().map(precision_point))
        && self.verbs == other.verbs
        && self.num_attributes == other.num_attributes
    }
  }

  impl Eq for CachePath {}
}

fn extend_buffer<V>(dist: &mut VertexBuffers<V>, from: VertexBuffers<V>) {
  if dist.vertices.is_empty() {
    dist.vertices.extend(from.vertices.into_iter());
    dist.indices.extend(from.indices.into_iter());
  } else {
    let offset = dist.indices.len() as u32;
    dist
      .indices
      .extend(from.indices.into_iter().map(|i| offset + i));
    dist.vertices.extend(from.vertices.into_iter());
  }
}

#[cfg(feature = "wgpu")]
#[cfg(test)]
pub mod tests {
  use crate::WgpuImpl;

  use super::*;
  use futures::executor::block_on;
  use ribir_algo::ShareResource;
  use ribir_painter::{image::ColorFormat, Color};
  use std::borrow::Cow;

  pub fn color_image(color: Color, width: u32, height: u32) -> ShareResource<PixelImage> {
    let data = std::iter::repeat(color.into_components())
      .take(width as usize * height as usize)
      .flatten()
      .collect::<Vec<_>>();

    let img = PixelImage::new(Cow::Owned(data), width, height, ColorFormat::Rgba8);
    ShareResource::new(img)
  }

  #[test]
  fn smoke_store_image() {
    use crate::wgpu_impl::WgpuImpl;
    use ribir_painter::AntiAliasing;

    let mut wgpu = block_on(WgpuImpl::headless(AntiAliasing::None));
    let mut mgr = TexturesMgr::new(&mut wgpu);

    let red_img = color_image(Color::RED, 32, 32);
    let red_rect = mgr.store_image(&red_img, &mut wgpu);

    assert_eq!(red_rect.rect.min().to_array(), [0, 0]);

    // same image should have same position in atlas
    assert_eq!(red_rect, mgr.store_image(&red_img, &mut wgpu));
    color_img_check(&mgr, &red_rect, &mut wgpu, Color::RED);

    let yellow_img = color_image(Color::YELLOW, 64, 64);
    let yellow_rect = mgr.store_image(&yellow_img, &mut wgpu);

    // the color should keep after atlas rearrange
    color_img_check(&mgr, &red_rect, &mut wgpu, Color::RED);
    color_img_check(&mgr, &yellow_rect, &mut wgpu, Color::YELLOW);

    let extra_blue_img = color_image(Color::BLUE, 1024, 1024);
    let blue_rect = mgr.store_image(&extra_blue_img, &mut wgpu);

    color_img_check(&mgr, &blue_rect, &mut wgpu, Color::BLUE);
    color_img_check(&mgr, &red_rect, &mut wgpu, Color::RED);
    color_img_check(&mgr, &yellow_rect, &mut wgpu, Color::YELLOW);
  }

  fn color_img_check(
    mgr: &TexturesMgr<wgpu::Texture>,
    rect: &TextureSlice,
    wgpu: &mut WgpuImpl,
    color: Color,
  ) {
    wgpu.begin_frame();
    let texture = mgr.texture(rect.tex_id);
    let img = texture.copy_as_image(&rect.rect, wgpu);
    wgpu.end_frame();

    let img = block_on(img).unwrap();
    assert!(
      img
        .pixel_bytes()
        .chunks(4)
        .all(|c| c == &color.into_components())
    );
  }
}
