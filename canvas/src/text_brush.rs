mod fonts;
use super::{canvas::Vertex, mem_texture::MemTexture, DeviceSize, Rect};
pub use fonts::*;
use glyph_brush::{BrushAction, BrushError, FontId, GlyphBrush, GlyphBrushBuilder, GlyphCruncher};
use lyon::tessellation::{Count, VertexBuffers};
use std::sync::Arc;

pub(crate) type Section<'a> = glyph_brush::Section<'a, u32>;
pub(crate) const DEFAULT_FONT_FAMILY: &str = "serif";

pub struct TextBrush {
  texture: MemTexture<u8>,
  quad_vertices_cache: Vec<[Vertex; 4]>,
  fonts: Fonts,
  brush: GlyphBrush<[Vertex; 4], u32>,
}

impl TextBrush {
  pub(crate) fn new(init_size: DeviceSize, max_size: DeviceSize) -> Self {
    let brush = GlyphBrushBuilder::using_fonts(vec![])
      .initial_cache_size((init_size.width, init_size.height))
      .build();

    TextBrush {
      brush,
      quad_vertices_cache: vec![],
      fonts: Fonts::new(),
      texture: MemTexture::new(init_size, max_size),
    }
  }

  /// Add a custom font from bytes, so canvas support this font to draw text.
  /// If the data represents a collection (`.ttc`/`.otc`/etc.), `font_index`
  /// specifies the index of the font to load from it. If the data represents
  /// a single font, pass 0 for `font_index`.
  #[inline]
  pub fn load_font_from_bytes(
    &mut self,
    font_data: Vec<u8>,
    font_index: u32,
  ) -> Result<&Font, Box<dyn std::error::Error>> {
    self
      .fonts
      .load_from_bytes(Arc::new(font_data), font_index, &mut self.brush)
  }

  /// Loads a font from the path to a `.ttf`/`.otf`/etc. file.
  ///
  /// If the file is a collection (`.ttc`/`.otc`/etc.), `font_index` specifies
  /// the index of the font to load from it. If the file represents a single
  /// font, pass 0 for `font_index`.
  #[inline]
  pub fn load_font_from_path<P: AsRef<std::path::Path>>(
    &mut self,
    path: P,
    font_index: u32,
  ) -> Result<&Font, Box<dyn std::error::Error>> {
    self.fonts.load_from_path(path, font_index, &mut self.brush)
  }

  /// Performs font matching according to the CSS Fonts Level 3 specification
  /// and returns matched fonts.
  #[inline]
  pub fn select_best_match(
    &mut self,
    family_names: &str,
    props: &FontProperties,
  ) -> Result<&Font, Box<dyn std::error::Error>> {
    self
      .fonts
      .select_best_match(family_names, props, &mut self.brush)
  }

  #[inline]
  pub fn default_font(&mut self) -> &Font {
    self
      .select_best_match(DEFAULT_FONT_FAMILY, &FontProperties::default())
      .expect("Canvas default font not exist!")
  }

  #[inline]
  pub fn log_glyph_cache_png_to(&self, path: &str) {
    self.texture.log_png_to(path, png::ColorType::Grayscale);
  }

  pub(crate) fn section_bounds(&mut self, sec: &Section) -> Option<Rect> {
    self
      .brush
      .glyph_bounds(sec)
      .map(|rect| euclid::rect(rect.min.x, rect.min.y, rect.width(), rect.height()))
  }

  #[inline]
  pub(crate) fn texture(&self) -> &MemTexture<u8> { &self.texture }

  #[inline]
  pub(crate) fn texture_mut(&mut self) -> &mut MemTexture<u8> { &mut self.texture }

  #[inline]
  pub(crate) fn queue(&mut self, section: Section) { self.brush.queue(section); }

  /// Processes all queued texts, and push the vertices and indices into the
  /// buffer
  pub(crate) fn process_queued(
    &mut self,
    buffer: &mut VertexBuffers<Vertex, u32>,
  ) -> Result<lyon::tessellation::Count, BrushError> {
    self.try_process_queued()?;

    let quad_vertices = &self.quad_vertices_cache;
    let VertexBuffers { indices, vertices } = buffer;

    let count = glyph_vertices_count(quad_vertices.len() as u32);
    vertices.reserve(count.vertices as usize);
    indices.reserve(count.indices as usize);

    quad_vertices
      .iter()
      .fold(vertices.len() as u32, |state, _| {
        let tl = state;
        let tr = tl + 1;
        let bl = tl + 2;
        let br = tl + 3;
        indices.push(tl);
        indices.push(tr);
        indices.push(bl);
        indices.push(bl);
        indices.push(tr);
        indices.push(br);
        state + 4
      });

    unsafe {
      let count = quad_vertices.len() * 4;
      let len = vertices.len();
      std::ptr::copy_nonoverlapping(
        quad_vertices.as_ptr() as *const Vertex,
        vertices.as_mut_ptr().add(len),
        count,
      );
      vertices.set_len(len + count);
    }

    Ok(count)
  }

  pub(crate) fn grow_texture(&mut self) {
    if self.texture.expand_size(false) {
      let size = self.texture.size();
      self.brush.resize_texture(size.width, size.height);
    } else {
      log::error!(
        "The text cache buffer is overflow, too much texts to draw at once.
      Maybe you should split your single big text draw as many pieces to draw"
      );
    }
  }

