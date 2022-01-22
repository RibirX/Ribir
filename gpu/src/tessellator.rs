use crate::{
  error::Error, ColorPrimitive, ColorRenderData, RenderData, Texture, TexturePrimitive,
  TextureRenderData, Vertex,
};
use algo::FrameCache;
use lyon_tessellation::{path::Path, *};
use painter::{
  Brush, DeviceRect, DeviceSize, PaintCommand, PaintPath, PathStyle, Rect, TileMode, Transform,
  Vector,
};
use std::{
  collections::VecDeque,
  hash::Hash,
  mem::{size_of, transmute},
  ops::RangeBounds,
};
use text::{
  font_db::ID,
  layout::{GlyphAt, LayoutConfig},
  shaper::{GlyphId, TextShaper},
};
mod atlas;
use atlas::TextureAtlas;

mod mem_texture;
mod texture_records;
use texture_records::TextureRecords;

const ATLAS_ID: usize = 0;
const TOLERANCE: f32 = 0.01;
const MAX_VERTEX_CAN_BATCH: usize = 1_000_000;
const TEXTURE_ID_FROM: usize = 1;

/// `Tessellator` use to generate triangles from
pub struct Tessellator {
  // texture atlas for pure color and image to draw.
  pub(crate) atlas: TextureAtlas,
  buffer: TessData,
  texture_records: TextureRecords,
  vertices_cache: FrameCache<VerticesKey<Path>, Box<VertexCache>>,
}

enum PrimitiveVec {
  Color(Vec<ColorPrimitive>),
  Texture(Vec<TexturePrimitive>),
}

#[derive(Default)]
struct TessData {
  vertices: Vec<Vertex>,
  indices: Vec<u32>,
  primitives: PrimitiveVec,
  buffer_list: VecDeque<CacheItem>,
  /// The max vertex can batch. It's not a strict number, it's unaffected if
  /// it's less than the count of vertex generate by one paint command, default
  /// value is [`MAX_VERTEX_CAN_BATCH`]!
  vertex_batch_limit: usize,
  /// The scaling difference of a same path need to retessellate.
  threshold: f32,
  shaper: TextShaper,
}

struct CacheItem {
  prim_id: u32,
  cache_ptr: *mut VertexCache,
}

#[derive(Debug, Clone)]
struct VerticesKey<P> {
  tolerance: f32,
  threshold: f32,
  style: PathStyle,
  path: PathKey<P>,
}

#[derive(Debug, Clone)]
enum PathKey<P> {
  Path(P),
  Glyph { glyph_id: GlyphId, face_id: ID },
}
#[derive(Default)]
struct VertexCache {
  vertices: Box<[[f32; 2]]>,
  indices: Box<[u32]>,
  indices_offset: Option<u32>,
}

impl Tessellator {
  /// Create a `Tessellator` with the init texture size and the maximum texture
  /// size. `threshold` is the scale difference of a path need to retessellate.
  #[inline]
  pub fn new(
    tex_init_size: DeviceSize,
    tex_max_size: DeviceSize,
    threshold: f32,
    shaper: TextShaper,
  ) -> Self {
    Self {
      atlas: TextureAtlas::new(tex_init_size, tex_max_size),
      buffer: TessData {
        vertex_batch_limit: MAX_VERTEX_CAN_BATCH,
        threshold,
        shaper,
        ..<_>::default()
      },
      vertices_cache: <_>::default(),
      texture_records: TextureRecords::new(TEXTURE_ID_FROM),
    }
  }

  /// The vertex count to trigger a gpu submit. It's not a strict number, may
  /// exceed this limit by one paint command
  pub fn set_vertex_batch_limit(&mut self, count: usize) { self.buffer.vertex_batch_limit = count; }

