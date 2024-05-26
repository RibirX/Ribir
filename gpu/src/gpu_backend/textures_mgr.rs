use std::{any::Any, cmp::Ordering, hash::Hash, ops::Range};

use guillotiere::euclid::SideOffsets2D;
use rayon::{prelude::ParallelIterator, slice::ParallelSlice};
use ribir_algo::Resource;
use ribir_geom::{transform_to_device_rect, DeviceRect, DeviceSize, Size, Transform};
use ribir_painter::{image::ColorFormat, PaintPath, Path, PixelImage, Vertex, VertexBuffers};

use super::{
  atlas::{Atlas, AtlasConfig, AtlasDist},
  Texture,
};
use crate::GPUBackendImpl;
const TOLERANCE: f32 = 0.1_f32;
const PAR_CHUNKS_SIZE: usize = 64;

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Copy)]
pub(super) enum TextureID {
  Alpha(usize),
  Rgba(usize),
  Bundle(usize),
}

pub(super) struct TexturesMgr<T: Texture> {
  alpha_atlas: Atlas<T>,
  rgba_atlas: Atlas<T>,
  /// Similar to the `rgba_atlas`, this is used to allocate the target texture
  /// for drawing commands.
  ///
  /// We keep it separate from `rgba_atlas` because the backend may not permit a
  /// texture to be used both as a target and as a sampled resource in the same
  /// draw call.
  target_atlas: Atlas<T>,
  fill_task: Vec<FillTask>,
  fill_task_buffers: VertexBuffers<()>,
  need_clear_areas: Vec<DeviceRect>,
}

