mod fonts;
use super::{array_2d::Array2D, canvas, surface::Surface, Canvas, DeviceSize, Point, Rect, Vertex};
pub use fonts::*;
use glyph_brush::{BrushAction, BrushError, FontId, GlyphBrush, GlyphBrushBuilder, GlyphCruncher};
use lyon::tessellation::VertexBuffers;
use std::{cell::Cell, rc::Rc, sync::Arc};

#[derive(Debug, Default, Clone)]
pub(crate) struct GlyphStatistics(Rc<Cell<u32>>);

pub(crate) type Section<'a> = glyph_brush::Section<'a, GlyphStatistics>;
pub(crate) const DEFAULT_FONT_FAMILY: &str = "serif";

const INIT_SIZE: DeviceSize = DeviceSize::new(1024, 1024);

pub(crate) struct TextBrush {
  texture: Array2D<u8>,
  texture_updated: bool,
  texture_resized: bool,
  quad_vertices_cache: Vec<[Vertex; 4]>,
  fonts: Fonts,
  brush: GlyphBrush<[Vertex; 4], GlyphStatistics>,
}

impl TextBrush {
  pub fn new() -> Self {
    let brush = GlyphBrushBuilder::using_fonts(vec![])
      .initial_cache_size((INIT_SIZE.width, INIT_SIZE.height))
      .build();

    TextBrush {
      texture_updated: false,
      texture_resized: false,
      brush,
      quad_vertices_cache: vec![],
      fonts: Fonts::new(),
      texture: Array2D::fill_from(INIT_SIZE.width, INIT_SIZE.height, 0),
    }
  }

  // #[inline]
  // pub(crate) fn glyphs(&mut self, sec: &Section) ->
  // glyph_brush::SectionGlyphIter {   self.brush.glyphs(sec)
  // }

  // pub(crate) fn glyph_bounds(
  //   &self,
  //   glyph: &glyph_brush::SectionGlyph,
  // ) -> euclid::Box2D<f32, LogicUnit> {
  //   use glyph_brush::ab_glyph::Font as BrushFont;
  //   let font = &self.brush.fonts()[glyph.font_id];
  //   let rect = font.glyph_bounds(&glyph.glyph);

  //   euclid::Box2D::new(
  //     Point::new(rect.min.x, rect.min.y),
  //     Point::new(rect.max.x, rect.max.y),
  //   )
  // }

  pub fn section_bounds(&mut self, sec: &Section) -> Option<Rect> {
    self
      .brush
      .glyph_bounds(sec)
      .map(|rect| euclid::rect(rect.min.x, rect.min.y, rect.width(), rect.height()))
  }

  #[inline]
  pub fn texture_size(&self) -> DeviceSize { self.texture.size() }

  #[inline]
  fn queue(&mut self, section: &Section) { self.brush.queue(section); }

  fn process_queued(&mut self) -> Result<&[[Vertex; 4]], BrushError> {
    let Self {
      brush,
      texture,
      texture_updated,
      ..
    } = self;
    let action = brush.process_queued(
      |rect, data| {
        texture.copy_from_slice(rect.min[1], rect.min[0], rect.width(), data);
        *texture_updated = true;
      },
      Self::to_vertex,
    )?;
    match action {
      BrushAction::Draw(vertices) => self.quad_vertices_cache = vertices,
      BrushAction::ReDraw => {}
    }

    Ok(self.quad_vertices_cache.as_slice())
  }

  fn resize_texture(&mut self, suggested: (u32, u32)) {
    self.texture_resized = true;
    const MAX: u32 = canvas::surface::Texture::MAX_DIMENSION;
    let new_size = if suggested.0 >= MAX || suggested.1 >= MAX {
      (MAX, MAX)
    } else {
      suggested
    };
    self.texture = Array2D::fill_from(new_size.1, new_size.0, 0);
    self.brush.resize_texture(new_size.0, new_size.1)
  }

