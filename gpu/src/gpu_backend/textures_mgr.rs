use super::atlas::{Atlas, AtlasHandle};
use super::Texture;
use crate::add_draw_rect_vertices;
use crate::{gpu_backend::atlas::ATLAS_MAX_ITEM, GPUBackendImpl};
use guillotiere::euclid::SideOffsets2D;
use rayon::{prelude::ParallelIterator, slice::ParallelSlice};
use ribir_algo::ShareResource;
use ribir_geom::{rect_corners, DevicePoint, DeviceRect, DeviceSize, Point, Transform};
use ribir_painter::{
  image::ColorFormat, AntiAliasing, PaintPath, Path, PathSegment, PixelImage, Vertex, VertexBuffers,
};
use std::hash::{Hash, Hasher};
use std::{cmp::Ordering, ops::Range};
const TOLERANCE: f32 = 0.1_f32;
const PAR_CHUNKS_SIZE: usize = 64;

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Copy)]
pub(super) enum TextureID {
  Alpha(usize),
  Rgba(usize),
}

pub(super) struct TexturesMgr<T: Texture> {
  alpha_atlas: Atlas<T, PathKey, f32>,
  rgba_atlas: Atlas<T, ShareResource<PixelImage>, ()>,
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
      TextureID::Alpha(id) => $mgr.alpha_atlas.get_texture_mut(id),
      TextureID::Rgba(id) => $mgr.rgba_atlas.get_texture_mut(id),
    }
  };
}

macro_rules! id_to_texture {
  ($mgr: ident, $id: expr) => {
    match $id {
      TextureID::Alpha(id) => $mgr.alpha_atlas.get_texture(id),
      TextureID::Rgba(id) => $mgr.rgba_atlas.get_texture(id),
    }
  };
}

fn get_transform_pref_scale(transform: &Transform) -> f32 {
  let Transform { m11, m12, m21, m22, .. } = *transform;
  (m11.abs() + m12.abs()).max(m21.abs() + m22.abs())
}

impl<T: Texture> TexturesMgr<T>
where
  T::Host: GPUBackendImpl<Texture = T>,
{
  pub(super) fn new(gpu_impl: &mut T::Host, anti_aliasing: AntiAliasing) -> Self {
    Self {
      alpha_atlas: Atlas::new("Alpha atlas", ColorFormat::Alpha8, anti_aliasing, gpu_impl),
      rgba_atlas: Atlas::new(
        "Rgba atlas",
        ColorFormat::Rgba8,
        AntiAliasing::None,
        gpu_impl,
      ),
      fill_task: <_>::default(),
      fill_task_buffers: <_>::default(),
    }
  }

  pub(super) fn set_anti_aliasing(&mut self, anti_aliasing: AntiAliasing, host: &mut T::Host) {
    self.alpha_atlas.set_anti_aliasing(anti_aliasing, host);
  }

  /// Store an alpha path in texture and return the texture and a transform that
  /// can transform the mask to viewport
  pub(super) fn store_alpha_path(
    &mut self,
    path: Path,
    transform: &Transform,
    gpu_impl: &mut T::Host,
  ) -> (TextureSlice, Transform) {
    fn cache_to_view_matrix(
      path: &Path,
      path_ts: &Transform,
      slice_origin: DevicePoint,
      cache_scale: f32,
    ) -> Transform {
      // scale origin to the cached path, and aligned the pixel, let the view get an
      // integer pixel mask as much as possible.
      let aligned_origin = (path.bounds().origin * cache_scale).round();

      // back to slice origin
      Transform::translation(-slice_origin.x as f32, -slice_origin.y as f32)
        // move to cache path axis.
        .then_translate(aligned_origin.to_vector().cast_unit())
        // scale back to path axis.
        .then_scale(1. / cache_scale, 1. / cache_scale)
        // apply path transform matrix to view.
        .then(path_ts)
    }

    let prefer_scale: f32 = get_transform_pref_scale(transform);
    let key = PathKey::from_path(path);

    if let Some(h) = self
      .alpha_atlas
      .get(&key)
      .filter(|h| h.attr >= prefer_scale)
      .copied()
    {
      let slice = alpha_tex_slice(&self.alpha_atlas, &h).cut_blank_edge();
      let matrix = cache_to_view_matrix(key.path(), transform, slice.rect.origin, h.attr);
      (slice, matrix)
    } else {
      let path = key.path().clone();
      let scale_bounds = path.bounds().scale(prefer_scale, prefer_scale);
      let prefer_cache_size = path_add_edges(scale_bounds.round_out().size.to_i32().cast_unit());
      let h = self
        .alpha_atlas
        .allocate(key, prefer_scale, prefer_cache_size, gpu_impl);
      let slice = alpha_tex_slice(&self.alpha_atlas, &h);
      let mask_slice = slice.cut_blank_edge();

      let matrix = cache_to_view_matrix(&path, transform, mask_slice.rect.origin, prefer_scale);

      let ts = Transform::scale(prefer_scale, prefer_scale)
        .then_translate(-scale_bounds.origin.to_vector().cast_unit())
        .then_translate(mask_slice.rect.origin.to_f32().to_vector().cast_unit());

      self
        .fill_task
        .push(FillTask { slice, path, ts, clip_rect: None });

      (mask_slice, matrix)
    }
  }

  pub(super) fn store_clipped_path(
    &mut self,
    clip_view: DeviceRect,
    path: PaintPath,
    gpu_impl: &mut T::Host,
  ) -> (TextureSlice, Transform) {
    let alloc_size: DeviceSize = path_add_edges(clip_view.size);
    let path_ts = path.transform;

    let key = PathKey::from_path_with_clip(path, clip_view);

    let slice = if let Some(h) = self.alpha_atlas.get(&key).copied() {
      alpha_tex_slice(&self.alpha_atlas, &h).cut_blank_edge()
    } else {
      let path = key.path().clone();
      let h = self.alpha_atlas.allocate(key, 1., alloc_size, gpu_impl);
      let slice = alpha_tex_slice(&self.alpha_atlas, &h);
      let no_blank_slice = slice.cut_blank_edge();
      let clip_rect = Some(slice.rect);
      let offset = (no_blank_slice.rect.origin - clip_view.origin)
        .to_f32()
        .cast_unit();
      let ts = path_ts.then_translate(offset);
      let task = FillTask { slice, ts, path, clip_rect };
      self.fill_task.push(task);
      no_blank_slice
    };

    let offset = (clip_view.origin - slice.rect.origin).to_f32();
    (slice, Transform::translation(offset.x, offset.y))
  }

  pub(super) fn store_image(
    &mut self,
    img: &ShareResource<PixelImage>,
    gpu_impl: &mut T::Host,
  ) -> TextureSlice {
    match img.color_format() {
      ColorFormat::Rgba8 => {
        if let Some(h) = self.rgba_atlas.get(img).copied() {
          rgba_tex_slice(&self.rgba_atlas, &h)
        } else {
          let size = DeviceSize::new(img.width() as i32, img.height() as i32);
          let h = self.rgba_atlas.allocate(img.clone(), (), size, gpu_impl);
          let slice = rgba_tex_slice(&self.rgba_atlas, &h);

          let texture = self.rgba_atlas.get_texture_mut(h.tex_id());
          texture.write_data(&slice.rect, img.pixel_bytes(), gpu_impl);
          slice
        }
      }
      ColorFormat::Alpha8 => todo!(),
    }
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

    let scale = get_transform_pref_scale(ts);

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
    self.alpha_atlas.end_frame();
    self.rgba_atlas.end_frame();
  }
}

