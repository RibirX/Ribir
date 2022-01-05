use crate::{
  ColorPrimitive, ColorRenderData, RenderData, Texture, TexturePrimitive, TextureRenderData, Vertex,
};
use algo::FrameCache;
use lyon_tessellation::{path::Path, *};
use painter::{Brush, DeviceSize, PaintCommand, PathStyle, TileMode};
use std::{collections::VecDeque, hash::Hash};
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
  stroke_tess: StrokeTessellator,
  fill_tess: FillTessellator,
  vertex_caches: FrameCache<VertexesKey, Box<VertexCache>>,
  /// The scaling difference of a same path need to retessellate.
  threshold: f32,
  buffer: TessData,
  texture_records: TextureRecords,
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
  cache_list: VecDeque<CacheItem>,
  /// The max vertex can batch. It's not a strict number, it's unaffected if
  /// it's less than the count of vertex generate by one paint command, default
  /// value is [`MAX_VERTEX_CAN_BATCH`]!
  vertex_batch_limit: usize,
}

struct CacheItem {
  cmd_idx: usize,
  cache_ptr: *mut VertexCache,
}
#[derive(Debug, Clone)]
struct VertexesKey {
  scale: f32,
  threshold: f32,
  path: Path,
  style: PathStyle,
}

struct VertexCache {
  vertexes: Box<[[f32; 2]]>,
  indices: Box<[u32]>,
  indices_offset: Option<u32>,
}

impl Tessellator {
  /// Create a `Tessellator` with the init texture size and the maximum texture
  /// size. `threshold` is the scale difference of a path need to retessellate.
  #[inline]
  pub fn new(tex_init_size: DeviceSize, tex_max_size: DeviceSize, threshold: f32) -> Self {
    Self {
      atlas: TextureAtlas::new(tex_init_size, tex_max_size),
      stroke_tess: <_>::default(),
      fill_tess: <_>::default(),
      vertex_caches: <_>::default(),
      threshold,
      buffer: TessData {
        vertex_batch_limit: MAX_VERTEX_CAN_BATCH,
        ..<_>::default()
      },
      texture_records: TextureRecords::new(TEXTURE_ID_FROM),
    }
  }

  /// The vertex count to trigger a gpu submit. It's not a strict number, may
  /// exceed this limit by one paint command
  pub fn set_vertex_batch_limit(&mut self, count: usize) { self.buffer.vertex_batch_limit = count; }

