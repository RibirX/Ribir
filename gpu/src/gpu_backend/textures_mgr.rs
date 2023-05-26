use super::atlas::Atlas;
use super::Texture;
use crate::add_draw_rect_vertices;
use crate::{gpu_backend::atlas::ATLAS_MAX_ITEM, GPUBackendImpl};
use guillotiere::{euclid::SideOffsets2D, AllocId};
use rayon::{prelude::ParallelIterator, slice::ParallelSlice};
use ribir_algo::{FrameCache, ShareResource};
use ribir_geom::{rect_corners, DevicePoint, DeviceRect, DeviceSize, Point, Transform};
use ribir_painter::{
  image::ColorFormat, AntiAliasing, PaintPath, Path, PathSegment, PixelImage, Vertex, VertexBuffers,
};
use slab::Slab;
use std::hash::{Hash, Hasher};
use std::{cmp::Ordering, ops::Range};
const TOLERANCE: f32 = 0.1_f32;
const PAR_CHUNKS_SIZE: usize = 64;

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Copy)]
pub(super) enum TextureID {
  AlphaAtlas,
  RgbaAtlas,
  Extra(u32),
}

pub(super) struct TexturesMgr<T: Texture> {
  // todo: pair of alpha_atlas and path_cache
  alpha_atlas: Atlas<T>,
  rgba_atlas: Atlas<T>,
  extra_textures: Slab<T>,
  path_cache: FrameCache<PathKey, (f32, TextureDist)>,
  resource_cache: FrameCache<*const (), TextureDist>,
  fill_task: Vec<FillTask>,
  fill_task_buffers: VertexBuffers<f32>,
}

