use crate::{
  ColorPrimitive, DrawTriangles, GlRender, Primitive, Texture, TexturePrimitive, TriangleLists,
  Vertex,
};
use algo::FrameCache;
use lyon_tessellation::{path::Path, *};
use painter::{Brush, PaintCommand, PaintPath, PathStyle, TileMode, Vector};
use std::{collections::VecDeque, hash::Hash, mem::size_of};
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
  // todo: only a 4 bytes pixel atlas provide, should we add a 3 bytes atlas (useful for rgb) ?
  // texture atlas for pure color and image to draw.
  atlas: TextureAtlas,
  texture_records: TextureRecords,
  vertices_cache: Option<FrameCache<VerticesKey<Path>, Box<VertexCache>>>,
  vertices: Vec<Vertex>,
  indices: Vec<u32>,
  primitives: Vec<Primitive>,
  commands: Vec<DrawTriangles>,
  buffer_list: VecDeque<CacheItem>,
  /// The max vertex can batch. It's not a strict number, it's unaffected if
  /// it's less than the count of vertex generate by one paint command, default
  /// value is [`MAX_VERTEX_CAN_BATCH`]!
  vertex_batch_limit: usize,
  /// The scaling difference of a same path need to retessellate.
  threshold: f32,
  shaper: TextShaper,
}

#[derive(Clone, Copy)]
enum PrimitiveType {
  Color,
  Texture(usize),
}
struct CacheItem {
  prim_id: u32,
  cache_ptr: *mut VertexCache,
  prim_type: PrimitiveType,
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
}

impl Tessellator {
  /// Create a `Tessellator` with the init texture size and the maximum texture
  /// size. `threshold` is the scale difference of a path need to retessellate.
  #[inline]
  pub fn new(
    tex_init_size: (u16, u16),
    tex_max_size: (u16, u16),
    threshold: f32,
    shaper: TextShaper,
  ) -> Self {
    Self {
      atlas: TextureAtlas::new(tex_init_size.into(), tex_max_size.into()),
      vertex_batch_limit: MAX_VERTEX_CAN_BATCH,
      threshold,
      shaper,
      vertices_cache: None,
      texture_records: TextureRecords::new(TEXTURE_ID_FROM),
      vertices: vec![],
      indices: vec![],
      primitives: vec![],
      commands: vec![],
      buffer_list: <_>::default(),
    }
  }

  /// The vertex count to trigger a gpu submit. It's not a strict number, may
  /// exceed this limit by one paint command
  pub fn set_vertex_batch_limit(&mut self, count: usize) { self.vertex_batch_limit = count; }

  pub fn tessellate<R: GlRender>(&mut self, commands: &[PaintCommand], render: &mut R) {
    if commands.is_empty() {
      return;
    }

    // parallel generate triangles
    let mut vertices_cache = self.vertices_cache.take();
    let mut uninit_vertices = vertices_cache
      .get_or_insert_with(<_>::default)
      .as_uninit_map();
    commands.iter().for_each(|cmd| {
      self.command_to_buffer(
        cmd,
        |key| uninit_vertices.get_or_delay_init::<dyn KeySlice>(key),
        render,
      )
    });
    uninit_vertices.par_init_with(|key| Self::gen_triangles(&self.shaper, &key));
    self.vertices_cache = vertices_cache;

    while !self.buffer_list.is_empty() {
      let used_atlas = unsafe { self.fill_vertices() };
      if used_atlas {
        render.add_texture(self.atlas_texture());
        self.atlas.data_synced();
      }

      render.draw_triangles(self.get_triangle_list());
      self.clear_buffer();
    }

    self.end_frame()
  }

  fn end_frame(&mut self) {
    assert!(self.buffer_list.is_empty());
    // end frame to clear miss cache, atlas and vertexes clear before by itself.
    self.texture_records.end_frame();
    if let Some(vertices_cache) = self.vertices_cache.as_mut() {
      vertices_cache.end_frame("Vertices");
    }
    self.atlas.end_frame();
  }