  pub fn tessellate<F>(&mut self, commands: Vec<PaintCommand>, mut gpu_submit: F)
  where
    F: FnMut(RenderData),
  {
    if commands.is_empty() {
      return;
    }

    // all vertexes are cached and a cache list described the vertexes order.
    self.push_cache_list_to_buffer(&commands);

    // Try prepare atlas texture for all brush, if not success, deallocate the
    // sub texture which cache missed and then refill by auto grow mode.
    let any_brush_fill_fail = !self.try_fill_atlas(&commands);
    if any_brush_fill_fail {
      self.atlas.end_frame();
    }

    let mut submitted = 0;
    loop {
      // find the atlas cannot batch command position.
      let mut filled = commands.len();
      if any_brush_fill_fail {
        let fill_failed_pos = commands[submitted..].iter().position(|cmd| {
          matches!(
            &cmd.brush, Brush::Image { img, .. }
            if self.atlas.store_image(img.clone(), true).is_err()
          )
        });
        if let Some(pos) = fill_failed_pos {
          filled = pos;
        }
      };

      if submitted == filled {
        let img = if let Brush::Image { img, .. } = &commands[submitted].brush {
          img
        } else {
          unreachable!("if fill texture failed, must be a large texture here");
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
        // fill buffer as much as possible
        let to_submit = &commands[filled..filled + 1];
        submitted += unsafe { self.buffer.fill_vertexes(to_submit, &self.atlas) };
        assert_ne!(submitted, filled);
        self.buffer.submit_tex_vertexes(&mut gpu_submit, texture);
      } else {
        while submitted < filled {
          let to_submit = &commands[submitted..filled];
          submitted += unsafe { self.buffer.fill_vertexes(to_submit, &self.atlas) };
          if self.buffer.is_color_buffer() {
            self.buffer.submit_color_vertexes(&mut gpu_submit);
          } else {
            let atlas_tex = self.atlas.as_render_texture(ATLAS_ID);
            self.buffer.submit_tex_vertexes(&mut gpu_submit, atlas_tex);
          }
          self.atlas.gpu_synced();
        }
      }

      if submitted >= commands.len() {
        break;
      }
    }

    // end frame to clear miss cache, atlas and vertexes clear before by itself.
    self.texture_records.frame_end();
    assert!(self.buffer.cache_list.is_empty())
  }

  fn tesselate_path(&mut self, path: &Path, style: PathStyle, scale: f32) -> VertexCache {
    let tolerance = TOLERANCE / scale;
    match style {
      painter::PathStyle::Fill => self.fill_tess(path, tolerance),
      painter::PathStyle::Stroke(line_width) => self.stroke_tess(path, line_width, tolerance),
    }
  }

  fn stroke_tess(&mut self, path: &Path, line_width: f32, tolerance: f32) -> VertexCache {
    let mut buffers = VertexBuffers::new();
    self
      .stroke_tess
      .tessellate_path(
        path,
        &StrokeOptions::tolerance(tolerance).with_line_width(line_width),
        &mut BuffersBuilder::new(&mut buffers, move |v: StrokeVertex| v.position().to_array()),
      )
      .unwrap();

    VertexCache {
      vertexes: buffers.vertices.into_boxed_slice(),
      indices: buffers.indices.into_boxed_slice(),
      indices_offset: None,
    }
  }

  fn fill_tess(&mut self, path: &Path, tolerance: f32) -> VertexCache {
    let mut buffers = VertexBuffers::new();
    self
      .fill_tess
      .tessellate_path(
        path,
        &FillOptions::tolerance(tolerance),
        &mut BuffersBuilder::new(&mut buffers, move |v: FillVertex| v.position().to_array()),
      )
      .unwrap();

    VertexCache {
      vertexes: buffers.vertices.into_boxed_slice(),
      indices: buffers.indices.into_boxed_slice(),
      indices_offset: None,
    }
  }

  fn try_fill_atlas(&mut self, commands: &[PaintCommand]) -> bool {
    commands
      .iter()
      .map(|cmd| {
        if let Brush::Image { img, .. } = &cmd.brush {
          self.atlas.store_image(img.clone(), true).is_ok()
        } else {
          true
        }
      })
      .all(|b| b)
  }

  /// Force get vertexes cache, even if no cache have, a slot will be inserted
  /// and  will generate cache later. Return the missed cache.
  fn push_cache_list_to_buffer(&mut self, commands: &[PaintCommand]) {
    let mut cache_missed = vec![];

    let mut force_cache = |cmd_idx: usize, path: Path, scale: f32, style: PathStyle| {
      let key = VertexesKey::new(scale, path, style, self.threshold);
      if let Some(cache) = self.vertex_caches.get_mut(&key) {
        self.buffer.cache_list.push_back(CacheItem {
          cmd_idx,
          cache_ptr: &mut **cache as *mut VertexCache,
        });
      } else {
        let mut cache = Box::new(VertexCache {
          vertexes: Box::new([]),
          indices: Box::new([]),
          indices_offset: None,
        });
        self.buffer.cache_list.push_back(CacheItem {
          cmd_idx,
          cache_ptr: &mut *cache as *mut VertexCache,
        });
        cache_missed.push(key.clone());
        self.vertex_caches.insert(key, cache);
      }
    };

    commands.iter().enumerate().for_each(|(idx, cmd)| {
      let PaintCommand { path, transform, path_style, .. } = cmd;
      match path {
        painter::PaintPath::Path(path) => {
          let scale = transform.m11.max(transform.m22).max(f32::EPSILON);
          force_cache(idx, path.clone(), scale, *path_style);
        }
        painter::PaintPath::Text { .. } => todo!(),
      };
    });

    self.vertex_caches.frame_end("Vertexes");

    // missed cache generate
    // todo: pair iter
    cache_missed.iter().for_each(|k| {
      let vertexes = self.tesselate_path(&k.path, k.style, k.scale);
      let cache = self
        .vertex_caches
        .get_mut(k)
        .expect("should keep a slot before");
      **cache = vertexes;
    });
  }
}

fn float_to_u32(f: f32, threshold: f32) -> u32 {
  let precisest = 1. / (threshold.max(f32::EPSILON));
  (f * precisest) as u32
}

impl VertexesKey {
  fn new(scale: f32, path: Path, style: PathStyle, threshold: f32) -> Self {
    Self { scale, path, style, threshold }
  }
}

impl PartialEq for VertexesKey {
  fn eq(&self, other: &Self) -> bool {
    self.scale == other.scale && self.style == other.style && self.path.iter().eq(other.path.iter())
  }
}

impl Eq for VertexesKey {}

impl Hash for VertexesKey {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    type LyonPoint = lyon_tessellation::geom::Point<f32>;
    fn event_hash<H>(index: usize, points: &[LyonPoint], state: &mut H)
    where
      H: std::hash::Hasher,
    {
      index.hash(state);
      points.iter().for_each(|p| {
        let bytes: [u32; 2] = unsafe { std::mem::transmute(*p) };
        bytes.hash(state);
      });
    }