  pub fn tessellate<F>(&mut self, commands: &[PaintCommand], mut gpu_submit: F)
  where
    F: FnMut(RenderData),
  {
    if commands.is_empty() {
      return;
    }

    // Try store all image brush in atlas texture , if not success, deallocate the
    // sub texture which cache missed and then refill by auto grow mode.
    let any_brush_fill_fail = !self.try_fill_atlas(commands);
    if any_brush_fill_fail {
      self.atlas.end_frame();
    }

    let mut submitted = 0;
    loop {
      // generate buffer until failed or cannot batch.
      let count = self.buffer.generate_buffer(
        &commands[submitted..],
        &mut self.atlas,
        &mut self.vertices_cache,
      );

      match count {
        Ok(count) => {
          assert!(count > 0);
          while !self.buffer.buffer_list.is_empty() {
            unsafe { self.buffer.fill_vertices() };
            let atlas_tex = self.atlas.as_render_texture(ATLAS_ID);
            self.buffer.submit_vertices(&mut gpu_submit, atlas_tex);
            self.reset_vertex_cache_offset();
            self.atlas.gpu_synced();
          }
          submitted += count;
        }
        Err(Error::TextureSpaceLimit)
          if commands[submitted..]
            .iter()
            .filter(|cmd| matches!(cmd.brush, Brush::Image { .. }))
            .count()
            > 8 =>
        {
          // clear atlas if there is many image brush command
          self.atlas.clear()
        }
        _ => {
          // independent submit render data, if it's a large image or few image to draw.
          let cmd = &commands[submitted];
          self.image_brush_independent_submit(cmd, &mut gpu_submit);
          submitted += 1;
        }
      }

      if submitted >= commands.len() {
        break;
      }
    }

    self.end_frame()
  }

  fn reset_vertex_cache_offset(&mut self) {
    self
      .vertices_cache
      .values_mut()
      .for_each(|v| v.indices_offset = None)
  }

  fn end_frame(&mut self) {
    assert!(self.buffer.buffer_list.is_empty());
    // end frame to clear miss cache, atlas and vertexes clear before by itself.
    self.texture_records.end_frame();
    self.vertices_cache.end_frame("Vertices");
  }

  fn image_brush_independent_submit<F>(&mut self, cmd: &PaintCommand, mut gpu_submit: F)
  where
    F: FnMut(RenderData),
  {
    let (img, tile_mode) = if let Brush::Image { img, tile_mode } = &cmd.brush {
      (img, tile_mode)
    } else {
      unreachable!();
    };
    let size = img.size();
    let format = img.color_format();
    let mut img_data = None;
    let id = self.texture_records.get_id(&img).unwrap_or_else(|| {
      img_data = Some(img.pixel_bytes());
      self.texture_records.insert(img.clone())
    });

    let data = img_data.as_ref().map(|d| &**d);
    let texture = Texture { id, data, size, format };

    let primitive = Primitive::texture_prim(
      tile_mode,
      cmd.transform.clone(),
      &DeviceRect::from_size(img.size()),
      || cmd.box_rect_without_transform(),
    );
    let shaper = self.buffer.shaper.clone();
    let res = self.buffer.command_to_buffer(cmd, primitive, |key| {
      self
        .vertices_cache
        .get_or_insert_with(&key as &dyn KeySlice, || {
          Box::new(TessData::gen_triangles(&shaper, &key))
        })
        .as_mut() as *mut _
    });
    assert!(res);
    unsafe { self.buffer.fill_vertices() };
    self.buffer.submit_vertices(&mut gpu_submit, texture);
  }

  fn try_fill_atlas(&mut self, commands: &[PaintCommand]) -> bool {
    commands.iter().all(|cmd| {
      if let Brush::Image { img, .. } = &cmd.brush {
        self.atlas.store_image(img).is_ok()
      } else {
        true
      }
    })
  }
}

fn tesselate_path(path: &Path, style: PathStyle, tolerance: f32) -> VertexCache {
  match style {
    painter::PathStyle::Fill => fill_tess(path, tolerance),
    painter::PathStyle::Stroke(line_width) => stroke_tess(path, line_width, tolerance),
  }
}

