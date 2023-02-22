use crate::{
  ColorPrimitive, DrawTriangles, GlRender, Primitive, StencilPrimitive, Texture, TexturePrimitive,
  TriangleLists, Vertex,
};
use lyon_tessellation::{path::Path as LyonPath, *};
use rayon::iter::ParallelIterator;
use rayon::prelude::ParallelSliceMut;
use ribir_algo::{FrameCache, Resource, ShareResource};
use ribir_painter::{
  Brush, ClipInstruct, PaintCommand, PaintInstruct, PaintPath, Path, PathStyle, TileMode, Transform,
};
use ribir_text::{
  font_db::ID,
  shaper::{GlyphId, TextShaper},
  Glyph,
};
use std::{collections::VecDeque, hash::Hash};
mod atlas;
use atlas::TextureAtlas;

mod mem_texture;
mod texture_records;
use texture_records::TextureRecords;

const ATLAS_ID: usize = 0;
const TOLERANCE: f32 = 0.01;
const MAX_VERTEX_CAN_BATCH: usize = 256 * 1024;
const TEXTURE_ID_FROM: usize = 1;
const PAR_CHUNKS_SIZE: usize = 128;
/// `Tessellator` use to generate triangles from
pub struct Tessellator {
  // todo: only a 4 bytes pixel atlas provide, should we add a 3 bytes atlas (useful for rgb) ?
  // texture atlas for pure color and image to draw.
  atlas: TextureAtlas,
  texture_records: TextureRecords,
  vertices_cache: Option<FrameCache<VerticesKey, Box<VertexCache>>>,
  vertices: Vec<Vertex>,
  indices: Vec<u32>,
  primitives: Vec<Primitive>,
  commands: Vec<DrawTriangles>,
  buffer_list: VecDeque<CacheItem>,
  /// The max vertex can batch. It's not a strict number, it's unaffected if
  /// it's less than the count of vertex generate by one paint command, default
  /// value is [`MAX_VERTEX_CAN_BATCH`]!
  vertex_batch_limit: usize,
  shaper: TextShaper,
}

#[derive(Clone, Copy)]
enum PrimitiveType {
  Color,
  Texture { id: usize },
  PushStencil,
  PopStencil,
}
struct CacheItem {
  prim_id: u32,
  cache_ptr: *mut VertexCache,
  prim_type: PrimitiveType,
}

#[derive(Debug, Clone)]
struct VerticesKey {
  tolerance: f32,
  path: PathKey,
}

#[derive(Debug, Clone)]
enum PathKey {
  Path(ShareResource<Path>),
  Glyph {
    glyph_id: GlyphId,
    face_id: ID,
    style: PathStyle,
  },
}
#[derive(Default)]
struct VertexCache {
  vertices: Box<[Vertex]>,
  indices: Box<[u32]>,
}