    let Self { scale, threshold, path, style } = self;
    let scale = float_to_u32(*scale, *threshold);
    scale.hash(state);

    match style {
      PathStyle::Fill => 1.hash(state),
      PathStyle::Stroke(line_width) => {
        2.hash(state);
        float_to_u32(*line_width, *threshold).hash(state)
      }
    }

    path.iter().for_each(|e| match e {
      path::Event::Begin { at } => event_hash(1, &[at], state),
      path::Event::Line { from, to } => event_hash(2, &[from, to], state),
      path::Event::Quadratic { from, ctrl, to } => event_hash(3, &[from, ctrl, to], state),
      path::Event::Cubic { from, ctrl1, ctrl2, to } => {
        event_hash(4, &[from, ctrl1, ctrl2, to], state)
      }
      path::Event::End { last, first, close } => {
        event_hash(5, &[first, last], state);
        close.hash(state)
      }
    });
  }
}

impl TessData {
  fn is_color_buffer(&self) -> bool { matches!(&self.primitives, PrimitiveVec::Color(_)) }

  fn submit_tex_vertexes<F: FnMut(RenderData)>(&mut self, f: &mut F, texture: Texture) {
    if self.primitives.is_empty() {
      return;
    }

    let primitives = match &self.primitives {
      PrimitiveVec::Color(_) => unreachable!("Try to submit texture vertexes with a color buffer"),
      PrimitiveVec::Texture(p) => &**p,
    };

    f(RenderData::Image(TextureRenderData {
      vertices: &self.vertices,
      indices: &self.indices,
      primitives,
      texture,
    }));
    self.clear_vertexes();
  }

  fn submit_color_vertexes<F: FnMut(RenderData)>(&mut self, f: &mut F) {
    if self.primitives.is_empty() {
      return;
    }

    let primitives = match &self.primitives {
      PrimitiveVec::Color(c) => &**c,
      PrimitiveVec::Texture(_) => {
        unreachable!("Try to submit texture vertexes with a color buffer")
      }
    };

    f(RenderData::Color(ColorRenderData {
      vertices: &self.vertices,
      indices: &self.indices,
      primitives,
    }));
    self.clear_vertexes();
  }

  /// Generate vertexes for the commands as much as possible. Return how many
  /// commands processed.
  ///
  /// Caller also should guarantee the cache pointe is valid.
  unsafe fn fill_vertexes(&mut self, commands: &[PaintCommand], atlas: &TextureAtlas) -> usize {
    let mut cache_from = 0;
    let count = commands
      .iter()
      .map_while(|cmd| {
        self
          .push_primitive_to_buffer(cmd, atlas)
          .and_then(|prim_id| {
            cache_from = self.fill_a_command_vertexes(cache_from, prim_id);
            (self.indices.len() < self.vertex_batch_limit).then(|| ())
          })
      })
      .count();

    self.cache_list.drain(..cache_from);
    count
  }