struct FillTask {
  slice: TextureSlice,
  path: Path,
  // transform to construct vertex
  ts: Transform,
  clip_rect: Option<DeviceRect>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct TextureSlice {
  pub(super) tex_id: TextureID,
  pub(super) rect: DeviceRect,
}

macro_rules! id_to_texture_mut {
  ($mgr: ident, $id: expr) => {
    match $id {
      TextureID::AlphaAtlas => &mut $mgr.alpha_atlas.texture,
      TextureID::RgbaAtlas => &mut $mgr.rgba_atlas.texture,
      TextureID::Extra(id) => &mut $mgr.extra_textures[id as usize],
    }
  };
}

macro_rules! id_to_texture {
  ($mgr: ident, $id: expr) => {
    match $id {
      TextureID::AlphaAtlas => &$mgr.alpha_atlas.texture,
      TextureID::RgbaAtlas => &$mgr.rgba_atlas.texture,
      TextureID::Extra(id) => &$mgr.extra_textures[id as usize],
    }
  };
}

impl<T: Texture> TexturesMgr<T>
where
  T::Host: GPUBackendImpl<Texture = T>,
{
  pub(super) fn new(gpu_impl: &mut T::Host, anti_aliasing: AntiAliasing) -> Self {
    Self {
      alpha_atlas: Atlas::new(ColorFormat::Alpha8, anti_aliasing, gpu_impl),
      rgba_atlas: Atlas::new(ColorFormat::Rgba8, anti_aliasing, gpu_impl),
      extra_textures: <_>::default(),
      path_cache: <_>::default(),
      resource_cache: <_>::default(),
      fill_task: <_>::default(),
      fill_task_buffers: <_>::default(),
    }
  }

  pub(super) fn set_anti_aliasing(&mut self, anti_aliasing: AntiAliasing, host: &mut T::Host) {
    let alpha_tex = self.texture_mut(TextureID::AlphaAtlas);
    alpha_tex.set_anti_aliasing(anti_aliasing, host);
    self.alpha_atlas.clear();
  }

  /// Store an alpha path in texture and return the texture and a transform that
  /// can transform the mask to viewport
  pub(super) fn store_alpha_path(
    &mut self,
    path: Path,
    transform: &Transform,
    gpu_impl: &mut T::Host,
  ) -> (TextureSlice, Transform) {
    fn cache_transform(path: &Path, cache_scale: f32, tex_slice: &TextureSlice) -> Transform {
      let scale_bounds = path.bounds().scale(cache_scale, cache_scale).round_out();
      Transform::scale(cache_scale, cache_scale)
        .then_translate(tex_slice.rect.origin.to_f32().cast_unit() - scale_bounds.origin)
    }

    fn cache_to_view(cache_ts: &Transform, path_ts: &Transform) -> Transform {
      cache_ts.inverse().unwrap().then(path_ts)
    }

    let prefer_scale: f32 = transform.m11.max(transform.m22);
    let key = PathKey::from_path(path);

    let mut slice_ts = None;

    if let Some((scale, dist)) = self.path_cache.get(&key) {
      if *scale < prefer_scale {
        // we will add a larger path cache later.
        let (_, dist) = self.path_cache.pop(&key).unwrap();
        match dist {
          TextureDist::Atlas { alloc_id, .. } => {
            self.alpha_atlas.deallocate(alloc_id);
          }
          TextureDist::Extra(id) => {
            self.extra_textures.remove(id as usize);
          }
        };
      } else {
        let slice = dist.as_tex_slice(&self.extra_textures).cut_blank_edge();
        let cache_ts = cache_transform(key.path(), *scale, &slice);
        slice_ts = Some((slice, cache_ts));
      }
    }
    let (slice, cache_ts) = slice_ts.unwrap_or_else(|| {
      let anti_aliasing = self.anti_aliasing();
      let path = key.path().clone();
      let scale_bounds = path.bounds().scale(prefer_scale, prefer_scale).round_out();
      let prefer_cache_size = path_add_edges(scale_bounds.size.to_i32().cast_unit());
      let tex_dist = self.inner_alloc(
        ColorFormat::Alpha8,
        anti_aliasing,
        prefer_cache_size,
        gpu_impl,
      );
      let slice = tex_dist.as_tex_slice(&self.extra_textures);
      let cut_blank_slice = slice.cut_blank_edge();
      let ts = cache_transform(&path, prefer_scale, &cut_blank_slice);

      self
        .fill_task
        .push(FillTask { slice, path, ts, clip_rect: None });
      self.path_cache.put(key, (prefer_scale, tex_dist));

      (cut_blank_slice, ts)
    });

    (slice, cache_to_view(&cache_ts, transform))
  }

  pub(super) fn store_clipped_path(
    &mut self,
    clip_view: DeviceRect,
    path: PaintPath,
    gpu_impl: &mut T::Host,
  ) -> (TextureSlice, Transform) {
    let alloc_size: DeviceSize = path_add_edges(clip_view.size);

    let anti_aliasing = self.anti_aliasing();
    let tex_dist = self.inner_alloc(ColorFormat::Alpha8, anti_aliasing, alloc_size, gpu_impl);

    let slice = tex_dist.as_tex_slice(&self.extra_textures);
    let path_slice = slice.cut_blank_edge();

    let ts = path
      .transform
      .then_translate(path_slice.rect.origin.to_f32().cast_unit() - path.paint_bounds.origin);

    let task = FillTask {
      slice,
      ts,
      path: path.path.clone(),
      clip_rect: Some(slice.rect),
    };
    self.fill_task.push(task);
    self.path_cache.push(
      PathKey::from_path_with_clip(path, clip_view),
      (1., tex_dist),
    );

    (slice, ts)
  }

  pub(super) fn store_image(
    &mut self,
    img: &ShareResource<PixelImage>,
    gpu_impl: &mut T::Host,
  ) -> TextureSlice {
    if let Some(dist) = self.resource_cache.get(&ShareResource::as_ptr(img)) {
      return dist.as_tex_slice(&self.extra_textures);
    }

    let size = DeviceSize::new(img.width() as i32, img.height() as i32);
    let tex_dist = self.inner_alloc(img.color_format(), AntiAliasing::None, size, gpu_impl);
    let tex_rect = tex_dist.as_tex_slice(&self.extra_textures);

    let texture = self.texture_mut(tex_rect.tex_id);
    texture.write_data(&tex_rect.rect, img.pixel_bytes(), gpu_impl);

    self
      .resource_cache
      .put(ShareResource::as_ptr(img), tex_dist);

    tex_rect
  }

  fn inner_alloc(
    &mut self,
    format: ColorFormat,
    anti_aliasing: AntiAliasing,
    size: DeviceSize,
    gpu_impl: &mut T::Host,
  ) -> TextureDist {
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

    let texture = gpu_impl.new_texture(size, anti_aliasing, format);
    let id = self.extra_textures.insert(texture);
    TextureDist::Extra(id as u32)
  }

  pub(super) fn texture_mut(&mut self, tex_id: TextureID) -> &mut T {
    id_to_texture_mut!(self, tex_id)
  }

  pub(super) fn texture(&self, tex_id: TextureID) -> &T { id_to_texture!(self, tex_id) }

  fn fill_tess(
    path: &Path,
    ts: &Transform,
    tex_size: DeviceSize,
    slice_bounds: &DeviceRect,
    buffer: &mut VertexBuffers<f32>,
  ) -> Range<u32> {
    let start = buffer.indices.len() as u32;

    let rect = rect_corners(&slice_bounds.to_f32().cast_unit());
    add_draw_rect_vertices(rect, tex_size, 0., buffer);

    let tex_width = tex_size.width as f32;
    let tex_height = tex_size.height as f32;

    let scale = ts.m11.max(ts.m22);

    path.tessellate(TOLERANCE / scale, buffer, |pos| {
      let pos = ts.transform_point(pos);
      Vertex::new([pos.x / tex_width, pos.y / tex_height], 1.)
    });
    start..buffer.indices.len() as u32
  }

  pub(crate) fn submit<G: GPUBackendImpl<Texture = T>>(&mut self, gpu_impl: &mut G) {
    if self.fill_task.is_empty() {
      return;
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
        let FillTask { slice, path, clip_rect, ts } = f;
        let texture = id_to_texture!(self, slice.tex_id);
        let rg = Self::fill_tess(
          path,
          ts,
          texture.size(),
          &slice.rect,
          &mut self.fill_task_buffers,
        );
        draw_indices.push((slice.tex_id, rg, clip_rect));
      }
    } else {
      let mut tasks = Vec::with_capacity(self.fill_task.len());
      for f in self.fill_task.iter() {
        let FillTask { slice, path, clip_rect, ts } = f;
        let texture = id_to_texture!(self, slice.tex_id);
        tasks.push((slice, ts, texture.size(), slice.rect, path, clip_rect));
      }

      let par_tess_res = tasks
        .par_chunks(PAR_CHUNKS_SIZE)
        .map(|tasks| {
          let mut buffer = VertexBuffers::default();
          let mut indices = Vec::with_capacity(tasks.len());
          for (slice, ts, tex_size, slice_bounds, path, clip_rect) in tasks.iter() {
            let rg = Self::fill_tess(path, ts, *tex_size, slice_bounds, &mut buffer);
            indices.push((slice.tex_id, rg, *clip_rect));
          }
          (indices, buffer)
        })
        .collect::<Vec<_>>();

      par_tess_res.into_iter().for_each(|(indices, buffer)| {
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

      let (tex_id,  rg,Some(clip_rect)) = &draw_indices[idx] else { break; };
      let texture = id_to_texture_mut!(self, *tex_id);
      gpu_impl.draw_alpha_triangles_with_scissor(rg, texture, *clip_rect);
      idx += 1;
    }

    loop {
      if idx >= draw_indices.len() {
        break;
      }
      let (tex_id,rg, None) = &draw_indices[idx] else { unreachable!(); };
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
    if self.rgba_atlas.hint_clear() {
      self.rgba_atlas.clear();
      self.resource_cache.clear();
    } else {
      self
        .resource_cache
        .end_frame("image atlas")
        .for_each(|dist| match dist {
          TextureDist::Atlas { alloc_id, .. } => {
            self.rgba_atlas.deallocate(alloc_id);
          }
          TextureDist::Extra(id) => {
            self.extra_textures.remove(id as usize);
          }
        });
    }
    if self.alpha_atlas.hint_clear() {
      self.alpha_atlas.clear();
      self.path_cache.clear();
    } else {
      self
        .path_cache
        .end_frame("path atlas")
        .for_each(|(_, dist)| match dist {
          TextureDist::Atlas { alloc_id, .. } => {
            self.alpha_atlas.deallocate(alloc_id);
          }
          TextureDist::Extra(id) => {
            self.extra_textures.remove(id as usize);
          }
        });
    }
  }

  fn anti_aliasing(&self) -> AntiAliasing { self.texture(TextureID::AlphaAtlas).anti_aliasing() }
}

#[derive(Debug, Clone, Copy)]
enum TextureDist {
  Atlas {
    alloc_id: AllocId,
    tex_rect: TextureSlice,
  },
  Extra(u32),
}

pub(crate) fn valid_cache_item(size: &DeviceSize) -> bool { size.lower_than(ATLAS_MAX_ITEM).any() }

fn extend_buffer<V>(dist: &mut VertexBuffers<V>, from: VertexBuffers<V>) {
  if dist.vertices.is_empty() {
    dist.vertices.extend(from.vertices.into_iter());
    dist.indices.extend(from.indices.into_iter());
  } else {
    let offset = dist.vertices.len() as u32;
    dist
      .indices
      .extend(from.indices.into_iter().map(|i| offset + i));
    dist.vertices.extend(from.vertices.into_iter());
  }
}

const BLANK_EDGE: i32 = 2;

fn path_add_edges(mut size: DeviceSize) -> DeviceSize {
  size.width += BLANK_EDGE * 2;
  size.height += BLANK_EDGE * 2;
  size
}

impl TextureSlice {
  pub fn cut_blank_edge(mut self) -> Self {
    let blank_side = SideOffsets2D::new_all_same(BLANK_EDGE);
    self.rect = self.rect.inner_rect(blank_side);
    self
  }
}

impl TextureDist {
  fn as_tex_slice<T: Texture>(&self, extra_textures: &Slab<T>) -> TextureSlice {
    match self {
      TextureDist::Atlas { tex_rect, .. } => *tex_rect,
      TextureDist::Extra(id) => TextureSlice {
        tex_id: TextureID::Extra(*id),
        rect: DeviceRect::from_size(extra_textures[*id as usize].size()),
      },
    }
  }
}

#[derive(Debug, Clone)]
enum PathKey {
  Path {
    path: Path,
    hash: u64,
  },
  PathWithClip {
    path: PaintPath,
    hash: u64,
    clip_rect: DeviceRect,
  },
}

fn pos_100_device(pos: Point) -> DevicePoint {
  Point::new(pos.x * 100., pos.y * 100.).to_i32().cast_unit()
}

fn path_inner_pos(pos: Point, path: &Path) -> DevicePoint {
  // Path pan to origin for comparison
  let pos = pos - path.bounds().origin;
  pos_100_device(pos.to_point())
}

fn path_hash(path: &Path, pos_adjust: impl Fn(Point) -> DevicePoint) -> u64 {
  let mut state = ahash::AHasher::default();

  for s in path.segments() {
    // core::mem::discriminant(&s).hash(&mut state);
    match s {
      PathSegment::MoveTo(to) | PathSegment::LineTo(to) => {
        pos_adjust(to).hash(&mut state);
      }
      PathSegment::QuadTo { ctrl, to } => {
        pos_adjust(ctrl).hash(&mut state);
        pos_adjust(to).hash(&mut state);
      }
      PathSegment::CubicTo { to, ctrl1, ctrl2 } => {
        pos_adjust(to).hash(&mut state);
        pos_adjust(ctrl1).hash(&mut state);
        pos_adjust(ctrl2).hash(&mut state);
      }
      PathSegment::Close(b) => b.hash(&mut state),
    };
  }

  state.finish()
}

fn path_eq(a: &Path, b: &Path, pos_adjust: impl Fn(Point, &Path) -> DevicePoint) -> bool {
  let a_adjust = |pos| pos_adjust(pos, a);
  let b_adjust = |pos| pos_adjust(pos, b);

  a.segments().zip(b.segments()).all(|(a, b)| match (a, b) {
    (PathSegment::MoveTo(a), PathSegment::MoveTo(b))
    | (PathSegment::LineTo(a), PathSegment::LineTo(b)) => a_adjust(a) == b_adjust(b),
    (PathSegment::QuadTo { ctrl, to }, PathSegment::QuadTo { ctrl: ctrl_b, to: to_b }) => {
      a_adjust(ctrl) == b_adjust(ctrl_b) && a_adjust(to) == b_adjust(to_b)
    }
    (
      PathSegment::CubicTo { to, ctrl1, ctrl2 },
      PathSegment::CubicTo {
        to: to_b,
        ctrl1: ctrl1_b,
        ctrl2: ctrl2_b,
      },
    ) => {
      a_adjust(to) == b_adjust(to_b)
        && a_adjust(ctrl1) == b_adjust(ctrl1_b)
        && a_adjust(ctrl2) == b_adjust(ctrl2_b)
    }
    (PathSegment::Close(a), PathSegment::Close(b)) => a == b,
    _ => false,
  })
}

impl PathKey {
  fn from_path(value: Path) -> Self {
    let hash = path_hash(&value, |pos| path_inner_pos(pos, &value));
    PathKey::Path { path: value, hash }
  }

  fn from_path_with_clip(path: PaintPath, clip_rect: DeviceRect) -> Self {
    let hash = path_hash(&path.path, pos_100_device);
    PathKey::PathWithClip { path, hash, clip_rect }
  }

  fn path(&self) -> &Path {
    match self {
      PathKey::Path { path, .. } => path,
      PathKey::PathWithClip { path, .. } => &path.path,
    }
  }
}

impl Hash for PathKey {
  fn hash<H: Hasher>(&self, state: &mut H) {
    match self {
      PathKey::Path { hash, .. } => hash.hash(state),
      PathKey::PathWithClip { hash, clip_rect, .. } => {
        clip_rect.hash(state);
        hash.hash(state)
      }
    }
  }
}

impl PartialEq for PathKey {
  fn eq(&self, other: &Self) -> bool {
    match (self, other) {
      (PathKey::Path { path: a, .. }, PathKey::Path { path: b, .. }) => {
        path_eq(a, b, path_inner_pos)
      }
      (
        PathKey::PathWithClip { path: a, clip_rect: view_rect_a, .. },
        PathKey::PathWithClip { path: b, clip_rect: view_rect_b, .. },
      ) => {
        view_rect_a == view_rect_b
          && a.transform == b.transform
          && path_eq(&a.path, &b.path, move |p, _| pos_100_device(p))
      }
      _ => false,
    }
  }
}

impl Eq for PathKey {}

pub fn prefer_cache_size(path: &Path, transform: &Transform) -> DeviceSize {
  let prefer_scale: f32 = transform.m11.max(transform.m22);
  let prefer_cache_size = path
    .bounds()
    .scale(prefer_scale, prefer_scale)
    .round_out()
    .size
    .to_i32()
    .cast_unit();
  path_add_edges(prefer_cache_size)
}

#[cfg(feature = "wgpu")]
#[cfg(test)]
pub mod tests {
  use super::*;
  use crate::{WgpuImpl, WgpuTexture};
  use futures::executor::block_on;
  use ribir_algo::ShareResource;
  use ribir_geom::*;
  use ribir_painter::{image::ColorFormat, AntiAliasing, Color, Path};
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
    let mut wgpu = block_on(WgpuImpl::headless());
    let mut mgr = TexturesMgr::new(&mut wgpu, AntiAliasing::None);

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
    mgr: &TexturesMgr<WgpuTexture>,
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
        .all(|c| c == color.into_components())
    );
  }

  #[test]
  fn transform_path_share_cache() {
    let mut wgpu = block_on(WgpuImpl::headless());
    let mut mgr = TexturesMgr::<WgpuTexture>::new(&mut wgpu, AntiAliasing::None);

    let path1 = Path::rect(&rect(0., 0., 300., 300.));
    let path2 = Path::rect(&rect(100., 100., 300., 300.));
    let ts = Transform::scale(2., 2.);

    let (slice1, ts1) = mgr.store_alpha_path(path1, &ts, &mut wgpu);
    let (slice2, ts2) = mgr.store_alpha_path(path2, &Transform::identity(), &mut wgpu);
    assert_eq!(slice1, slice2);

    assert_eq!(ts1, Transform::new(1., 0., 0., 1., -2., -2.));
    assert_eq!(ts2, Transform::new(0.5, 0., 0., 0.5, 99., 99.));
  }
}