struct FillTask {
  slice: TextureSlice,
  path: PaintPath,
  // transform to construct vertex
  transform: Transform,
  clip_rect: Option<DeviceRect>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct TextureSlice {
  pub(super) tex_id: TextureID,
  pub(super) rect: DeviceRect,
}

macro_rules! id_to_texture_mut {
  ($mgr:ident, $id:expr) => {
    match $id {
      TextureID::Alpha(id) => $mgr.alpha_atlas.get_texture_mut(id),
      TextureID::Rgba(id) => $mgr.rgba_atlas.get_texture_mut(id),
      TextureID::Bundle(id) => $mgr.target_atlas.get_texture_mut(id),
    }
  };
}

macro_rules! id_to_texture {
  ($mgr:ident, $id:expr) => {
    match $id {
      TextureID::Alpha(id) => $mgr.alpha_atlas.get_texture(id),
      TextureID::Rgba(id) => $mgr.rgba_atlas.get_texture(id),
      TextureID::Bundle(id) => $mgr.target_atlas.get_texture(id),
    }
  };
}

impl<T: Texture> TexturesMgr<T>
where
  T::Host: GPUBackendImpl<Texture = T>,
{
  pub(super) fn new(gpu_impl: &mut T::Host) -> Self {
    let limits = gpu_impl.limits();
    let max_size = limits.texture_size;

    Self {
      alpha_atlas: Atlas::new(
        AtlasConfig::new("Alpha atlas", max_size),
        ColorFormat::Alpha8,
        gpu_impl,
      ),
      rgba_atlas: Atlas::new(
        AtlasConfig::new("Rgba atlas", max_size),
        ColorFormat::Rgba8,
        gpu_impl,
      ),
      target_atlas: Atlas::new(
        AtlasConfig::new("Bundle atlas", max_size),
        ColorFormat::Rgba8,
        gpu_impl,
      ),
      fill_task: <_>::default(),
      fill_task_buffers: <_>::default(),
      need_clear_areas: vec![],
    }
  }

  /// Store an alpha path in texture and return the texture and a transform that
  /// can transform the mask to viewport
  pub(super) fn store_alpha_path(
    &mut self, path: &PaintPath, matrix: &Transform, viewport: &DeviceRect, gpu: &mut T::Host,
  ) -> (TextureSlice, Transform) {
    match path {
      PaintPath::Share(p) => {
        let cache_scale: f32 = self.cache_scale(&path.bounds().size, matrix);
        let key = p.clone().into_any();
        let (slice, scale) = if let Some(h) = self.alpha_atlas.get(&key, cache_scale).copied() {
          let mask_slice = self.alpha_atlas_dist_to_tex_silice(&h.dist);
          (mask_slice, h.scale)
        } else {
          let scale_bounds = p.bounds().scale(cache_scale, cache_scale);
          let (dist, slice) =
            self.alpha_allocate(scale_bounds.round_out().size.to_i32().cast_unit(), gpu);
          let _ = self.alpha_atlas.cache(key, cache_scale, dist);
          let offset = slice.rect.origin.to_f32().cast_unit() - scale_bounds.origin;
          let transform = Transform::scale(cache_scale, cache_scale).then_translate(offset);
          self
            .fill_task
            .push(FillTask { slice, path: path.clone(), transform, clip_rect: None });
          (slice, cache_scale)
        };

        let path_origin = p.bounds().origin * scale;
        let slice_origin = slice.rect.origin.to_vector().to_f32();
        // back to slice origin
        let matrix = Transform::translation(-slice_origin.x, -slice_origin.y)
          // move to cached path axis.
          .then_translate(path_origin.to_vector().cast_unit())
          // scale back to path axis.
          .then_scale(1. / scale, 1. / scale)
          // apply path transform matrix to view.
          .then(matrix);

        (slice.expand_for_paste(), matrix)
      }
      PaintPath::Own(_) => {
        let paint_bounds = transform_to_device_rect(path.bounds(), matrix);
        let alloc_size = size_expand_blank(paint_bounds.size);

        let (visual_rect, clip_rect) = if self.alpha_atlas.is_good_size_to_alloc(alloc_size) {
          (paint_bounds, None)
        } else {
          // We intersect the path bounds with the viewport to reduce the number of pixels
          // drawn for large paths.
          let visual_rect = paint_bounds.intersection(viewport).unwrap();
          (visual_rect, Some(visual_rect))
        };

        let (_, slice) = self.alpha_allocate(visual_rect.size, gpu);
        let offset = (slice.rect.origin - visual_rect.origin)
          .to_f32()
          .cast_unit();
        let ts = matrix.then_translate(offset);
        let task = FillTask { slice, transform: ts, path: path.clone(), clip_rect };
        self.fill_task.push(task);

        let offset = (visual_rect.origin - slice.rect.origin).to_f32();
        (slice.expand_for_paste(), Transform::translation(offset.x, offset.y))
      }
    }
  }

  pub(super) fn store_image(
    &mut self, img: &Resource<PixelImage>, gpu: &mut T::Host,
  ) -> TextureSlice {
    let atlas = match img.color_format() {
      ColorFormat::Rgba8 => &mut self.rgba_atlas,
      ColorFormat::Alpha8 => &mut self.alpha_atlas,
    };

    let h =
      atlas.get_or_cache(img.clone().into_any(), 1., img.size(), gpu, |rect, texture, gpu| {
        texture.write_data(rect, img.pixel_bytes(), gpu)
      });

    TextureSlice { tex_id: TextureID::Rgba(h.tex_id()), rect: h.tex_rect(atlas) }
  }

  pub(super) fn store_commands(
    &mut self, size: DeviceSize, target: Resource<dyn Any>, scale: f32, gpu: &mut T::Host,
    init: impl FnOnce(&DeviceRect, &mut T, &mut T::Host),
  ) -> (f32, TextureSlice) {
    let dist = self
      .target_atlas
      .get_or_cache(target, scale, size, gpu, init);
    (
      dist.scale,
      TextureSlice {
        tex_id: TextureID::Bundle(dist.tex_id()),
        rect: dist.tex_rect(&self.target_atlas),
      },
    )
  }

  pub(super) fn texture(&self, tex_id: TextureID) -> &T { id_to_texture!(self, tex_id) }

  fn alpha_allocate(
    &mut self, mut size: DeviceSize, gpu: &mut T::Host,
  ) -> (AtlasDist, TextureSlice) {
    size = size_expand_blank(size);
    // Allocate with a 2-pixel blank edge to ensure that neighboring slices do not
    // affect the current slice.
    let dist = self.alpha_atlas.allocate(size, gpu);

    (dist, self.alpha_atlas_dist_to_tex_silice(&dist))
  }

  fn alpha_atlas_dist_to_tex_silice(&self, dist: &AtlasDist) -> TextureSlice {
    let blank_side = SideOffsets2D::new_all_same(ALPHA_BLANK_EDGE);
    let rect = dist.tex_rect(&self.alpha_atlas);

    TextureSlice { tex_id: TextureID::Alpha(dist.tex_id()), rect: rect.inner_rect(blank_side) }
  }

  pub(super) fn cache_scale(&self, size: &Size, matrix: &Transform) -> f32 {
    let Transform { m11, m12, m21, m22, .. } = matrix;
    let scale = (m11.abs() + m12.abs()).max(m21.abs() + m22.abs());
    let dis = size.width.max(size.height);
    if dis * scale < 32. {
      // If the path is too small, set a minimum tessellation size of 32 pixels.
      32. / dis
    } else {
      // 2 * BLANK_EDGE is the blank edge for each side.
      let max_size = size_shrink_blank(self.alpha_atlas.max_size()).to_f32();
      let max_scale = (max_size.width / size.width).min(max_size.width / size.height);
      scale.min(max_scale)
    }
  }

  fn fill_tess(
    path: &Path, ts: &Transform, slice_size: &DeviceSize, buffer: &mut VertexBuffers<()>,
  ) -> Range<u32> {
    let start = buffer.indices.len() as u32;
    let path_size = path.bounds().size;
    let slice_size = slice_size.to_f32();
    let scale = (slice_size.width / path_size.width).max(slice_size.height / path_size.height);
    path.tessellate(TOLERANCE / scale, buffer, |pos| {
      let pos = ts.transform_point(pos);
      Vertex::new([pos.x, pos.y], ())
    });
    start..buffer.indices.len() as u32
  }

  pub(crate) fn draw_alpha_textures<G: GPUBackendImpl<Texture = T>>(&mut self, gpu_impl: &mut G)
  where
    T: Texture<Host = G>,
  {
    if self.fill_task.is_empty() {
      return;
    }

    if !self.need_clear_areas.is_empty() {
      let tex = self.alpha_atlas.get_texture_mut(0);
      tex.clear_areas(&self.need_clear_areas, gpu_impl);
      self.need_clear_areas.clear();
    }

    self.fill_task.sort_by(|a, b| {
      let a_clip = a.clip_rect.is_some();
      let b_clip = b.clip_rect.is_some();
      if a_clip == b_clip {
        a.slice.tex_id.cmp(&b.slice.tex_id)
      } else if a_clip {
        Ordering::Less
      } else {
        Ordering::Greater
      }
    });

    let mut draw_indices = Vec::with_capacity(self.fill_task.len());
    if self.fill_task.len() < PAR_CHUNKS_SIZE {
      for f in self.fill_task.iter() {
        let FillTask { slice, path, clip_rect, transform: ts } = f;
        let rg = Self::fill_tess(path, ts, &slice.rect.size, &mut self.fill_task_buffers);
        draw_indices.push((slice.tex_id, rg, clip_rect));
      }
    } else {
      let mut tasks = Vec::with_capacity(self.fill_task.len());
      for f in self.fill_task.iter() {
        let FillTask { slice, path, clip_rect, transform: ts } = f;
        tasks.push((slice, ts, path, clip_rect));
      }

      let par_tess_res = tasks
        .par_chunks(PAR_CHUNKS_SIZE)
        .map(|tasks| {
          let mut buffer = VertexBuffers::default();
          let mut indices = Vec::with_capacity(tasks.len());
          for (slice, ts, path, clip_rect) in tasks.iter() {
            let rg = Self::fill_tess(path, ts, &slice.rect.size, &mut buffer);
            indices.push((slice.tex_id, rg, *clip_rect));
          }
          (indices, buffer)
        })
        .collect::<Vec<_>>();

      par_tess_res
        .into_iter()
        .for_each(|(indices, buffer)| {
          let offset = self.fill_task_buffers.indices.len() as u32;
          draw_indices.extend(indices.into_iter().map(|(id, mut rg, clip)| {
            rg.start += offset;
            rg.end += offset;
            (id, rg, clip)
          }));
          extend_buffer(&mut self.fill_task_buffers, buffer);
        })
    };

    gpu_impl.load_alpha_vertices(&self.fill_task_buffers);

    let mut idx = 0;
    loop {
      if idx >= draw_indices.len() {
        break;
      }

      let (tex_id, rg, Some(clip_rect)) = &draw_indices[idx] else {
        break;
      };
      let texture = id_to_texture_mut!(self, *tex_id);
      gpu_impl.draw_alpha_triangles_with_scissor(rg, texture, *clip_rect);
      idx += 1;
    }

    loop {
      if idx >= draw_indices.len() {
        break;
      }
      let (tex_id, rg, None) = &draw_indices[idx] else {
        unreachable!();
      };
      let next = draw_indices[idx..]
        .iter()
        .position(|(next, _, _)| tex_id != next);

      let indices = if let Some(mut next) = next {
        next += idx;
        idx = next;
        let (_, end, _) = &draw_indices[next];
        rg.start..end.start
      } else {
        idx = draw_indices.len();
        rg.start..self.fill_task_buffers.indices.len() as u32
      };

      let texture = id_to_texture_mut!(self, *tex_id);
      gpu_impl.draw_alpha_triangles(&indices, texture);
    }

    self.fill_task.clear();
    self.fill_task_buffers.vertices.clear();
    self.fill_task_buffers.indices.clear();
  }

  pub(crate) fn end_frame(&mut self) {
    self.alpha_atlas.end_frame_with(|rect| {
      self.need_clear_areas.push(rect);
    });
    self.rgba_atlas.end_frame();
    self.target_atlas.end_frame();
  }
}

fn extend_buffer<V>(dist: &mut VertexBuffers<V>, from: VertexBuffers<V>) {
  if dist.vertices.is_empty() {
    dist.vertices.extend(from.vertices);
    dist.indices.extend(from.indices);
  } else {
    let offset = dist.vertices.len() as u32;
    dist
      .indices
      .extend(from.indices.into_iter().map(|i| offset + i));
    dist.vertices.extend(from.vertices);
  }
}

const ALPHA_BLANK_EDGE: i32 = 2;

fn size_expand_blank(mut size: DeviceSize) -> DeviceSize {
  size.width += ALPHA_BLANK_EDGE * 2;
  size.height += ALPHA_BLANK_EDGE * 2;
  size
}

fn size_shrink_blank(mut size: DeviceSize) -> DeviceSize {
  size.width -= ALPHA_BLANK_EDGE * 2;
  size.height -= ALPHA_BLANK_EDGE * 2;
  size
}

impl TextureSlice {
  pub fn expand_for_paste(mut self) -> TextureSlice {
    const EXPANDED_EDGE: i32 = 1;
    let blank_side = SideOffsets2D::new_all_same(EXPANDED_EDGE);
    self.rect = self.rect.outer_rect(blank_side);
    self
  }
}

#[cfg(feature = "wgpu")]
#[cfg(test)]
pub mod tests {
  use std::borrow::Cow;

