mod fonts;
use super::{canvas, surface::Surface, Canvas, DeviceSize, Rect};
pub use fonts::*;
use glyph_brush::{BrushAction, BrushError, FontId, GlyphBrush, GlyphBrushBuilder, GlyphCruncher};
use log::{log_enabled, warn};
use std::{cell::Cell, rc::Rc, sync::Arc};

#[derive(Debug, Default, Clone)]
pub(crate) struct GlyphStatistics(Rc<Cell<u32>>);

pub(crate) type Section<'a> = glyph_brush::Section<'a, GlyphStatistics>;
pub(crate) const DEFAULT_FONT_FAMILY: &str = "serif";

const INIT_SIZE: DeviceSize = DeviceSize::new(512, 512);

#[derive(Debug, Clone)]
pub(crate) struct QuadVertex {
  pub(crate) pixel_coords: Rect,
  pub(crate) tex_coords: Rect,
}

pub(crate) struct TextBrush {
  texture: wgpu::Texture,
  view: wgpu::TextureView,
  quad_vertices_cache: Vec<QuadVertex>,
  fonts: Fonts,
  brush: GlyphBrush<QuadVertex, GlyphStatistics>,
}

impl TextBrush {
  pub(crate) fn new(device: &wgpu::Device) -> Self {
    let brush = GlyphBrushBuilder::using_fonts(vec![])
      .initial_cache_size((INIT_SIZE.width, INIT_SIZE.height))
      .build();

    let texture = Self::texture(device, INIT_SIZE);
    TextBrush {
      brush,
      view: texture.create_default_view(),
      texture,
      quad_vertices_cache: vec![],
      fonts: Fonts::new(),
    }
  }

  #[inline]
  pub(crate) fn view(&self) -> &wgpu::TextureView { &self.view }

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

  #[inline]
  pub(crate) fn section_bounds(&mut self, sec: &Section) -> Option<Rect> {
    self
      .brush
      .glyph_bounds(sec)
      .map(|rect| euclid::rect(rect.min.x, rect.min.y, rect.width(), rect.height()))
  }

  #[inline]
  fn queue(&mut self, section: &Section) { self.brush.queue(section); }

  fn process_queued(
    &mut self,
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
  ) -> Result<&[QuadVertex], BrushError> {
    let Self { brush, texture, .. } = self;
    let action = brush.process_queued(
      |rect, data| Self::update_texture(texture, device, encoder, rect, data),
      Self::to_vertex,
    )?;
    match action {
      BrushAction::Draw(vertices) => self.quad_vertices_cache = vertices,
      BrushAction::ReDraw => {}
    }

    Ok(self.quad_vertices_cache.as_slice())
  }