fn stroke_tess(path: &Path, line_width: f32, tolerance: f32) -> VertexCache {
  let mut buffers = VertexBuffers::new();
  let mut stroke_tess = StrokeTessellator::default();
  stroke_tess
    .tessellate_path(
      path,
      &StrokeOptions::tolerance(tolerance).with_line_width(line_width),
      &mut BuffersBuilder::new(&mut buffers, move |v: StrokeVertex| v.position().to_array()),
    )
    .unwrap();

  VertexCache {
    vertices: buffers.vertices.into_boxed_slice(),
    indices: buffers.indices.into_boxed_slice(),
    indices_offset: None,
  }
}

fn fill_tess(path: &Path, tolerance: f32) -> VertexCache {
  let mut buffers = VertexBuffers::new();
  let mut fill_tess = FillTessellator::default();
  fill_tess
    .tessellate_path(
      path,
      &FillOptions::tolerance(tolerance),
      &mut BuffersBuilder::new(&mut buffers, move |v: FillVertex| v.position().to_array()),
    )
    .unwrap();

  VertexCache {
    vertices: buffers.vertices.into_boxed_slice(),
    indices: buffers.indices.into_boxed_slice(),
    indices_offset: None,
  }
}

impl TessData {
  /// Generate commands buffer until failed or there is a different type paint
  /// command occur.
  ///
  /// Return how many commands processed.
  fn generate_buffer(
    &mut self,
    commands: &[PaintCommand],
    atlas: &mut TextureAtlas,
    vertices_cache: &mut FrameCache<VerticesKey<Path>, Box<VertexCache>>,
  ) -> Result<usize, Error> {
    let mut uninit_vertices = vertices_cache.as_uninit_map();

    let mut count = 0;
    for cmd in commands.iter() {
      let primitive = match Primitive::from_command(cmd, atlas) {
        Ok(p) => p,
        Err(e) if count == 0 => return Err(e),
        Err(_) => break,
      };

      let res = self.command_to_buffer(cmd, primitive, |key| {
        uninit_vertices.get_or_delay_init::<dyn KeySlice>(key)
      });
      if !res {
        break;
      }
      count += 1;
    }

    unsafe {
      uninit_vertices.par_init_with(|key| Self::gen_triangles(&self.shaper, &key));
    }

    Ok(count)
  }