impl Tessellator {
  /// Create a `Tessellator` with the init texture size and the maximum texture
  /// size. `threshold` is the scale difference of a path need to retessellate.
  #[inline]
  pub fn new(tex_init_size: (u16, u16), tex_max_size: (u16, u16), shaper: TextShaper) -> Self {
    Self {
      atlas: TextureAtlas::new(tex_init_size.into(), tex_max_size.into()),
      vertex_batch_limit: MAX_VERTEX_CAN_BATCH,
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
    let mut no_cache_paths = vec![];
    let mut stencil_path = vec![];
    commands.iter().for_each(|cmd| {
      self.command_to_buffer(
        cmd,
        &mut stencil_path,
        |key| uninit_vertices.get_or_delay_init(key),
        |tolerance, path| {
          let mut vc = Box::<VertexCache>::default();
          let ptr = &mut *vc as *mut VertexCache;
          no_cache_paths.push((tolerance, path, vc));
          ptr
        },
        render,
      )
    });

    uninit_vertices.par_init_with(
      |key| Self::gen_triangles(&self.shaper, key),
      PAR_CHUNKS_SIZE,
    );
    no_cache_paths
      .par_chunks_mut(PAR_CHUNKS_SIZE)
      .for_each(|slice| {
        for (tolerance, path, cache) in slice {
          let valid = tesselate_path(&path.path, path.style, *tolerance);
          **cache = valid;
        }
      });
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

  fn prim_from_paint<R: GlRender>(
    &mut self,
    cmd: &PaintInstruct,
    render: &mut R,
  ) -> (Primitive, PrimitiveType) {
    match &cmd.brush {
      Brush::Color(color) => {
        let c = ColorPrimitive::new(
          color.into_linear_f32_components(),
          cmd.transform.clone().to_arrays(),
          cmd.opacity,
        );
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
          let box_rect = match &cmd.path {
            PaintPath::Path(path) => path.box_rect(),
            PaintPath::Text { .. } => todo!(),
          };
          if tile_mode.contains(TileMode::COVER_X) {
            factor[0] = w as f32 / box_rect.width();
          }
          if tile_mode.contains(TileMode::COVER_Y) {
            factor[1] = h as f32 / box_rect.height();
          }
        }
        let t = TexturePrimitive::new([x, y, w, h], factor, cmd.transform.to_arrays(), cmd.opacity);
        (t.into(), PrimitiveType::Texture { id })
      }
      Brush::Gradient => todo!(),
    }
  }

  fn prim_from_command<R: GlRender>(
    &mut self,
    cmd: &PaintCommand,
    stencil_path: &[&ClipInstruct],
    render: &mut R,
  ) -> (Primitive, PrimitiveType) {
    match cmd {
      PaintCommand::Paint(paint) => self.prim_from_paint(paint, render),
      PaintCommand::PushClip(clip) => (
        StencilPrimitive::new(clip.transform.clone().to_arrays()).into(),
        PrimitiveType::PushStencil,
      ),
      PaintCommand::PopClip => {
        let transform = stencil_path.last().unwrap().transform;
        (
          StencilPrimitive::new(transform.to_arrays()).into(),
          PrimitiveType::PopStencil,
        )
      }
    }
  }

  fn add_primitive(&mut self, p: Primitive) -> u32 {
    if self.primitives.last() != Some(&p) {
      self.primitives.push(p);
    }
    self.primitives.len() as u32 - 1
  }

  fn command_to_buffer<'a, F, F2, R>(
    &mut self,
    cmd: &'a PaintCommand,
    stencil_path: &mut Vec<&'a ClipInstruct>,
    cache: F,
    not_cache: F2,
    render: &mut R,
  ) where
    F: FnMut(VerticesKey) -> *mut VertexCache,
    F2: FnMut(f32, &'a Path) -> *mut VertexCache,
    R: GlRender,
  {
    let (primitive, prim_type) = self.prim_from_command(cmd, stencil_path, render);
    let (path, transform) = match cmd {
      PaintCommand::Paint(p) => (&p.path, &p.transform),
      PaintCommand::PushClip(clip) => {
        stencil_path.push(clip);
        (&clip.path, &clip.transform)
      }
      PaintCommand::PopClip => {
        let clip = stencil_path.pop().unwrap();
        (&clip.path, &clip.transform)
      }
    };

    self.path_to_buffer(path, transform, cache, not_cache, primitive, prim_type)
  }

  fn path_to_buffer<'a, F, F2>(
    &mut self,
    path: &'a PaintPath,
    transform: &Transform,
    mut cache: F,
    mut not_cache: F2,
    primitive: Primitive,
    prim_type: PrimitiveType,
  ) where
    F: FnMut(VerticesKey) -> *mut VertexCache,
    F2: FnMut(f32, &'a Path) -> *mut VertexCache,
  {
    let scale = transform.m11.max(transform.m22).max(f32::EPSILON);
    match path {
      PaintPath::Path(path) => {
        let prim_id = self.add_primitive(primitive);
        let tolerance = TOLERANCE / scale;
        let cache_ptr = match path {
          Resource::Share(path) => {
            let path = PathKey::Path(path.clone());
            let key = VerticesKey { tolerance, path };
            cache(key)
          }
          Resource::Local(path) => not_cache(tolerance, path),
        };

        self
          .buffer_list
          .push_back(CacheItem { prim_id, cache_ptr, prim_type });
      }
      PaintPath::Text { font_size, glyphs, style } => {
        let tolerance = TOLERANCE / (font_size.into_pixel().value() * scale);
        let font_size_ems = font_size.into_pixel().value();
        glyphs.iter().for_each(
          |&Glyph {
             face_id,
             x_offset,
             y_offset,
             glyph_id,
             ..
           }| {
            let path = PathKey::Glyph { face_id, glyph_id, style: *style };
            let key = VerticesKey { tolerance, path };
            let cache_ptr = cache(key);

            let t = transform
              // because glyph is up down mirror, this `font_size` offset help align after rotate.
              .pre_translate((x_offset.value(), y_offset.value()).into())
              .pre_scale(font_size_ems, font_size_ems);

            let mut p = primitive;
            let color_primitive = unsafe { &mut p.color_primitive };
            color_primitive.transform = t.to_arrays();

            let prim_id = self.add_primitive(p);
            self
              .buffer_list
              .push_back(CacheItem { prim_id, cache_ptr, prim_type });
          },
        );
      }
    };
  }

  /// Generate vertices from the buffer
  ///
  /// Caller also should guarantee the cache pointer is valid.
  unsafe fn fill_vertices(&mut self) -> bool {
    let mut use_atlas = false;
    let mut vertices_count = 0;
    let mut indices_count = 0;
    for item in &self.buffer_list {
      let cache = &mut *item.cache_ptr;
      vertices_count += cache.vertices.len();
      indices_count += cache.indices.len();
      if indices_count >= self.vertex_batch_limit {
        indices_count = self.vertex_batch_limit;
        break;
      }
    }
    self.vertices.reserve(vertices_count);
    self.indices.reserve(indices_count);

    let mut count = 0;
    for &CacheItem { prim_id, cache_ptr, prim_type } in self.buffer_list.iter() {
      let cache = &mut *cache_ptr;
      if self.indices.len() + cache.indices.len() > self.vertex_batch_limit {
        if cache.indices.len() > self.vertex_batch_limit {
          panic!("A paint command generate vertexes over the limit.")
        } else {
          break;
        }
      }

      unsafe fn copy_append<T>(vec: &mut Vec<T>, other: &[T]) {
        let start = vec.len();
        std::ptr::copy(other.as_ptr(), vec[start..].as_mut_ptr(), other.len());
        vec.set_len(start + other.len());
      }

      let offset = self.vertices.len() as u32;
      let indices_start = self.indices.len();
      copy_append(&mut self.indices, &cache.indices);
      self.indices[indices_start..]
        .iter_mut()
        .for_each(|i| *i += offset);

      cache.vertices.iter_mut().for_each(|v| v.prim_id = prim_id);
      copy_append(&mut self.vertices, &cache.vertices);

      let indices_start = indices_start as u32;
      let indices_count = cache.indices.len() as u32;

      match (self.commands.last_mut(), prim_type) {
        (Some(DrawTriangles::Color(rg)), PrimitiveType::Color) => {
          rg.end += indices_count;
        }
        (Some(DrawTriangles::Texture { rg, texture_id }), PrimitiveType::Texture { id })
          if *texture_id == id =>
        {
          rg.end += indices_count;
        }
        (_, PrimitiveType::Color) => self.commands.push(DrawTriangles::Color(
          indices_start..indices_start + indices_count,
        )),
        (_, PrimitiveType::Texture { id }) => {
          self.commands.push(DrawTriangles::Texture {
            rg: indices_start..indices_start + indices_count,
            texture_id: id,
          });
        }
        (_, PrimitiveType::PushStencil) => {
          self.commands.push(DrawTriangles::PushStencil(
            indices_start..indices_start + indices_count,
          ));
        }
        (_, PrimitiveType::PopStencil) => {
          self.commands.push(DrawTriangles::PopStencil(
            indices_start..indices_start + indices_count,
          ));
        }
      }

      use_atlas =
        use_atlas || matches!(prim_type, PrimitiveType::Texture {id, .. } if id == ATLAS_ID);
      count += 1;
    }

    self.buffer_list.drain(..count);
    use_atlas
  }

  fn gen_triangles(shaper: &TextShaper, key: &VerticesKey) -> VertexCache {
    let &VerticesKey { tolerance, ref path, .. } = key;
    match path {
      PathKey::Path(path) => tesselate_path(&path.path, path.style, tolerance),
      &PathKey::Glyph { glyph_id, face_id, style } => {
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

fn tesselate_path(path: &LyonPath, style: PathStyle, tolerance: f32) -> VertexCache {
  match style {
    ribir_painter::PathStyle::Fill => fill_tess(path, tolerance),
    ribir_painter::PathStyle::Stroke(mut options) => {
      options.tolerance = tolerance;
      stroke_tess(path, &options)
    }
  }
}

fn stroke_tess(path: &LyonPath, options: &StrokeOptions) -> VertexCache {
  let mut buffers = VertexBuffers::new();
  let mut stroke_tess = StrokeTessellator::default();
  stroke_tess
    .tessellate_path(
      path,
      options,
      &mut BuffersBuilder::new(&mut buffers, move |v: StrokeVertex| Vertex {
        pixel_coords: v.position().to_array(),
        prim_id: u32::MAX,
      }),
    )
    .unwrap();

  VertexCache {
    vertices: buffers.vertices.into_boxed_slice(),
    indices: buffers.indices.into_boxed_slice(),
  }
}

fn fill_tess(path: &LyonPath, tolerance: f32) -> VertexCache {
  let mut buffers = VertexBuffers::new();
  let mut fill_tess = FillTessellator::default();
  fill_tess
    .tessellate_path(
      path,
      &FillOptions::tolerance(tolerance),
      &mut BuffersBuilder::new(&mut buffers, move |v: FillVertex| Vertex {
        pixel_coords: v.position().to_array(),
        prim_id: u32::MAX,
      }),
    )
    .unwrap();

  VertexCache {
    vertices: buffers.vertices.into_boxed_slice(),
    indices: buffers.indices.into_boxed_slice(),
  }
}

impl Hash for VerticesKey {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.tolerance.to_bits().hash(state);
    core::mem::discriminant(&self.path).hash(state);
    match &self.path {
      PathKey::Path(p) => p.hash(state),
      PathKey::Glyph { glyph_id, face_id, style } => {
        glyph_id.hash(state);
        face_id.hash(state);
        core::mem::discriminant(style).hash(state);
        match style {
          PathStyle::Fill => {}
          PathStyle::Stroke(options) => stroke_options_hash(options, state),
        }
      }
    }
  }
}

impl PartialEq for VerticesKey {
  fn eq(&self, other: &Self) -> bool {
    self.tolerance == other.tolerance
      && match (&self.path, &other.path) {
        (PathKey::Path(l_p), PathKey::Path(r_p)) => l_p == r_p,
        (
          PathKey::Glyph {
            glyph_id: l_id,
            face_id: l_face,
            style: l_style,
          },
          PathKey::Glyph {
            glyph_id: r_id,
            face_id: r_face,
            style: r_style,
          },
        ) => l_id == r_id && l_face == r_face && l_style == r_style,
        _ => false,
      }
  }
}

impl Eq for VerticesKey {}

fn stroke_options_hash<H: std::hash::Hasher>(options: &StrokeOptions, state: &mut H) {
  options.line_width.to_bits().hash(state);
  core::mem::discriminant(&options.start_cap).hash(state);
  core::mem::discriminant(&options.end_cap).hash(state);
  core::mem::discriminant(&options.line_join).hash(state);
  options.miter_limit.to_bits().hash(state);
}

#[cfg(test)]
mod tests {
  use std::{
    error::Error,
    sync::{Arc, RwLock},
  };

  use crate::TriangleLists;
  use ribir_painter::{Color, DeviceSize, Painter, Point, Radius, Rect, Size};

  use ribir_text::{font_db::FontDB, shaper::TextShaper, TypographyStore};
  extern crate test;
  use test::Bencher;

  use super::{atlas::tests::color_image, *};

  impl<F: FnMut(TriangleLists)> GlRender for F {
    fn begin_frame(&mut self) {}

    fn add_texture(&mut self, _: Texture) {}

    fn draw_triangles(&mut self, data: TriangleLists) { self(data) }

    fn end_frame(&mut self, _: bool) {}

    fn capture(&self, _: crate::CaptureCallback) -> Result<(), Box<dyn Error>> { Ok(()) }

    fn resize(&mut self, _: DeviceSize) {}
  }

  fn tessellator() -> Tessellator {
    let shaper = TextShaper::new(<_>::default());
    shaper.font_db_mut().load_system_fonts();
    Tessellator::new((128, 128), (512, 512), shaper)
  }

  fn circle_rectangle_color_paint(painter: &mut Painter) {
    painter
      .set_brush(Color::RED)
      .circle(Point::new(10., 10.), 5.);
    painter.fill();

    painter
      .rect(&Rect::new(Point::new(0., 0.), Size::new(10., 10.)))
      .set_line_width(2.)
      .stroke();
  }

  fn two_img_paint(painter: &mut Painter) {
    let img = color_image(Color::YELLOW, 100, 100);
    painter
      .set_brush(Brush::Image {
        img,
        tile_mode: TileMode::REPEAT_BOTH,
      })
      .circle(Point::new(10., 10.), 5.);
    painter.fill();
    painter
      .rect(&Rect::new(Point::new(0., 0.), Size::new(10., 10.)))
      .set_line_width(2.)
      .stroke();
  }

  #[test]
  fn color_commands_should_batch() {
    let mut tess = tessellator();
    let mut painter = default_painter();
    circle_rectangle_color_paint(&mut painter);
    let mut render_data = vec![];
    tess.tessellate(&painter.finish(), &mut |data: TriangleLists| {
      data.commands.iter().for_each(|cmd| match cmd {
        DrawTriangles::Color(_) => render_data.push(true),
        DrawTriangles::Texture { .. } => render_data.push(false),
        _ => (),
      });
    });

    assert_eq!(&render_data, &[true]);
  }

  #[test]
  fn img_should_batch() {
    let mut tess = tessellator();
    let mut painter = default_painter();
    two_img_paint(&mut painter);
    let mut render_data = vec![];
    tess.tessellate(&painter.finish(), &mut |data: TriangleLists| {
      data.commands.iter().for_each(|cmd| match cmd {
        DrawTriangles::Color(_) => render_data.push(true),
        DrawTriangles::Texture { .. } => render_data.push(false),
        _ => (),
      });
    });

    assert_eq!(&render_data, &[false]);
  }

  #[test]
  fn image_color_cannot_batch() {
    let mut tess = tessellator();
    let mut painter = default_painter();
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
        _ => (),
      });
    });

    assert_eq!(&render_data, &[true, false, true, false]);
  }