  use futures::executor::block_on;
  use ribir_geom::*;
  use ribir_painter::Color;

  use super::*;
  use crate::{WgpuImpl, WgpuTexture};

  pub fn color_image(color: Color, width: u32, height: u32) -> Resource<PixelImage> {
    let data = std::iter::repeat(color.into_components())
      .take(width as usize * height as usize)
      .flatten()
      .collect::<Vec<_>>();

    let img = PixelImage::new(Cow::Owned(data), width, height, ColorFormat::Rgba8);
    Resource::new(img)
  }

  #[test]
  fn smoke_store_image() {
    let mut wgpu = block_on(WgpuImpl::headless());
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
    mgr: &TexturesMgr<WgpuTexture>, rect: &TextureSlice, wgpu: &mut WgpuImpl, color: Color,
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
        .all(|c| c == color.into_components())
    );
  }

  #[test]
  fn transform_path_share_cache() {
    let mut wgpu = block_on(WgpuImpl::headless());
    let mut mgr = TexturesMgr::<WgpuTexture>::new(&mut wgpu);

    let p = Resource::new(Path::rect(&rect(0., 0., 300., 300.)));
    let p = PaintPath::Share(p.clone());

    let viewport = rect(0, 0, 1024, 1024);
    let (slice1, ts1) = mgr.store_alpha_path(&p, &Transform::scale(2., 2.), &viewport, &mut wgpu);

    let (slice2, ts2) =
      mgr.store_alpha_path(&p, &Transform::translation(100., 100.), &viewport, &mut wgpu);
    assert_eq!(slice1, slice2);

    assert_eq!(ts1, Transform::new(1., 0., 0., 1., -2., -2.));
    assert_eq!(ts2, Transform::new(0.5, 0., 0., 0.5, 99., 99.));
  }

  #[test]
  fn fix_resource_address_conflict() {
    // because the next resource may allocate at same address of a deallocated
    // address.

    let mut wgpu = block_on(WgpuImpl::headless());
    let mut mgr = TexturesMgr::<WgpuTexture>::new(&mut wgpu);
    {
      let red_img = color_image(Color::RED, 32, 32);
      mgr.store_image(&red_img, &mut wgpu);
    }

    for _ in 0..10 {
      mgr.end_frame();
      let red_img = color_image(Color::RED, 32, 32).into_any();
      assert!(mgr.rgba_atlas.get(&red_img, 1.).is_none());
    }
  }
}