  fn command_to_buffer<'a, F>(
    &mut self,
    cmd: &'a PaintCommand,
    primitive: Primitive,
    mut cache: F,
  ) -> bool
  where
    F: FnMut(VerticesKey<&'a Path>) -> *mut VertexCache,
  {
    // Check before process glyphs.
    if !self.primitives.can_push(&primitive) {
      return false;
    }

    let PaintCommand { path, transform, .. } = cmd;
    let style = cmd.path_style;
    let scale = transform.m11.max(transform.m22).max(f32::EPSILON);
    let threshold = self.threshold;
    match path {
      PaintPath::Path(path) => {
        let path = PathKey::Path(path);
        let tolerance = TOLERANCE / scale;
        let key = VerticesKey { tolerance, threshold, style, path };
        let cache_ptr = cache(key);
        let prim_id = self.primitives.push(primitive).unwrap();
        self.buffer_list.push_back(CacheItem { prim_id, cache_ptr });
      }
      &PaintPath::Text {
        font_size,
        letter_space,
        line_height,
        ref text,
        ref font_face,
        ..
      } if font_size > f32::EPSILON => {
        let face_ids = self.shaper.font_db_mut().select_all_match(&font_face);
        let glyphs = self.shaper.shape_text(&text, &face_ids);
        // todo: layout is a higher level work should not work here, maybe should work
        // in texts widget layout.
        // paint should directly shape and draw text, not care about bidi reordering,
        // text wrap or break.
        let cfg = LayoutConfig {
          font_size,
          line_height,
          letter_space,
          h_align: None,
          v_align: None,
        };

        let tolerance = TOLERANCE / (font_size * scale);
        text::layout::layout_text(&text, &glyphs, &cfg, None).for_each(
          |GlyphAt { glyph_id, face_id, x, y }| {
            let path = PathKey::<&Path>::Glyph { face_id, glyph_id };
            let key = VerticesKey { tolerance, threshold, style, path };
            let cache_ptr = cache(key);
            let t = transform
              .pre_scale(font_size, font_size)
              .pre_translate(Vector::new(x, y));
            let mut p = primitive.clone();
            p.set_transform(t);

            let prim_id = self.primitives.push(p).unwrap();
            self.buffer_list.push_back(CacheItem { prim_id, cache_ptr });
          },
        );
      }
      _ => {}
    };
    true
  }

  /// submit vertexes and return how many primitives consumed.
  fn submit_vertices<F: FnMut(RenderData)>(&mut self, f: &mut F, texture: Texture) {
    if self.indices.is_empty() {
      return;
    }

    let render_data = match &self.primitives {
      PrimitiveVec::Color(p) => RenderData::Color(ColorRenderData {
        vertices: &self.vertices,
        indices: &self.indices,
        primitives: p.as_slice(),
      }),
      PrimitiveVec::Texture(p) => RenderData::Image(TextureRenderData {
        vertices: &self.vertices,
        indices: &self.indices,
        primitives: p.as_slice(),
        texture,
      }),
    };
    f(render_data);

    // if buffer list is not empty, we retain the last primitive, it's maybe used by
    // others in buffer list.
    if !self.buffer_list.is_empty() {
      let drain_end = self.vertices.last().expect("must have").prim_id;
      if drain_end > 0 {
        self.primitives.drain(..drain_end as usize);
        self.buffer_list.iter_mut().for_each(|c| {
          c.prim_id -= drain_end;
        });
      }
    } else {
      self.primitives.clear();
    }

    self.indices.clear();
    self.vertices.clear();
  }

  /// Generate vertices from the buffer
  ///
  /// Caller also should guarantee the cache pointer is valid.
  unsafe fn fill_vertices(&mut self) {
    while let Some(CacheItem { prim_id, cache_ptr }) = self.buffer_list.pop_front() {
      let cache = &mut *cache_ptr;
      if let Some(offset) = cache.indices_offset {
        self
          .indices
          .extend(cache.indices.iter().map(|i| i + offset))
      } else {
        cache.indices_offset = Some(self.vertices.len() as u32);
        self.vertices.extend(
          cache
            .vertices
            .iter()
            .map(|pos| Vertex { pixel_coords: *pos, prim_id }),
        );
        self.indices.extend_from_slice(&cache.indices);
      }

      if self.indices.len() > self.vertex_batch_limit {
        break;
      }
    }
  }

  fn gen_triangles(shaper: &TextShaper, key: &VerticesKey<&Path>) -> VertexCache {
    let &VerticesKey { tolerance, style, .. } = key;
    match key.path {
      PathKey::Path(path) => tesselate_path(path, style, tolerance),
      PathKey::Glyph { glyph_id, face_id } => {
        let face = {
          let mut font_db = shaper.font_db_mut();
          font_db
            .face_data_or_insert(face_id)
            .expect("Font face not exist!")
            .clone()
        };

        if let Some(path) = face.outline_glyph(glyph_id) {
          tesselate_path(&path, style, tolerance)
        } else {
          //todo, image or svg fallback?
          VertexCache::default()
        }
      }
    }
  }
}

#[derive(Clone)]
enum Primitive {
  Color(ColorPrimitive),
  Texture(TexturePrimitive),
}

impl Primitive {
  fn from_command(cmd: &PaintCommand, atlas: &mut TextureAtlas) -> Result<Self, Error> {
    match &cmd.brush {
      Brush::Color(color) => {
        let c = ColorPrimitive {
          color: color.clone().into_arrays(),
          transform: cmd.transform.clone().to_arrays(),
        };
        Ok(Primitive::Color(c))
      }
      Brush::Image { img, tile_mode } => {
        let alloc = atlas.store_image(img)?;
        let t = Self::texture_prim(
          tile_mode,
          cmd.transform.clone(),
          &alloc.rectangle.to_u32().to_rect().cast_unit(),
          || cmd.box_rect_without_transform(),
        );
        Ok(t)
      }
      Brush::Gradient => todo!(),
    }
  }