  #[inline]
  fn load_font_from_bytes(
    &mut self,
    font_data: Vec<u8>,
    font_index: u32,
  ) -> Result<&Font, Box<dyn std::error::Error>> {
    self
      .fonts
      .load_from_bytes(Arc::new(font_data), font_index, &mut self.brush)
  }

  #[inline]
  fn load_font_from_path<P: AsRef<std::path::Path>>(
    &mut self,
    path: P,
    font_index: u32,
  ) -> Result<&Font, Box<dyn std::error::Error>> {
    self.fonts.load_from_path(path, font_index, &mut self.brush)
  }

  #[inline]
  fn select_best_match(
    &mut self,
    family_names: &str,
    props: &FontProperties,
  ) -> Result<&Font, Box<dyn std::error::Error>> {
    self
      .fonts
      .select_best_match(family_names, props, &mut self.brush)
  }

  pub fn flush_cache(
    &mut self,
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    texture: &wgpu::Texture,
  ) {
    if self.texture_updated {
      self.texture_updated = false;

      let buffer = device.create_buffer_with_data(self.texture.data(), wgpu::BufferUsage::COPY_SRC);

      let DeviceSize { width, height, .. } = self.texture.size();
      encoder.copy_buffer_to_texture(
        wgpu::BufferCopyView {
          buffer: &buffer,
          layout: wgpu::TextureDataLayout {
            offset: 0,
            bytes_per_row: width,
            rows_per_image: height,
          },
        },
        wgpu::TextureCopyView {
          texture,
          mip_level: 0,
          origin: wgpu::Origin3d::ZERO,
        },
        wgpu::Extent3d {
          width,
          height,
          depth: 1,
        },
      )
    }
  }

  fn to_vertex(
    glyph_brush::GlyphVertex {
      mut tex_coords,
      mut pixel_coords,
      bounds,
      extra,
    }: glyph_brush::GlyphVertex<GlyphStatistics>,
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
    extra.0.set(extra.0.get() + 1);

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
        pixel_coords: Point::new(px_min.x, px_min.y),
        texture_coords: Point::new(tx_min.x, tx_min.y),
      },
      Vertex {
        pixel_coords: Point::new(px_max.x, px_min.y),
        texture_coords: Point::new(tx_max.x, tx_min.y),
      },
      Vertex {
        pixel_coords: Point::new(px_min.x, px_max.y),
        texture_coords: Point::new(tx_min.x, tx_max.y),
      },
      Vertex {
        pixel_coords: Point::new(px_max.x, px_max.y),
        texture_coords: Point::new(tx_max.x, tx_max.y),
      },
    ]
  }
}

impl<S: Surface> Canvas<S> {
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
    self.glyph_brush.load_font_from_bytes(font_data, font_index)
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
    self.glyph_brush.load_font_from_path(path, font_index)
  }

  /// Performs font matching according to the CSS Fonts Level 3 specification
  /// and returns matched fonts.
  pub fn select_best_match(
    &mut self,
    family_names: &str,
    props: &FontProperties,
  ) -> Result<&Font, Box<dyn std::error::Error>> {
    self.glyph_brush.select_best_match(family_names, props)
  }

  pub(crate) fn default_font(&mut self) -> &Font {
    self
      .glyph_brush
      .select_best_match(DEFAULT_FONT_FAMILY, &FontProperties::default())
      .expect("Canvas default font not exist!")
  }

  #[inline]
  pub(crate) fn queue(&mut self, section: &Section) { self.glyph_brush.queue(&section); }

  /// Processes all queued texts, and push the vertices and indices into the
  /// buffer, return true if the texture has updated.
  pub(crate) fn process_queued(&mut self, buffer: &mut VertexBuffers<Vertex, u32>) {
    loop {
      self.ensure_encoder_exist();

      match self.glyph_brush.process_queued() {
        Ok(_) => break,
        Err(glyph_brush::BrushError::TextureTooSmall { suggested }) => {
          self.submit();
          self.glyph_brush.resize_texture(suggested);
        }
      };
    }

    if self.glyph_brush.texture_resized {
      self.glyph_brush.texture_resized = false;
      self.resize_glyph_texture();
      self.update_uniforms();
    }

    let quad_vertices = &self.glyph_brush.quad_vertices_cache;
    let VertexBuffers { indices, vertices } = buffer;
    vertices.reserve(quad_vertices.len() * 4);
    indices.reserve(quad_vertices.len() * 6);

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
  }

  #[cfg(debug_assertions)]
  pub fn log_glyph_texture(&mut self) {
    let Canvas { glyph_brush, .. } = self;

    let pkg_root = env!("CARGO_MANIFEST_DIR");
    let atlas_capture = format!("{}/.log/{}", pkg_root, "glyph_texture_cache.png");

    let DeviceSize { width, height, .. } = glyph_brush.texture_size();

    let mut png_encoder = png::Encoder::new(
      std::fs::File::create(&atlas_capture).unwrap(),
      width,
      height,
    );
    png_encoder.set_depth(png::BitDepth::Eight);
    png_encoder.set_color(png::ColorType::Grayscale);
    png_encoder
      .write_header()
      .unwrap()
      .write_image_data(glyph_brush.texture.data())
      .unwrap();

    log::debug!(
      "Write a image of canvas glyphs texture at: {}",
      &atlas_capture
    );
  }
}