  fn prim_from_command<R: GlRender>(
    &mut self,
    cmd: &PaintCommand,
    render: &mut R,
  ) -> (Primitive, PrimitiveType) {
    match &cmd.brush {
      Brush::Color(color) => {
        let c = ColorPrimitive {
          color: color.clone().into_arrays(),
          transform: cmd.transform.clone().to_arrays(),
        };
        (c.into(), PrimitiveType::Color)
      }
      Brush::Image { img, tile_mode } => {
        let mut id = ATLAS_ID;
        let rect = self.atlas.store_image(img).unwrap_or_else(|_| {
          let size = img.size();

          let format = img.color_format();
          id = self.texture_records.get_id(img).unwrap_or_else(|| {
            let data = Some(img.pixel_bytes());
            let id = self.texture_records.insert(img.clone());
            render.add_texture(Texture { id, data, size, format });
            id
          });

          mem_texture::Rect::from_size(img.size().into())
        });
        let (x, y) = rect.min().to_tuple();
        let (w, h) = rect.size.to_tuple();
        let mut factor = [1., 1.];
        if tile_mode.is_cover_mode() {
          let box_rect = cmd.box_rect_without_transform();
          if tile_mode.contains(TileMode::COVER_X) {
            factor[0] = w as f32 / box_rect.width();
          }
          if tile_mode.contains(TileMode::COVER_Y) {
            factor[1] = h as f32 / box_rect.height();
          }
        }
        let t = TexturePrimitive {
          tex_rect: [x, y, w, h],
          factor,
          transform: cmd.transform.to_arrays(),
        };
        (t.into(), PrimitiveType::Texture(id))
      }
      Brush::Gradient => todo!(),
    }
  }

  fn add_primitive(&mut self, p: Primitive) -> u32 {
    if self.primitives.last() != Some(&p) {
      self.primitives.push(p);
    }
    self.primitives.len() as u32 - 1
  }