  fn texture_prim<F: FnOnce() -> Rect>(
    tile_mode: &TileMode,
    transform: Transform,
    texture_rect: &DeviceRect,
    path_box: F,
  ) -> Self {
    let (mut x_base, mut y_base) = texture_rect.size.to_f32().to_tuple();
    if tile_mode.is_cover_mode() {
      let box_rect = path_box();
      if tile_mode.contains(TileMode::COVER_X) {
        x_base = box_rect.width();
      }
      if tile_mode.contains(TileMode::COVER_Y) {
        y_base = box_rect.height()
      }
    }
    let t = TexturePrimitive {
      tex_offset: texture_rect.min().to_array(),
      factor: [1. / x_base, 1. / y_base],
      transform: transform.to_arrays(),
    };
    Primitive::Texture(t)
  }

  fn set_transform(&mut self, t: Transform) {
    match self {
      Primitive::Color(c) => c.transform = t.clone().to_arrays(),
      Primitive::Texture(tex) => tex.transform = t.clone().to_arrays(),
    }
  }
}

impl PrimitiveVec {
  fn convert_to_color_primitives(&mut self) -> Option<&mut Vec<ColorPrimitive>> {
    match self {
      PrimitiveVec::Color(c) => Some(c),
      PrimitiveVec::Texture(t) if t.is_empty() => {
        assert_eq!(size_of::<ColorPrimitive>(), size_of::<TexturePrimitive>());

        let vec = std::mem::take(t);
        *self = PrimitiveVec::Color(unsafe { transmute(vec) });
        match self {
          PrimitiveVec::Color(c) => Some(c),
          PrimitiveVec::Texture(_) => unreachable!(),
        }
      }
      _ => None,
    }
  }

  fn convert_to_texture_primitives(&mut self) -> Option<&mut Vec<TexturePrimitive>> {
    match self {
      PrimitiveVec::Color(c) if c.is_empty() => {
        assert_eq!(size_of::<ColorPrimitive>(), size_of::<TexturePrimitive>());

        let vec = std::mem::take(c);
        *self = PrimitiveVec::Texture(unsafe { transmute(vec) });
        match self {
          PrimitiveVec::Color(_) => unreachable!(),
          PrimitiveVec::Texture(t) => Some(t),
        }
      }
      PrimitiveVec::Texture(t) => Some(t),
      _ => None,
    }
  }

  fn drain<R>(&mut self, range: R)
  where
    R: RangeBounds<usize>,
  {
    match self {
      PrimitiveVec::Color(v) => {
        v.drain(range);
      }
      PrimitiveVec::Texture(v) => {
        v.drain(range);
      }
    };
  }

  // return the primitive index if it's can batch with existed primitives in
  // buffer.
  fn push(&mut self, prim: Primitive) -> Option<u32> {
    match prim {
      Primitive::Color(c) => {
        let primitives = self.convert_to_color_primitives()?;
        if primitives.last() != Some(&c) {
          primitives.push(c);
        }
        Some(primitives.len() as u32 - 1)
      }
      Primitive::Texture(t) => {
        let primitives = self.convert_to_texture_primitives()?;
        if primitives.last() != Some(&t) {
          primitives.push(t);
        }
        Some(primitives.len() as u32 - 1)
      }
    }
  }

  fn clear(&mut self) {
    match self {
      PrimitiveVec::Color(c) => c.clear(),
      PrimitiveVec::Texture(t) => t.clear(),
    }
  }

  fn can_push(&self, prim: &Primitive) -> bool {
    match self {
      PrimitiveVec::Color(c) => c.is_empty() || matches!(prim, Primitive::Color(_)),
      PrimitiveVec::Texture(t) => t.is_empty() || matches!(prim, Primitive::Texture(_)),
    }
  }
}

impl Default for PrimitiveVec {
  fn default() -> Self { PrimitiveVec::Color(vec![]) }
}

// trait implement for vertices cache

fn threshold_hash(value: f32, threshold: f32) -> u32 {
  let precisest = 1. / (threshold.max(f32::EPSILON));
  (value * precisest) as u32
}

use std::borrow::Borrow;