  fn try_process_queued(&mut self) -> Result<(), BrushError> {
    let Self { brush, texture, .. } = self;
    let action = brush.process_queued(
      |rect, data| {
        let rect = euclid::Box2D::new(rect.min.into(), rect.max.into()).to_rect();
        texture.update_texture(&rect, data);
      },
      Self::convert_vertex,
    )?;
    match action {
      BrushAction::Draw(vertices) => self.quad_vertices_cache = vertices,
      BrushAction::ReDraw => {}
    }

    Ok(())
  }

  fn convert_vertex(
    glyph_brush::GlyphVertex {
      mut tex_coords,
      mut pixel_coords,
      bounds,
      extra,
    }: glyph_brush::GlyphVertex<u32>,
  ) -> [Vertex; 4] {
    // handle overlapping bounds, modify uv_rect to preserve texture aspect
    if pixel_coords.max.x > bounds.max.x {
      let old_width = pixel_coords.width();
      pixel_coords.max.x = bounds.max.x;
      tex_coords.max.x = tex_coords.min.x + tex_coords.width() * pixel_coords.width() / old_width;
    }

    if pixel_coords.min.x < bounds.min.x {
      let old_width = pixel_coords.width();
      pixel_coords.min.x = bounds.min.x;
      tex_coords.min.x = tex_coords.max.x - tex_coords.width() * pixel_coords.width() / old_width;
    }

    if pixel_coords.max.y > bounds.max.y {
      let old_height = pixel_coords.height();
      pixel_coords.max.y = bounds.max.y;
      tex_coords.max.y =
        tex_coords.min.y + tex_coords.height() * pixel_coords.height() / old_height;
    }

    if pixel_coords.min.y < bounds.min.y {
      let old_height = pixel_coords.height();
      pixel_coords.min.y = bounds.min.y;
      tex_coords.min.y =
        tex_coords.max.y - tex_coords.height() * pixel_coords.height() / old_height;
    }

    let glyph_brush::ab_glyph::Rect {
      min: px_min,
      max: px_max,
    } = pixel_coords;
    let glyph_brush::ab_glyph::Rect {
      min: tx_min,
      max: tx_max,
    } = tex_coords;

    [
      Vertex {
        pixel_coords: [px_min.x, px_min.y],
        texture_coords: [tx_min.x, tx_min.y],
        prim_id: *extra,
      },
      Vertex {
        pixel_coords: [px_max.x, px_min.y],
        texture_coords: [tx_max.x, tx_min.y],
        prim_id: *extra,
      },
      Vertex {
        pixel_coords: [px_min.x, px_max.y],
        texture_coords: [tx_min.x, tx_max.y],
        prim_id: *extra,
      },
      Vertex {
        pixel_coords: [px_max.x, px_max.y],
        texture_coords: [tx_max.x, tx_max.y],
        prim_id: *extra,
      },
    ]
  }
}

fn glyph_vertices_count(glyphs: u32) -> Count {
  Count {
    vertices: glyphs * 4,
    indices: glyphs * 6,
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use glyph_brush::Text;

  const INIT_SIZE: DeviceSize = DeviceSize::new(1024, 1024);
  const MAX_SIZE: DeviceSize = DeviceSize::new(4096, 4096);

  fn add_default_fonts(brush: &mut TextBrush) {
    brush
      .load_font_from_bytes(include_bytes!("../fonts/DejaVuSans.ttf").to_vec(), 0)
      .unwrap();
    brush
      .load_font_from_bytes(include_bytes!("../fonts/GaramondNo8-Reg.ttf").to_vec(), 0)
      .unwrap();
  }
  #[test]
  fn custom_fonts_use() {
    let mut brush = TextBrush::new(INIT_SIZE, MAX_SIZE);

    let deja = include_bytes!("../fonts/DejaVuSans.ttf");
    brush.load_font_from_bytes(deja.to_vec(), 0).unwrap();
    let crate_root = env!("CARGO_MANIFEST_DIR").to_owned();
    brush
      .load_font_from_path(crate_root + "/fonts/GaramondNo8-Reg.ttf", 0)
      .unwrap();

    let font = brush.select_best_match("DejaVu Sans", &FontProperties::default());
    assert!(font.is_ok());

    let font = brush.select_best_match("GaramondNo8", &FontProperties::default());
    assert!(font.is_ok());
  }

  #[test]
  fn glyph_cache_check() {
    let mut brush = TextBrush::new(INIT_SIZE, MAX_SIZE);
    brush.texture_mut().data_synced();

    add_default_fonts(&mut brush);
    let str = "Hello_glyph!";
    let section = Section::new().add_text(Text::default().with_text(str));
    brush.queue(section);

    let mut buffer = VertexBuffers::new();
    brush.process_queued(&mut buffer).unwrap();

    assert_eq!(buffer.vertices.len(), str.chars().count() * 4);
    assert_eq!(brush.texture().is_updated(), true);
    assert_eq!(brush.texture().is_resized(), false);

    brush.log_glyph_cache_png_to("glyph_texture_cache.png");

    unit_test::assert_img_eq!(
      "../test_imgs/hello_glyph_cache.png",
      "../.log/glyph_texture_cache.png"
    );
  }

  extern crate test;
  use test::Bencher;

  #[bench]
  fn generate_vertices(b: &mut Bencher) {
    let mut brush = TextBrush::new(INIT_SIZE, MAX_SIZE);

    let _ = brush.select_best_match("DejaVu Serif, Arial", &FontProperties::default());
    let text = include_str!("../fonts/loads-of-unicode.txt");
    let sec = Section::new().add_text(glyph_brush::Text::default().with_text(text));
    b.iter(|| {
      let mut buffer = VertexBuffers::new();
      brush.queue(sec.clone());
      while brush.process_queued(&mut buffer).is_err() {
        brush.grow_texture();
      }
    })
  }
}