  fn command_to_buffer<'a, F, R>(&mut self, cmd: &'a PaintCommand, mut cache: F, render: &mut R)
  where
    F: FnMut(VerticesKey<&'a Path>) -> *mut VertexCache,
    R: GlRender,
  {
    let (primitive, prim_type) = self.prim_from_command(cmd, render);

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
        let prim_id = self.add_primitive(primitive);
        self
          .buffer_list
          .push_back(CacheItem { prim_id, cache_ptr, prim_type });
      }
      &PaintPath::Text {
        font_size,
        letter_space,
        line_height,
        ref text,
        ref font_face,
        ..
      } if font_size > f32::EPSILON => {
        let face_ids = self.shaper.font_db_mut().select_all_match(font_face);
        let glyphs = self.shaper.shape_text(text, &face_ids);
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

        let mut pre_face_id = None;
        let mut pre_unit_per_em = 0;
        let mut scaled_font_size = font_size;
        let mut tolerance = TOLERANCE / (scaled_font_size * scale);
        text::layout::layout_text(text, &glyphs, &cfg, None).for_each(
          |GlyphAt { glyph_id, face_id, x, y }| {
            if Some(face_id) != pre_face_id {
              pre_face_id = Some(face_id);
              let db = self.shaper.font_db();
              pre_unit_per_em = db.try_get_face_data(face_id).unwrap().units_per_em();
              scaled_font_size = font_size / pre_unit_per_em as f32;
              tolerance = TOLERANCE / (scaled_font_size * scale)
            };

            let path = PathKey::<&Path>::Glyph { face_id, glyph_id };
            let key = VerticesKey { tolerance, threshold, style, path };
            let cache_ptr = cache(key);
            let t = transform
              // because glyph is up down mirror, this `font_size` offset help align after rotate.
              .pre_translate(Vector::new(x, y + font_size))
              .pre_scale(scaled_font_size, scaled_font_size);

            let mut p = primitive.clone();
            p.transform = t.to_arrays();

            let prim_id = self.add_primitive(p);
            self
              .buffer_list
              .push_back(CacheItem { prim_id, cache_ptr, prim_type });
          },
        );
      }
      _ => {}
    };
  }

  /// Generate vertices from the buffer
  ///
  /// Caller also should guarantee the cache pointer is valid.
  unsafe fn fill_vertices(&mut self) -> bool {
    let mut use_atlas = false;
    while let Some(CacheItem { prim_id, cache_ptr, prim_type }) = self.buffer_list.pop_front() {
      let cache = &mut *cache_ptr;
      let offset = self.vertices.len() as u32;

      self.vertices.extend(
        cache
          .vertices
          .iter()
          .map(|pos| Vertex { pixel_coords: *pos, prim_id }),
      );
      let indices_start = self.indices.len() as u32;

      self
        .indices
        .extend(cache.indices.iter().map(|i| i + offset));

      let indices_count = cache.indices.len() as u32;

      match (self.commands.last_mut(), prim_type) {
        (Some(DrawTriangles::Color(rg)), PrimitiveType::Color) => {
          rg.end += indices_count;
        }
        (Some(DrawTriangles::Texture { rg, texture_id }), PrimitiveType::Texture(id))
          if *texture_id == id =>
        {
          rg.end += indices_count;
        }
        (_, PrimitiveType::Color) => self.commands.push(DrawTriangles::Color(
          indices_start..indices_start + indices_count,
        )),
        (_, PrimitiveType::Texture(texture_id)) => {
          self.commands.push(DrawTriangles::Texture {
            rg: indices_start..indices_start + indices_count,
            texture_id,
          });
        }
      }

      if self.indices.len() > self.vertex_batch_limit {
        break;
      }
      use_atlas = use_atlas || matches!(prim_type, PrimitiveType::Texture(id) if id == ATLAS_ID);
    }
    use_atlas
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

  fn get_triangle_list(&self) -> TriangleLists {
    TriangleLists {
      vertices: &self.vertices,
      indices: &self.indices,
      primitives: &self.primitives,
      commands: &self.commands,
    }
  }

  fn clear_buffer(&mut self) {
    self.vertices.clear();
    self.indices.clear();
    self.primitives.clear();
    self.commands.clear();
  }

  fn atlas_texture(&self) -> Texture {
    let tex = self.atlas.texture();
    let data = self.atlas.is_updated().then(|| tex.as_bytes());
    Texture {
      id: ATLAS_ID,
      size: tex.size().into(),
      data,
      format: TextureAtlas::FORMAT,
    }
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
  }
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
  use crate::TriangleLists;
  use painter::{Color, DeviceSize, Painter, Point, Radius, Rect, Size};
  use text::shaper::TextShaper;
  extern crate test;
  use test::Bencher;

  use super::{atlas::tests::color_image, *};

  impl<F: FnMut(TriangleLists)> GlRender for F {
    fn begin_frame(&mut self) {}

    fn add_texture(&mut self, _: Texture) {}

    fn draw_triangles(&mut self, data: TriangleLists) { self(data) }

    fn end_frame<'a>(
      &mut self,
      _: Option<Box<dyn for<'r> FnOnce(DeviceSize, Box<dyn Iterator<Item = &[u8]> + 'r>) + 'a>>,
    ) -> Result<(), &str> {
      Ok(())
    }

    fn resize(&mut self, _: DeviceSize) {}
  }

  fn tessellator() -> Tessellator {
    let shaper = TextShaper::default();
    shaper.font_db_mut().load_system_fonts();
    Tessellator::new((128, 128), (512, 512), 0.01, shaper)
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
    let img = color_image(Color::YELLOW, 100, 100);
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
    tess.tessellate(&painter.finish(), &mut |data: TriangleLists| {
      data.commands.iter().for_each(|cmd| match cmd {
        DrawTriangles::Color(_) => render_data.push(true),
        DrawTriangles::Texture { .. } => render_data.push(false),
      });
    });

    assert_eq!(&render_data, &[true]);
  }

  #[test]
  fn img_should_batch() {
    let mut tess = tessellator();
    let mut painter = Painter::new(1.);
    two_img_paint(&mut painter);
    let mut render_data = vec![];
    tess.tessellate(&painter.finish(), &mut |data: TriangleLists| {
      data.commands.iter().for_each(|cmd| match cmd {
        DrawTriangles::Color(_) => render_data.push(true),
        DrawTriangles::Texture { .. } => render_data.push(false),
      });
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
    tess.tessellate(&painter.finish(), &mut |data: TriangleLists| {
      data.commands.iter().for_each(|cmd| match cmd {
        DrawTriangles::Color(_) => render_data.push(true),
        DrawTriangles::Texture { .. } => render_data.push(false),
      });
    });

    assert_eq!(&render_data, &[true, false, true, false]);
  }

  #[test]
  fn large_image_cannot_batch() {
    let mut tess = tessellator();
    let mut painter = Painter::new(1.);

    two_img_paint(&mut painter);
    let large_img = color_image(Color::YELLOW, 1024, 1024);
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
    tess.tessellate(&painter.finish(), &mut |data: TriangleLists| {
      data.commands.iter().for_each(|cmd| match cmd {
        DrawTriangles::Color(_) => render_data.push(true),
        DrawTriangles::Texture { .. } => render_data.push(false),
      });
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
      tess.vertices_cache.take();
      tess.tessellate(&commands, &mut |_: TriangleLists| {})
    })
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
    tess.tessellate(&commands, &mut |_: TriangleLists| {});
    b.iter(|| tess.tessellate(&commands, &mut |_: TriangleLists| {}))
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
      tess.vertices_cache.take();
      tess.tessellate(&commands, &mut |_: TriangleLists| {})
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
    tess.tessellate(&commands, &mut |_: TriangleLists| {});
    b.iter(|| tess.tessellate(&commands, &mut |_: TriangleLists| {}))
  }
}