// todo: a more robust and also fast way to implement hash and eq for path.
enum Verb {
  _LineTo,
  _QuadraticTo,
  _CubicTo,
  _Begin,
  _Close,
  _End,
}
type LyonPoint = lyon_tessellation::geom::Point<f32>;
struct ShadowPath {
  points: Box<[LyonPoint]>,
  verbs: Box<[Verb]>,
  _num_attributes: usize,
}

fn as_bytes<T>(t: &[T]) -> &[u8] {
  let len = t.len() * size_of::<T>();
  unsafe { std::slice::from_raw_parts(t.as_ptr() as *const u8, len) }
}

impl Hash for VerticesKey<Path> {
  #[inline]
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) { (self as &dyn KeySlice).hash(state) }
}
impl PartialEq for VerticesKey<Path> {
  #[inline]
  fn eq(&self, other: &Self) -> bool { self as &dyn KeySlice == other as &dyn KeySlice }
}

impl Eq for VerticesKey<Path> {}

trait KeySlice {
  fn threshold(&self) -> f32;
  fn tolerance(&self) -> f32;
  fn style(&self) -> PathStyle;
  fn path(&self) -> PathKey<&Path>;
  fn to_key(&self) -> VerticesKey<Path>;
}

impl KeySlice for VerticesKey<Path> {
  fn threshold(&self) -> f32 { self.threshold }

  fn tolerance(&self) -> f32 { self.tolerance }

  fn style(&self) -> PathStyle { self.style }

  fn path(&self) -> PathKey<&Path> {
    match &self.path {
      PathKey::Path(path) => PathKey::Path(path),
      &PathKey::Glyph { glyph_id, face_id } => PathKey::Glyph { glyph_id, face_id },
    }
  }

  fn to_key(&self) -> VerticesKey<Path> { self.clone() }
}

impl<'a> KeySlice for VerticesKey<&'a Path> {
  #[inline]
  fn threshold(&self) -> f32 { self.threshold }

  #[inline]
  fn tolerance(&self) -> f32 { self.tolerance }

  #[inline]
  fn style(&self) -> PathStyle { self.style }

  #[inline]
  fn path(&self) -> PathKey<&Path> { self.path.clone() }

  fn to_key(&self) -> VerticesKey<Path> {
    VerticesKey {
      tolerance: self.tolerance,
      threshold: self.threshold,
      style: self.style,
      path: match self.path {
        PathKey::Path(path) => PathKey::Path(path.clone()),
        PathKey::Glyph { glyph_id, face_id } => PathKey::Glyph { glyph_id, face_id },
      },
    }
  }
}

impl<'a> Borrow<dyn KeySlice + 'a> for VerticesKey<Path> {
  fn borrow(&self) -> &(dyn KeySlice + 'a) { self }
}

impl<'a> Borrow<dyn KeySlice + 'a> for VerticesKey<&'a Path> {
  fn borrow(&self) -> &(dyn KeySlice + 'a) { self }
}

impl Hash for dyn KeySlice + '_ {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    threshold_hash(self.tolerance(), self.threshold()).hash(state);
    match self.style() {
      PathStyle::Fill => (-1).hash(state),
      PathStyle::Stroke(line_width) => threshold_hash(line_width, self.threshold()).hash(state),
    }
    match self.path() {
      PathKey::Path(path) => {
        let path: &ShadowPath = unsafe { std::mem::transmute(path) };
        as_bytes::<LyonPoint>(path.points.as_ref()).hash(state);
        as_bytes::<Verb>(path.verbs.as_ref()).hash(state);
      }
      PathKey::Glyph { glyph_id, face_id } => {
        glyph_id.hash(state);
        face_id.hash(state);
      }
    }
  }
}