  #[test]
  fn large_image_cannot_batch() {
    let mut tess = tessellator();
    let mut painter = default_painter();

    two_img_paint(&mut painter);
    let large_img = color_image(Color::YELLOW, 1024, 1024);
    painter.set_brush(Brush::Image {
      img: large_img,
      tile_mode: TileMode::REPEAT_BOTH,
    });
    #[derive(Debug, Clone)]
    struct PathHash(Path);

    painter.rect(&Rect::new(Point::new(0., 0.), Size::new(512., 512.)));
    painter.fill();
    two_img_paint(&mut painter);

    let mut render_data = vec![];
    tess.tessellate(&painter.finish(), &mut |data: TriangleLists| {
      data.commands.iter().for_each(|cmd| match cmd {
        DrawTriangles::Color(_) => render_data.push(true),
        DrawTriangles::Texture { .. } => render_data.push(false),
        _ => (),
      });
    });

    assert_eq!(&render_data, &[false, false, false]);
  }

  fn bench_rect_round() -> ribir_painter::Builder {
    let mut builder = Path::builder();
    builder.rect_round(
      &Rect::new(Point::zero(), Size::new(100., 100.)),
      &Radius::all(2.),
    );
    builder
  }

  #[bench]
  fn million_diff_round_rect(b: &mut Bencher) {
    let mut painter = default_painter();
    painter.set_brush(Color::RED);
    let fill_path = bench_rect_round().fill();
    let stroke_path = bench_rect_round().stroke(StrokeOptions::default().with_line_width(2.));
    painter.paint_path(fill_path).paint_path(stroke_path);
    let commands = painter.finish();
    let mut tess = tessellator();
    let commands = commands
      .into_iter()
      .cycle()
      .take(500_000)
      .collect::<Vec<_>>();
    tess.tessellate(&commands, &mut |_: TriangleLists| {});
    b.iter(|| {
      tess.vertices_cache.take();
      tess.tessellate(&commands, &mut |_: TriangleLists| {})
    })
  }