  // return if the primitive index if it's can batch with existed primitives in
  // buffer.
  fn push_primitive_to_buffer(&mut self, cmd: &PaintCommand, atlas: &TextureAtlas) -> Option<u32> {
    if self.primitives.is_empty() {
      match &cmd.brush {
        Brush::Color(_) => self.convert_to_color_primitive(),
        Brush::Image { .. } => self.convert_to_texture_primitive(),
        Brush::Gradient => unimplemented!(),
      };
    }

    match (&cmd.brush, &mut self.primitives) {
      (Brush::Color(color), PrimitiveVec::Color(primitives)) => {
        primitives.push(ColorPrimitive {
          color: color.clone().into_arrays(),
          transform: cmd.transform.clone().to_arrays(),
        });
        Some(primitives.len() as u32 - 1)
      }
      (Brush::Image { img, tile_mode }, PrimitiveVec::Texture(primitives)) => {
        let alloc = atlas
          .no_cache_hit_find(img)
          .expect("Should store in atlas before fill vertexes.");

        let (mut x_base, mut y_base) = img.size().to_f32().to_tuple();
        if tile_mode.is_cover_mode() {
          let box_rect = cmd.box_rect_without_transform();
          if tile_mode.contains(TileMode::COVER_X) {
            x_base = box_rect.width();
          }
          if tile_mode.contains(TileMode::COVER_Y) {
            y_base = box_rect.height()
          }
        }
        let primitive = TexturePrimitive {
          tex_offset: alloc.rectangle.min.to_u32().to_array(),
          factor: [1. / x_base, 1. / y_base],
          transform: cmd.transform.clone().to_arrays(),
        };

        primitives.push(primitive);
        Some(primitives.len() as u32 - 1)
      }
      _ => None,
    }
  }

  /// Fill a command vertex from the index `from` of cache list. return the next
  /// command cache index
  unsafe fn fill_a_command_vertexes(&mut self, from: usize, prim_id: u32) -> usize {
    let mut idx = from;
    let cmd_idx = self.cache_list[idx].cmd_idx;

    while cmd_idx == self.cache_list[idx].cmd_idx {
      let cache = &mut *self.cache_list[idx].cache_ptr;
      if let Some(offset) = cache.indices_offset {
        self
          .indices
          .extend(cache.indices.iter().map(|i| i + offset))
      } else {
        cache.indices_offset = Some(self.vertices.len() as u32);
        self.vertices.extend(
          cache
            .vertexes
            .iter()
            .map(|pos| Vertex { pixel_coords: *pos, prim_id }),
        );
        self.indices.extend_from_slice(&cache.indices);
      }

      idx += 1;
    }
    idx
  }

  fn convert_to_color_primitive(&mut self) {
    match &mut self.primitives {
      PrimitiveVec::Color(_) => {}
      PrimitiveVec::Texture(t) => {
        assert!(t.is_empty());
        use std::mem::{size_of, transmute};
        assert_eq!(size_of::<ColorPrimitive>(), size_of::<TexturePrimitive>());

        let vec = std::mem::take(t);
        self.primitives = PrimitiveVec::Color(unsafe { transmute(vec) });
      }
    }
  }

  fn convert_to_texture_primitive(&mut self) {
    match &mut self.primitives {
      PrimitiveVec::Color(c) => {
        assert!(c.is_empty());
        use std::mem::{size_of, transmute};
        assert_eq!(size_of::<ColorPrimitive>(), size_of::<TexturePrimitive>());

        let vec = std::mem::take(c);
        self.primitives = PrimitiveVec::Texture(unsafe { transmute(vec) });
      }
      PrimitiveVec::Texture(_) => {}
    }
  }

  fn clear_vertexes(&mut self) {
    self.vertices.clear();
    self.indices.clear();
    self.primitives.clear();
  }
}

impl PrimitiveVec {
  fn is_empty(&self) -> bool {
    match self {
      PrimitiveVec::Color(c) => c.is_empty(),
      PrimitiveVec::Texture(i) => i.is_empty(),
    }
  }

  fn clear(&mut self) {
    match self {
      PrimitiveVec::Color(c) => c.clear(),
      PrimitiveVec::Texture(i) => i.clear(),
    }
  }
}

impl Default for PrimitiveVec {
  fn default() -> Self { PrimitiveVec::Color(vec![]) }
}

#[cfg(test)]
mod tests {
  use crate::RenderData;
  use lyon_tessellation::path::traits::{Build, PathBuilder};
  use painter::{Color, DeviceSize, Painter, Point, Radius, Rect, Size, Transform};
  extern crate test;
  use test::Bencher;