impl PartialEq for dyn KeySlice + '_ {
  fn eq(&self, other: &Self) -> bool {
    threshold_hash(self.tolerance(), self.threshold())
      == threshold_hash(other.tolerance(), other.threshold())
      && match (self.style(), other.style()) {
        (PathStyle::Fill, PathStyle::Fill) => true,
        (PathStyle::Stroke(l1), PathStyle::Stroke(l2)) => {
          threshold_hash(l1, self.threshold()) == threshold_hash(l2, other.threshold())
        }
        _ => false,
      }
      && match (self.path(), other.path()) {
        (PathKey::Path(path1), PathKey::Path(path2)) => {
          let path1: &ShadowPath = unsafe { std::mem::transmute(path1.borrow()) };
          let path2: &ShadowPath = unsafe { std::mem::transmute(path2.borrow()) };
          as_bytes::<LyonPoint>(path1.points.as_ref())
            == as_bytes::<LyonPoint>(path2.points.as_ref())
            && as_bytes::<Verb>(path1.verbs.as_ref()) == as_bytes::<Verb>(path2.verbs.as_ref())
        }
        (
          PathKey::Glyph { glyph_id, face_id },
          PathKey::Glyph {
            glyph_id: glyph_id2,
            face_id: face_id2,
          },
        ) => glyph_id == glyph_id2 && face_id == face_id2,
        _ => false,
      }
  }
}

impl Eq for dyn KeySlice + '_ {}

impl ToOwned for dyn KeySlice + '_ {
  type Owned = VerticesKey<Path>;

  fn to_owned(&self) -> Self::Owned { self.to_key() }
}
#[cfg(test)]
mod tests {
  use crate::RenderData;
  use painter::{Color, DeviceSize, Painter, Point, Radius, Rect, Size, Transform};
  use text::shaper::TextShaper;
  extern crate test;
  use test::Bencher;

  use super::{atlas::tests::PureColorImage, *};

  fn tessellator() -> Tessellator {
    let shaper = TextShaper::default();
    shaper.font_db_mut().load_system_fonts();
    Tessellator::new(
      DeviceSize::new(128, 128),
      DeviceSize::new(512, 512),
      0.01,
      shaper,
    )
  }

  fn circle_rectangle_color_paint(painter: &mut Painter) {
    painter
      .set_brush(Color::RED)
      .circle(Point::new(10., 10.), 5.);
    painter.fill(None);

    painter.rect(&Rect::new(Point::new(0., 0.), Size::new(10., 10.)));
    painter.stroke(Some(2.), None);
  }

  fn two_img_paint(painter: &mut Painter) {
    let img = PureColorImage::shallow_img(Color::YELLOW, DeviceSize::new(100, 100));
    painter
      .set_brush(Brush::Image {
        img,
        tile_mode: TileMode::REPEAT_BOTH,
      })
      .circle(Point::new(10., 10.), 5.);
    painter.fill(None);
    painter.rect(&Rect::new(Point::new(0., 0.), Size::new(10., 10.)));
    painter.stroke(Some(2.), None);
  }

  #[test]
  fn color_commands_should_batch() {
    let mut tess = tessellator();
    let mut painter = Painter::new(1.);
    circle_rectangle_color_paint(&mut painter);
    let mut render_data = vec![];
    tess.tessellate(&painter.finish(), |r| match r {
      RenderData::Color(_) => render_data.push(true),
      RenderData::Image(_) => render_data.push(false),
    });

    assert_eq!(&render_data, &[true]);
  }

  #[test]
  fn img_should_batch() {
    let mut tess = tessellator();
    let mut painter = Painter::new(1.);
    two_img_paint(&mut painter);
    let mut render_data = vec![];
    tess.tessellate(&painter.finish(), |r| match r {
      RenderData::Color(_) => render_data.push(true),
      RenderData::Image(_) => render_data.push(false),
    });

    assert_eq!(&render_data, &[false]);
  }

  #[test]
  fn image_color_cannot_batch() {
    let mut tess = tessellator();
    let mut painter = Painter::new(1.);
    circle_rectangle_color_paint(&mut painter);
    two_img_paint(&mut painter);
    circle_rectangle_color_paint(&mut painter);
    circle_rectangle_color_paint(&mut painter);
    two_img_paint(&mut painter);

    let mut render_data = vec![];
    tess.tessellate(&painter.finish(), |r| match r {
      RenderData::Color(_) => render_data.push(true),
      RenderData::Image(_) => render_data.push(false),
    });

    assert_eq!(&render_data, &[true, false, true, false]);
  }