  pub(crate) fn resize_texture(&mut self, device: &wgpu::Device, suggested: (u32, u32)) {
    const MAX: u32 = canvas::surface::Texture::MAX_DIMENSION;
    let new_size = if suggested.0 >= MAX || suggested.1 >= MAX {
      (MAX, MAX)
    } else {
      suggested
    };

    self.texture = Self::texture(device, new_size.into());
    self.view = self.texture.create_default_view();
    if log_enabled!(log::Level::Warn) {
      warn!(
        "Increasing glyph texture size {old:?} -> {new:?}. \
             Consider building with `.initial_cache_size({new:?})` to avoid \
             resizing. Called from:\n{trace:?}",
        old = self.brush.texture_dimensions(),
        new = new_size,
        trace = backtrace::Backtrace::new()
      );
    }
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

  fn update_texture(
    texture: &wgpu::Texture,
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    rect: glyph_brush::Rectangle<u32>,
    tex_data: &[u8],
  ) {
    let buffer = device.create_buffer_with_data(tex_data, wgpu::BufferUsage::COPY_SRC);

    encoder.copy_buffer_to_texture(
      wgpu::BufferCopyView {
        buffer: &buffer,
        layout: wgpu::TextureDataLayout {
          offset: 0,
          bytes_per_row: rect.width(),
          rows_per_image: rect.height(),
        },
      },
      wgpu::TextureCopyView {
        texture,
        mip_level: 0,
        origin: wgpu::Origin3d {
          x: rect.min[0],
          y: rect.min[1],
          z: 0,
        },
      },
      wgpu::Extent3d {
        width: rect.width(),
        height: rect.height(),
        depth: 1,
      },
    )
  }

  fn to_vertex(
    glyph_brush::GlyphVertex {
      mut tex_coords,
      mut pixel_coords,
      bounds,
      extra,
    }: glyph_brush::GlyphVertex<GlyphStatistics>,
  ) -> QuadVertex {
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

    QuadVertex {
      pixel_coords: euclid::rect(
        pixel_coords.min.x,
        pixel_coords.min.y,
        pixel_coords.width(),
        pixel_coords.height(),
      ),
      tex_coords: euclid::rect(
        tex_coords.min.x,
        tex_coords.min.y,
        tex_coords.width(),
        tex_coords.height(),
      ),
    }
  }

  fn texture(device: &wgpu::Device, size: DeviceSize) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
      label: Some("new texture"),
      size: wgpu::Extent3d {
        width: size.width,
        height: size.height,
        depth: 1,
      },
      dimension: wgpu::TextureDimension::D2,
      format: wgpu::TextureFormat::R8Unorm,
      usage: wgpu::TextureUsage::COPY_DST
        | wgpu::TextureUsage::SAMPLED
        | wgpu::TextureUsage::COPY_SRC,
      mip_level_count: 1,
      sample_count: 1,
    })
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

  pub(crate) fn process_queued(&mut self) -> (&[QuadVertex], bool) {
    let mut texture_updated = false;

    loop {
      self.ensure_encoder_exist();

      let encoder = self.encoder.as_mut().unwrap();

      match self.glyph_brush.process_queued(&self.device, encoder) {
        Ok(_) => break,
        Err(glyph_brush::BrushError::TextureTooSmall { suggested }) => {
          self.submit();
          self.glyph_brush.resize_texture(&self.device, suggested);

          texture_updated = true;
        }
      };
    }

    if texture_updated {
      self.update_uniforms();
    }
    let quad_vertices = self.glyph_brush.quad_vertices_cache.as_slice();
    (quad_vertices, texture_updated)
  }

  #[cfg(debug_assertions)]
  pub fn log_glyph_texture(&mut self) {
    self.ensure_rgba_converter();

    let Canvas { device, queue, .. } = self;

    let pkg_root = env!("CARGO_MANIFEST_DIR");
    let atlas_capture = format!("{}/.log/{}", pkg_root, "glyph_texture_cache.png");

    let (width, height) = self.glyph_brush.brush.texture_dimensions();

    let size = width as u64 * height as u64 * std::mem::size_of::<u8>() as u64;

    // The output buffer lets us retrieve the data as an array
    let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
      size,
      usage: wgpu::BufferUsage::MAP_READ | wgpu::BufferUsage::STORAGE | wgpu::BufferUsage::COPY_DST,
      mapped_at_creation: false,
      label: None,
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
      label: Some("Encoder for encoding texture as png"),
    });

    encoder.copy_texture_to_buffer(
      wgpu::TextureCopyView {
        texture: &self.glyph_brush.texture,
        mip_level: 0,
        origin: wgpu::Origin3d::ZERO,
      },
      wgpu::BufferCopyView {
        buffer: &output_buffer,
        layout: wgpu::TextureDataLayout {
          offset: 0,
          bytes_per_row: std::mem::size_of::<u32>() as u32 * width as u32,
          rows_per_image: 0,
        },
      },
      wgpu::Extent3d {
        width,
        height,
        depth: 1,
      },
    );

    queue.submit(Some(encoder.finish()));

    // Note that we're not calling `.await` here.
    let buffer_future = output_buffer.map_async(wgpu::MapMode::Read, 0, wgpu::BufferSize(size));

    // Poll the device in a blocking manner so that our future resolves.
    device.poll(wgpu::Maintain::Wait);

    let data = output_buffer.get_mapped_range(0, wgpu::BufferSize::WHOLE);
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
      .write_image_data(data)
      .unwrap();

    log::debug!("Write a image of canvas atlas at: {}", &atlas_capture);
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
  #[ignore = "gpu need"]
  fn custom_fonts_use() {
    let mut canvas = block_on(Canvas::new(DeviceSize::new(400, 400)));

    let deja = include_bytes!("../fonts/DejaVuSans.ttf");
    canvas.load_font_from_bytes(deja.to_vec(), 0).unwrap();
    let crate_root = env!("CARGO_MANIFEST_DIR").to_owned();
    canvas
      .load_font_from_path(crate_root + "/fonts/GaramondNo8-Reg.ttf", 0)
      .unwrap();

    let brush = &mut canvas.glyph_brush;

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
    let (vertices, update_texture) = canvas.process_queued();

    assert_eq!(vertices.len(), str.chars().count());
    assert_eq!(update_texture, false);

    // force submit data
    if let Some(encoder) = canvas.encoder.take() {
      canvas.queue.submit(Some(encoder.finish()));
    }
    canvas.view.take();

    canvas.log_glyph_texture();

    unit_test::assert_img_eq!(
      "./test_imgs/hello_glyph_cache.png",
      "./.log/glyph_texture_cache.png"
    );
  }
}