fn alpha_tex_slice<T, K>(atlas: &Atlas<T, K, f32>, h: &AtlasHandle<f32>) -> TextureSlice
where
  T: Texture,
{
  TextureSlice {
    tex_id: TextureID::Alpha(h.tex_id()),
    rect: h.tex_rect(atlas),
  }
}

fn rgba_tex_slice<T, K>(atlas: &Atlas<T, K, ()>, h: &AtlasHandle<()>) -> TextureSlice
where
  T: Texture,
{
  TextureSlice {
    tex_id: TextureID::Rgba(h.tex_id()),
    rect: h.tex_rect(atlas),
  }
}

pub(crate) fn valid_cache_item(size: &DeviceSize) -> bool { size.lower_than(ATLAS_MAX_ITEM).any() }

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

const BLANK_EDGE: i32 = 2;

fn path_add_edges(mut size: DeviceSize) -> DeviceSize {
  size.width += BLANK_EDGE * 2;
  size.height += BLANK_EDGE * 2;
  size
}

impl TextureSlice {
  pub fn cut_blank_edge(mut self) -> TextureSlice {
    let blank_side = SideOffsets2D::new_all_same(BLANK_EDGE);
    self.rect = self.rect.inner_rect(blank_side);
    self
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
  let prefer_scale: f32 = get_transform_pref_scale(transform);
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

  #[test]
  fn store_clipped_path() {
    let mut wgpu = block_on(WgpuImpl::headless());
    let mut mgr = TexturesMgr::<WgpuTexture>::new(&mut wgpu, AntiAliasing::None);

    let path = PaintPath::new(
      Path::rect(&rect(20., 20., 300., 300.)),
      Transform::new(2., 0., 0., 2., -10., -10.),
    );
    let clip_view = ribir_geom::rect(10, 10, 100, 100);

    let (slice1, ts1) = mgr.store_clipped_path(clip_view, path.clone(), &mut wgpu);
    let (slice2, ts2) = mgr.store_clipped_path(clip_view, path, &mut wgpu);
    assert_eq!(slice1, slice2);
    assert_eq!(ts1, ts2);
    assert_eq!(slice1.rect, ribir_geom::rect(2, 2, 100, 100));
    assert_eq!(ts1, Transform::new(1., 0., 0., 1., 8., 8.));
  }

  #[test]
  fn fix_resource_address_conflict() {
    // because the next resource may allocate at same address of a deallocated
    // address.

    let mut wgpu = block_on(WgpuImpl::headless());
    let mut mgr = TexturesMgr::<WgpuTexture>::new(&mut wgpu, AntiAliasing::None);
    {
      let red_img = color_image(Color::RED, 32, 32);
      mgr.store_image(&red_img, &mut wgpu);
    }

    for _ in 0..10 {
      mgr.end_frame();
      let red_img = color_image(Color::RED, 32, 32);
      assert!(mgr.rgba_atlas.get(&red_img).is_none());
    }
  }
}