  use super::{atlas::tests::PureColorImage, *};

  fn tessellator() -> Tessellator {
    Tessellator::new(DeviceSize::new(128, 128), DeviceSize::new(512, 512), 0.01)
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
    let mut painter = Painter::new(Transform::default());
    circle_rectangle_color_paint(&mut painter);
    let mut render_data = vec![];
    tess.tessellate(painter.finish(), |r| match r {
      RenderData::Color(_) => render_data.push(true),
      RenderData::Image(_) => render_data.push(false),
    });

    assert_eq!(&render_data, &[true]);
  }

  #[test]
  fn img_should_batch() {
    let mut tess = tessellator();
    let mut painter = Painter::new(Transform::default());
    two_img_paint(&mut painter);
    let mut render_data = vec![];
    tess.tessellate(painter.finish(), |r| match r {
      RenderData::Color(_) => render_data.push(true),
      RenderData::Image(_) => render_data.push(false),
    });

    assert_eq!(&render_data, &[false]);
  }

  #[test]
  fn image_color_cannot_batch() {
    let mut tess = tessellator();
    let mut painter = Painter::new(Transform::default());
    circle_rectangle_color_paint(&mut painter);
    two_img_paint(&mut painter);
    circle_rectangle_color_paint(&mut painter);
    circle_rectangle_color_paint(&mut painter);
    two_img_paint(&mut painter);

    let mut render_data = vec![];
    tess.tessellate(painter.finish(), |r| match r {
      RenderData::Color(_) => render_data.push(true),
      RenderData::Image(_) => render_data.push(false),
    });

    assert_eq!(&render_data, &[true, false, true, false]);
  }

  #[test]
  fn large_image_cannot_batch() {
    let mut tess = tessellator();
    let mut painter = Painter::new(Transform::default());

    two_img_paint(&mut painter);
    let large_img = PureColorImage::shallow_img(Color::YELLOW, DeviceSize::new(1024, 1024));
    painter.set_brush(Brush::Image {
      img: large_img,
      tile_mode: TileMode::REPEAT_BOTH,
    });
    painter.rect(&Rect::new(Point::new(0., 0.), Size::new(512., 512.)));
    painter.fill(None);
    two_img_paint(&mut painter);

    let mut render_data = vec![];
    tess.tessellate(painter.finish(), |r| match r {
      RenderData::Color(_) => render_data.push(true),
      RenderData::Image(_) => render_data.push(false),
    });

    assert_eq!(&render_data, &[false, false, false]);
  }

  #[bench]
  fn million_diff_round_rect(b: &mut Bencher) {
    let mut painter = Painter::new(Transform::default());
    painter.set_brush(Color::RED).set_line_width(2.);
    (0..1_000_000).for_each(|i| {
      painter.rect_round(
        &Rect::new(Point::zero(), Size::new(i as f32 + 1., i as f32 + 1.)),
        &Radius::all(i as f32 / 10.),
      );
      if 1 % 2 == 0 {
        painter.stroke(None, None);
      } else {
        painter.fill(None);
      }
    });
    let commands = painter.finish();
    b.iter(|| {
      let mut tess = tessellator();
      tess.tessellate(commands.clone(), |_| {})
    })
  }

  #[bench]
  fn million_same_round_rect(b: &mut Bencher) {
    let mut painter = Painter::new(Transform::default());
    painter.set_brush(Color::RED).set_line_width(2.);
    painter.rect_round(
      &Rect::new(Point::zero(), Size::new(100., 100.)),
      &Radius::all(2.),
    );
    painter.fill(None);
    let cmd = painter.finish().pop().unwrap();
    let commands = vec![cmd; 1_000_000];
    b.iter(|| {
      let mut tess = tessellator();
      tess.tessellate(commands.clone(), |_| {})
    })
  }

  #[bench]
  fn million_char(b: &mut Bencher) {
    let mut painter = Painter::new(Transform::default());
    painter.set_brush(Color::RED).set_line_width(2.);
    // 30k different char
    let text = include_str!("../fonts/loads-of-unicode.txt");
    (0..34).for_each(|_| {
      painter.fill_text(text);
    });
    let commands = painter.finish();
    b.iter(|| {
      let mut tess = tessellator();
      tess.tessellate(commands.clone(), |_| {})
    })
  }
}