impl From<GlyphStatistics> for lyon::tessellation::Count {
  fn from(g: GlyphStatistics) -> Self {
    let glyph_count = g.0.get();
    Self {
      vertices: glyph_count * 4,
      indices: glyph_count * 6,
    }
  }
}

// GlyphStatistics as extra data for `Section` just use to count glyphs, not
// effect on sections content.
mod no_effect {
  use super::*;
  impl std::cmp::PartialEq for GlyphStatistics {
    #[inline]
    fn eq(&self, _: &Self) -> bool { true }
  }

  impl std::hash::Hash for GlyphStatistics {
    #[inline]
    fn hash<H: std::hash::Hasher>(&self, _: &mut H) {}
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::canvas::*;
  use futures::executor::block_on;
  use glyph_brush::Text;

  fn add_default_fonts<S: Surface>(brush: &mut Canvas<S>) {
    brush
      .load_font_from_bytes(include_bytes!("../fonts/DejaVuSans.ttf").to_vec(), 0)
      .unwrap();
    brush
      .load_font_from_bytes(include_bytes!("../fonts/GaramondNo8-Reg.ttf").to_vec(), 0)
      .unwrap();
  }
  #[test]
  fn custom_fonts_use() {
    let mut brush = TextBrush::new();

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
  #[ignore = "gpu need"]
  fn glyph_cache_check() {
    let mut canvas = block_on(Canvas::new(DeviceSize::new(400, 400)));
    add_default_fonts(&mut canvas);
    let str = "Hello_glyph!";
    let section = Section::new().add_text(Text::default().with_text(str));
    canvas.queue(&section);

    let mut buffer = VertexBuffers::new();
    let update_texture = canvas.process_queued(&mut buffer);

    assert_eq!(buffer.vertices.len(), str.chars().count() * 4);
    assert_eq!(update_texture, false);

    // force submit data
    if let Some(encoder) = canvas.encoder.take() {
      canvas.queue.submit(Some(encoder.finish()));
    }
    canvas.view.take();

    #[cfg(debug_assertions)]
    canvas.log_glyph_texture();

    unit_test::assert_img_eq!(
      "./test_imgs/hello_glyph_cache.png",
      "./.log/glyph_texture_cache.png"
    );
  }

  extern crate test;
  use test::Bencher;

  #[bench]
  #[ignore = "gpu need"]
  fn generate_vertices(b: &mut Bencher) {
    let mut canvas = block_on(Canvas::new(DeviceSize::new(800, 800)));
    let _ = canvas.select_best_match("Times New Roman", &FontProperties::default());
    let text = include_str!("../fonts/loads-of-unicode.txt");
    let sec = Section::new().add_text(glyph_brush::Text::default().with_text(text));
    b.iter(|| {
      let mut buffer = VertexBuffers::new();
      canvas.queue(&sec);
      canvas.process_queued(&mut buffer);
    })
  }
}
