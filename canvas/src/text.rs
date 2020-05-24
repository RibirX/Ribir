use super::{canvas, surface::Surface, Canvas, DeviceSize, LogicUnit, Point, Rect};
use glyph_brush::{
  ab_glyph::FontArc, BrushAction, BrushError, FontId, GlyphBrush, GlyphBrushBuilder, GlyphCruncher,
};
use log::{log_enabled, warn};

pub(crate) type Section<'a> = glyph_brush::Section<'a, ()>;

const INIT_SIZE: DeviceSize = DeviceSize::new(512, 512);

#[derive(Debug, Clone)]
pub(crate) struct QuadVertex {
  pub(crate) pixel_coords: Rect,
  pub(crate) tex_coords: Rect,
}

pub(crate) struct TextBrush {
  brush: GlyphBrush<QuadVertex, ()>,
  texture: wgpu::Texture,
  view: wgpu::TextureView,
  quad_vertices_cache: Vec<QuadVertex>,
  available_fonts: std::collections::HashMap<String, FontId>,
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
      available_fonts: Default::default(),
    }
  }

  #[inline]
  pub(crate) fn view(&self) -> &wgpu::TextureView { &self.view }

  #[inline]
  pub(crate) fn glyphs(&mut self, sec: &Section) -> glyph_brush::SectionGlyphIter {
    self.brush.glyphs(sec)
  }

  #[inline]
  pub(crate) fn glyph_bounds(&mut self, sec: &Section) -> Option<Rect> {
    self
      .brush
      .glyph_bounds(sec)
      .map(|rect| euclid::rect(rect.min.x, rect.min.y, rect.width(), rect.height()))
  }

  #[inline]
  pub(crate) fn draw_rect_for_cache(
    &self,
    glyph: &glyph_brush::SectionGlyph,
  ) -> Option<euclid::Box2D<f32, LogicUnit>> {
    self
      .brush
      .drawn_rect_at(glyph.font_id, &glyph.glyph)
      .map(|rect| {
        let min = Point::new(rect.min.x, rect.min.y);
        let max = Point::new(rect.max.x, rect.max.y);
        euclid::Box2D::new(min, max)
      })
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

  /// Add a font for brush, so brush can support this font.
  fn add_font(
    &mut self,
    name: &str,
    data: &'static [u8],
  ) -> Result<FontId, Box<dyn std::error::Error>> {
    if self.available_fonts.get(name).is_some() {
      let msg = format!("Font {} has already added.", name);
      return Err(FontAlreadyAdded(msg).into());
    }

    let font = FontArc::try_from_slice(data)?;
    let id = self.brush.add_font(font.clone());
    self.available_fonts.insert(name.to_string(), id);

    Ok(id)
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
        offset: 0,
        bytes_per_row: rect.width(),
        rows_per_image: rect.height(),
      },
      wgpu::TextureCopyView {
        texture,
        array_layer: 0,
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
      ..
    }: glyph_brush::GlyphVertex<()>,
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
  #[inline]
  pub fn add_font(
    &mut self,
    name: &str,
    data: &'static [u8],
  ) -> Result<FontId, Box<dyn std::error::Error>> {
    self.glyph_brush.add_font(name, data)
  }

  /// Get an using font id across its name
  #[inline]
  pub fn get_font_id_by_name(&mut self, name: &str) -> Option<FontId> {
    self.glyph_brush.available_fonts.get(name).cloned()
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

      label: None,
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
      label: Some("Encoder for encoding texture as png"),
    });

    encoder.copy_texture_to_buffer(
      wgpu::TextureCopyView {
        texture: &self.glyph_brush.texture,
        mip_level: 0,
        array_layer: 0,
        origin: wgpu::Origin3d::ZERO,
      },
      wgpu::BufferCopyView {
        buffer: &output_buffer,
        offset: 0,
        bytes_per_row: std::mem::size_of::<u32>() as u32 * width as u32,
        rows_per_image: 0,
      },
      wgpu::Extent3d {
        width: width,
        height: height,
        depth: 1,
      },
    );

    queue.submit(Some(encoder.finish()));

    // Note that we're not calling `.await` here.
    let buffer_future = output_buffer.map_read(0, size);

    // Poll the device in a blocking manner so that our future resolves.
    device.poll(wgpu::Maintain::Wait);

    let mapping = futures::executor::block_on(buffer_future).unwrap();
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
      .write_image_data(mapping.as_slice())
      .unwrap();

    log::debug!("Write a image of canvas atlas at: {}", &atlas_capture);
  }
}

#[derive(Debug)]
struct FontAlreadyAdded(String);
impl std::fmt::Display for FontAlreadyAdded {
  #[inline]
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str(self.0.as_str())
  }
}
impl std::error::Error for FontAlreadyAdded {}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::canvas::*;
  use futures::executor::block_on;
  use glyph_brush::Text;

  fn uninited_brush() -> TextBrush {
    let v = std::mem::MaybeUninit::uninit();
    let mut t_brush: TextBrush = unsafe { v.assume_init() };
    let brush = GlyphBrushBuilder::using_fonts(vec![])
      .initial_cache_size((INIT_SIZE.width, INIT_SIZE.height))
      .build();
    t_brush.brush = brush;
    t_brush.quad_vertices_cache = vec![];
    t_brush.available_fonts = Default::default();

    t_brush
  }

  fn free_uninit_brush(brush: TextBrush) {
    let TextBrush { texture, view, .. } = brush;
    std::mem::forget(texture);
    std::mem::forget(view);
  }

  fn add_default_fonts<S: Surface>(brush: &mut Canvas<S>) {
    brush
      .add_font("DejaVu", include_bytes!("../fonts/DejaVuSans.ttf"))
      .unwrap();
    brush
      .add_font(
        "GaramondNo8",
        include_bytes!("../fonts/GaramondNo8-Reg.ttf"),
      )
      .unwrap();
  }

  #[test]
  fn fonts_use() {
    let mut brush = uninited_brush();
    let deja = include_bytes!("../fonts/DejaVuSans.ttf");
    let graamond = include_bytes!("../fonts/GaramondNo8-Reg.ttf");
    let deja_id = brush.add_font("DejaVu", deja).unwrap();
    let graamond_id = brush.add_font("GaramondNo8", graamond).unwrap();

    assert_eq!(brush.available_fonts.get("DejaVu").unwrap(), &deja_id);
    assert_eq!(
      brush.available_fonts.get("GaramondNo8").unwrap(),
      &graamond_id
    );

    // name should be unique
    let res = brush.add_font("DejaVu", deja);
    assert!(res.is_err());

    assert_eq!(brush.available_fonts.get("DejaVu").unwrap(), &deja_id);
    assert_eq!(
      brush.available_fonts.get("GaramondNo8").unwrap(),
      &graamond_id
    );

    free_uninit_brush(brush);
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