  #[bench]
  fn million_same_round_rect(b: &mut Bencher) {
    let mut painter = default_painter();
    painter.set_brush(Color::RED);
    let fill_path = bench_rect_round().fill();
    let stroke_path = bench_rect_round().stroke(StrokeOptions::default().with_line_width(2.));
    let fill_path = ShareResource::new(fill_path);
    let stroke_path = ShareResource::new(stroke_path);
    painter.paint_path(fill_path).paint_path(stroke_path);
    let commands = painter.finish();
    let mut tess = tessellator();
    let commands = commands
      .into_iter()
      .cycle()
      .take(500_000)
      .collect::<Vec<_>>();
    tess.tessellate(&commands, &mut |_: TriangleLists| {});
    b.iter(|| tess.tessellate(&commands, &mut |_: TriangleLists| {}))
  }

  macro_rules! text_bench {
    ($name: ident, $with_cache_name: ident, $text: expr) => {
      #[bench]
      fn $name(b: &mut Bencher) {
        let mut painter = default_painter();
        painter.fill_text($text, None);
        let commands = painter.finish();
        b.iter(|| {
          let mut tess = tessellator();
          tess.tessellate(&commands, &mut |_: TriangleLists| {})
        })
      }

      #[bench]
      fn $with_cache_name(b: &mut Bencher) {
        let mut painter = default_painter();
        painter.fill_text($text, None);
        let commands = painter.finish();
        let mut tess = tessellator();
        tess.tessellate(&commands, &mut |_: TriangleLists| {});
        b.iter(|| tess.tessellate(&commands, &mut |_: TriangleLists| {}))
      }
    };
  }