  #[test]
  fn large_image_cannot_batch() {
    let mut tess = tessellator();
    let mut painter = Painter::new(1.);

    two_img_paint(&mut painter);
    let large_img = PureColorImage::shallow_img(Color::YELLOW, DeviceSize::new(1024, 1024));
    painter.set_brush(Brush::Image {
      img: large_img,
      tile_mode: TileMode::REPEAT_BOTH,
    });
    #[derive(Debug, Clone)]
    struct PathHash(Path);

    painter.rect(&Rect::new(Point::new(0., 0.), Size::new(512., 512.)));
    painter.fill(None);
    two_img_paint(&mut painter);

    let mut render_data = vec![];
    tess.tessellate(&painter.finish(), |r| match r {
      RenderData::Color(_) => render_data.push(true),
      RenderData::Image(_) => render_data.push(false),
    });

    assert_eq!(&render_data, &[false, false, false]);
  }

  #[bench]
  fn million_diff_round_rect(b: &mut Bencher) {
    let mut painter = Painter::new(1.);
    painter.set_brush(Color::RED).set_line_width(2.);
    (1..1_000_000).for_each(|i| {
      let round = (i as f32 * 0.00_001).min(0.1);
      painter.rect_round(
        &Rect::new(Point::zero(), Size::new(100. + round, 100. + round)),
        &Radius::all(round),
      );
      if i % 2 == 0 {
        painter.stroke(None, None);
      } else {
        painter.fill(None);
      }
    });
    let commands = painter.finish();
    let mut tess = tessellator();
    b.iter(|| {
      tess.vertices_cache.clear();
      tess.tessellate(&commands, |_| {})
    })
  }

  #[test]
  fn million_diff_round_rect_x() {
    let mut painter = Painter::new(1.);
    painter.set_brush(Color::RED).set_line_width(2.);
    (1..1_000_000).for_each(|i| {
      let round = (i as f32 * 0.00_001).min(0.1);
      painter.rect_round(
        &Rect::new(Point::zero(), Size::new(100. + round, 100. + round)),
        &Radius::all(round),
      );
      if i % 2 == 0 {
        painter.stroke(None, None);
      } else {
        painter.fill(None);
      }
    });
    let commands = painter.finish();
    let mut tess = tessellator();
    tess.tessellate(&commands, |_| {})
  }

  #[bench]
  fn million_same_round_rect(b: &mut Bencher) {
    let mut painter = Painter::new(1.);
    painter.set_brush(Color::RED).set_line_width(2.);
    painter.rect_round(
      &Rect::new(Point::zero(), Size::new(100., 100.)),
      &Radius::all(2.),
    );
    painter.fill(None);
    let cmd = painter.finish().pop().unwrap();
    let commands = vec![cmd; 1_000_00];
    let mut tess = tessellator();
    tess.tessellate(&commands, |_| {});
    b.iter(|| tess.tessellate(&commands, |_| {}))
  }

  #[bench]
  fn diff_char_30k(b: &mut Bencher) {
    let mut painter = Painter::new(1.);
    painter.set_brush(Color::RED).set_line_width(2.);
    // 30k different char
    let text = include_str!("../../fonts/loads-of-unicode.txt");
    // let text = text.chars().take(100000).collect::<String>();
    painter.fill_text(text);
    let commands = painter.finish();
    let mut tess = tessellator();
    b.iter(|| {
      tess.vertices_cache.clear();
      tess.tessellate(&commands, |_| {})
    })
  }

  #[bench]
  fn char_with_cache_30k(b: &mut Bencher) {
    let mut painter = Painter::new(1.);
    painter.set_brush(Color::RED).set_line_width(2.);
    // 30k different char
    let text = include_str!("../../fonts/loads-of-unicode.txt");
    // let text = text.chars().take(100000).collect::<String>();
    painter.fill_text(text);
    let commands = painter.finish();
    let mut tess = tessellator();
    tess.tessellate(&commands, |_| {});
    b.iter(|| tess.tessellate(&commands, |_| {}))
  }
}