  text_bench!(
    unicode_symbols_30k,
    unicode_symbols_with_cache_30k,
    include_str!("../../fonts/loads-of-unicode.txt")
  );

  text_bench!(
    latin,
    latin_with_cache,
    #[allow(clippy::invisible_characters)]
    r#"!"\#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUV
    WXYZ[\]^_`abcdefghijklmnopqrstuvwxyz{|}~€‚ƒ„…†‡ˆ‰Š‹ŒŽ‘’“”•–—˜™š›œž¡¢£¤¥¦§¨
    ©ª«¬­®¯°±²³´µ¶·¸¹º»¼½¾¿ÀÁÂÃÄÅÆÇÈÉÊËÌÍÎÏÐÑÒÓÔÕÖ×ØÙÚÛÜÝÞßàáâãäåæçèéêëìíîïðñòóô
    õö÷øùúûüýþÿĀāĂăĄąĆćĈĉĊċČčĎďlatin-AĐđĒēĔĕĖėĘęĚěĜĝĞğĠġĢģĤĥĦħĨĩĪīĬĭĮįİıĲĳĴĵĶķĸĹ
    ĺĻļĽľĿŀŁłŃńŅņŇňŉŊŋŌōŎŏŐőŒœŔŕŖŗŘřŚśŜŝŞşŠšŢţŤťŦŧŨũŪūŬŭŮůŰűŲųŴŵŶŷŸŹźŻżŽžſǍǎǏǐǑǒ
    ǓǔǕǖǗǘǙǚǛǜƏƒƠơƯƯǺǻǼǽǾǿ"#
  );

  text_bench!(
    chinese_2500,
    chinese_2500_with_cache,
    "一乙二十丁厂七卜人入八九几儿了力乃刀又三于干亏士工土才寸下大丈与万
    上小口巾山千乞川亿个勺久凡及夕丸么广亡门义之尸弓己已子卫也女飞刃习叉马乡丰王井开夫天无元专
    云扎艺木五支厅不太犬区历尤友匹车巨牙屯比互切瓦止少日中冈贝内水见午牛手毛气升长仁什片仆化仇
    币仍仅斤爪反介父从今凶分乏公仓月氏勿欠风丹匀乌凤勾文六方火为斗忆订计户认心尺引丑巴孔队办以
    允予劝双书幻玉刊示末未击打巧正扑扒功扔去甘世古节本术可丙左厉右石布龙平灭轧东卡北占业旧帅归
    且旦目叶甲申叮电号田由史只央兄叼叫另叨叹四生失禾丘付仗代仙们仪白仔他斥瓜乎丛令用甩印乐句匆
    册犯外处冬鸟务包饥主市立闪兰半汁汇头汉宁穴它讨写让礼训必议讯记永司尼民出辽奶奴加召皮边发孕
    圣对台矛纠母幼丝式刑动扛寺吉扣考托老执巩圾扩扫地扬场耳共芒亚芝朽朴机权过臣再协西压厌在有百
    存而页匠夸夺灰达列死成夹轨邪划迈毕至此贞师尘尖劣光当早吐吓虫曲团同吊吃因吸吗屿帆岁回岂刚则
    肉网年朱先丢舌竹迁乔伟传乒乓休伍伏优伐延件任伤价份华仰仿伙伪自血向似后行舟全会杀合兆企众爷
    伞创肌朵杂危旬旨负各名多争色壮冲冰庄庆亦刘齐交次衣产决充妄闭问闯羊并关米灯州汗污江池汤忙兴
    宇守宅字安讲军许论农讽设访寻那迅尽导异孙阵阳收阶阴防奸如妇好她妈戏羽观欢买红纤级约纪驰巡寿
    弄麦形进戒吞远违运扶抚坛技坏扰拒找批扯址走抄坝贡攻赤折抓扮抢孝均抛投坟抗坑坊抖护壳志扭块声
    把报却劫芽花芹芬苍芳严芦劳克苏杆杠杜材村杏极李杨求更束豆两丽医辰励否还歼来连步坚旱盯呈时吴
    助县里呆园旷围呀吨足邮男困吵串员听吩吹呜吧吼别岗帐财针钉告我乱利秃秀私每兵估体何但伸作伯伶
    佣低你住位伴身皂佛近彻役返余希坐谷妥含邻岔肝肚肠龟免狂犹角删条卵岛迎饭饮系言冻状亩况床库疗
    应冷这序辛弃冶忘闲间闷判灶灿弟汪沙汽沃泛沟没沈沉怀忧快完宋宏牢究穷灾良证启评补初社识诉诊词
    译君灵即层尿尾迟局改张忌际陆阿陈阻附妙妖妨努忍劲鸡驱纯纱纳纲驳纵纷纸纹纺驴纽奏春帮珍玻毒型
    挂封持项垮挎城挠政赴赵挡挺括拴拾挑指垫挣挤拼挖按挥挪某甚革荐巷带草茧茶荒茫荡荣故胡南药标枯
    柄栋相查柏柳柱柿栏树要咸威歪研砖厘厚砌砍面耐耍牵残殃轻鸦皆背战点临览竖省削尝是盼眨哄显哑冒
    映星昨畏趴胃贵界虹虾蚁思蚂虽品咽骂哗咱响哈咬咳哪炭峡罚贱贴骨钞钟钢钥钩卸缸拜看矩怎牲选适秒
    香种秋科重复竿段便俩贷顺修保促侮俭俗俘信皇泉鬼侵追俊盾待律很须叙剑逃食盆胆胜胞胖脉勉狭狮独
    狡狱狠贸怨急饶蚀饺饼弯将奖哀亭亮度迹庭疮疯疫疤姿亲音帝施闻阀阁差养美姜叛送类迷前首逆总炼炸
    炮烂剃洁洪洒浇浊洞测洗活派洽染济洋洲浑浓津恒恢恰恼恨举觉宣室宫宪突穿窃客冠语扁袄祖神祝误诱
    说诵垦退既屋昼费陡眉孩除险院娃姥姨姻娇怒架贺盈勇怠柔垒绑绒结绕骄绘给络骆绝绞统耕耗艳泰珠班
    素蚕顽盏匪捞栽捕振载赶起盐捎捏埋捉捆捐损都哲逝捡换挽热恐壶挨耻耽恭莲莫荷获晋恶真框桂档桐株
    桥桃格校核样根索哥速逗栗配翅辱唇夏础破原套逐烈殊顾轿较顿毙致柴桌虑监紧党晒眠晓鸭晃晌晕蚊哨
    哭恩唤啊唉罢峰圆贼贿钱钳钻铁铃铅缺氧特牺造乘敌秤租积秧秩称秘透笔笑笋债借值倚倾倒倘俱倡候俯
    倍倦健臭射躬息徒徐舰舱般航途拿爹爱颂翁脆脂胸胳脏胶脑狸狼逢留皱饿恋桨浆衰高席准座脊症病疾疼
    疲效离唐资凉站剖竞部旁旅畜阅羞瓶拳粉料益兼烤烘烦烧烛烟递涛浙涝酒涉消浩海涂浴浮流润浪浸涨烫
    涌悟悄悔悦害宽家宵宴宾窄容宰案请朗诸读扇袜袖袍被祥课谁调冤谅谈谊剥恳展剧屑弱陵陶陷陪娱娘通
    能难预桑绢绣验继球理捧堵描域掩捷排掉堆推掀授教掏掠培接控探据掘职基著勒黄萌萝菌菜萄菊萍菠营
    械梦梢梅检梳梯桶救副票戚爽聋袭盛雪辅辆虚雀堂常匙晨睁眯眼悬野啦晚啄距跃略蛇累唱患唯崖崭崇圈
    铜铲银甜梨犁移笨笼笛符第敏做袋悠偿偶偷您售停偏假得衔盘船斜盒鸽悉欲彩领脚脖脸脱象够猜猪猎猫
    猛馅馆凑减毫麻痒痕廊康庸鹿盗章竟商族旋望率着盖粘粗粒断剪兽清添淋淹渠渐混渔淘液淡深婆梁渗情
    惜惭悼惧惕惊惨惯寇寄宿窑密谋谎祸谜逮敢屠弹随蛋隆隐婚婶颈绩绪续骑绳维绵绸绿琴斑替款堪搭塔越
    趁趋超提堤博揭喜插揪搜煮援裁搁搂搅握揉斯期欺联散惹葬葛董葡敬葱落朝辜葵棒棋植森椅椒棵棍棉棚
    棕惠惑逼厨厦硬确雁殖裂雄暂雅辈悲紫辉敞赏掌晴暑最量喷晶喇遇喊景践跌跑遗蛙蛛蜓喝喂喘喉幅帽赌
    赔黑铸铺链销锁锄锅锈锋锐短智毯鹅剩稍程稀税筐等筑策筛筒答筋筝傲傅牌堡集焦傍储奥街惩御循艇舒
    番释禽腊脾腔鲁猾猴然馋装蛮就痛童阔善羡普粪尊道曾焰港湖渣湿温渴滑湾渡游滋溉愤慌惰愧愉慨割寒
    富窜窝窗遍裕裤裙谢谣谦属屡强粥疏隔隙絮嫂登缎缓编骗缘瑞魂肆摄摸填搏塌鼓摆携搬摇搞塘摊蒜勤鹊
    蓝墓幕蓬蓄蒙蒸献禁楚想槐榆楼概赖酬感碍碑碎碰碗碌雷零雾雹输督龄鉴睛睡睬鄙愚暖盟歇暗照跨跳跪
    路跟遣蛾蜂嗓置罪罩错锡锣锤锦键锯矮辞稠愁筹签简毁舅鼠催傻像躲微愈遥腰腥腹腾腿触解酱痰廉新韵
    意粮数煎塑慈煤煌满漠源滤滥滔溪溜滚滨粱滩慎誉塞谨福群殿辟障嫌嫁叠缝缠静碧璃墙撇嘉摧截誓境摘
    摔聚蔽慕暮蔑模榴榜榨歌遭酷酿酸磁愿需弊裳颗嗽蜻蜡蝇蜘赚锹锻舞稳算箩管僚鼻魄貌膜膊膀鲜疑馒裹
    敲豪膏遮腐瘦辣竭端旗精歉熄熔漆漂漫滴演漏慢寨赛察蜜谱嫩翠熊凳骡缩慧撕撒趣趟撑播撞撤增聪鞋蕉
    蔬横槽樱橡飘醋醉震霉瞒题暴瞎影踢踏踩踪蝶蝴嘱墨镇靠稻黎稿稼箱箭篇僵躺僻德艘膝膛熟摩颜毅糊遵
    潜潮懂额慰劈操燕薯薪薄颠橘整融醒餐嘴蹄器赠默镜赞篮邀衡膨雕磨凝辨辩糖糕燃澡激懒壁避缴戴擦鞠
    藏霜霞瞧蹈螺穗繁辫赢糟糠燥臂翼骤鞭覆蹦镰翻鹰警攀蹲颤瓣爆疆壤耀躁嚼嚷籍魔灌蠢霸露囊罐"
  );

  fn default_painter() -> Painter {
    let font_db = Arc::new(RwLock::new(FontDB::default()));
    font_db.write().unwrap().load_system_fonts();
    let store = TypographyStore::new(<_>::default(), font_db.clone(), TextShaper::new(font_db));
    Painter::new(1., store, Size::new(4096., 1024.))
  }
}
