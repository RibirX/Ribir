use super::{
  canvas, surface::Surface, Canvas, DeviceRect, DeviceSize, Frame, FrameImpl,
  Rect,
};
use glyph_brush::{
  ab_glyph::FontArc, BrushAction, BrushError, FontId, GlyphBrush,
  GlyphBrushBuilder,
};
use log::{log_enabled, warn};

pub(crate) type Section<'a> = glyph_brush::Section<'a, ()>;
pub(crate) type Text<'a> = glyph_brush::Text<'a, ()>;

const INIT_SIZE: DeviceSize = DeviceSize::new(512, 512);

#[derive(Debug, Clone)]
pub(crate) struct QuadVertex {
  pub(crate) pixel_coords: Rect,
  pub(crate) tex_coords: Rect,
}

pub(crate) struct TextBrush {
  brush: GlyphBrush<QuadVertex, ()>,
  texture: wgpu::Texture,
  quad_vertices_cache: Vec<QuadVertex>,
  available_fonts: std::collections::HashMap<String, FontId>,
}

impl TextBrush {
  pub(crate) fn new(device: &wgpu::Device) -> Self {
    let brush = GlyphBrushBuilder::using_fonts(vec![])
      .initial_cache_size((INIT_SIZE.width, INIT_SIZE.height))
      .build();

    TextBrush {
      brush,
      texture: Self::texture(device, INIT_SIZE),
      quad_vertices_cache: vec![],
      available_fonts: Default::default(),
    }
  }

  pub(crate) fn queue(&mut self, section: Section) {
    self.brush.queue(section);
  }

  pub(crate) fn process_queued<S, T>(
    &mut self,
    frame: &mut FrameImpl<S, T>,
  ) -> &[QuadVertex]
  where
    S: Surface,
    T: std::borrow::Borrow<wgpu::TextureView>,
  {
    loop {
      let Self { brush, texture, .. } = self;
      let (canvas, encoder) = frame.canvas_and_encoder();
      let action = brush.process_queued(
        |rect, data| {
          Self::update_texture(texture, &canvas.device, encoder, rect, data)
        },
        Self::to_vertex,
      );

      match action {
        Ok(res) => {
          match res {
            BrushAction::Draw(vertices) => self.quad_vertices_cache = vertices,
            BrushAction::ReDraw => {}
          }
          break;
        }
        Err(BrushError::TextureTooSmall { suggested }) => {
          const MAX: u32 = canvas::surface::Texture::MAX_DIMENSION;
          let new_size = if suggested.0 >= MAX || suggested.1 >= MAX {
            (MAX, MAX)
          } else {
            suggested
          };
          frame.submit();

          self.texture = Self::texture(&frame.canvas().device, new_size.into());
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
      }
    }
    self.quad_vertices_cache.as_slice()
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
    let buffer =
      device.create_buffer_with_data(tex_data, wgpu::BufferUsage::COPY_SRC);

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
      tex_coords.max.x = tex_coords.min.x
        + tex_coords.width() * pixel_coords.width() / old_width;
    }

    if pixel_coords.min.x < bounds.min.x {
      let old_width = pixel_coords.width();
      pixel_coords.min.x = bounds.min.x;
      tex_coords.min.x = tex_coords.max.x
        - tex_coords.width() * pixel_coords.width() / old_width;
    }

    if pixel_coords.max.y > bounds.max.y {
      let old_height = pixel_coords.height();
      pixel_coords.max.y = bounds.max.y;
      tex_coords.max.y = tex_coords.min.y
        + tex_coords.height() * pixel_coords.height() / old_height;
    }

    if pixel_coords.min.y < bounds.min.y {
      let old_height = pixel_coords.height();
      pixel_coords.min.y = bounds.min.y;
      tex_coords.min.y = tex_coords.max.y
        - tex_coords.height() * pixel_coords.height() / old_height;
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
      format: wgpu::TextureFormat::Rg8Unorm,
      usage: wgpu::TextureUsage::COPY_DST | wgpu::TextureUsage::SAMPLED,
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
    self.text_brush.add_font(name, data)
  }

  /// Get an using font id across its name
  #[inline]
  pub fn get_font_id_by_name(&mut self, name: &str) -> Option<FontId> {
    self.text_brush.available_fonts.get(name).cloned()
  }

  #[cfg(debug_assertions)]
  pub(crate) fn log_glyph_texture(&mut self) {
    self.create_converter_if_none();

    let Canvas {
      tex_atlas,
      device,
      queue,
      rgba_converter,
      ..
    } = self;

    let pkg_root = env!("CARGO_MANIFEST_DIR");
    let atlas_capture =
      format!("{}/.log/{}", pkg_root, "glyph_texture_cache.png");

    let size = self.text_brush.brush.texture_dimensions();
    let atlas = canvas::texture_to_png(
      &tex_atlas.texture.raw_texture,
      DeviceRect::from_size(size.into()),
      device,
      queue,
      rgba_converter.as_ref().unwrap(),
      std::fs::File::create(&atlas_capture).unwrap(),
    );

    let _r = futures::executor::block_on(atlas);

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
    let TextBrush { texture, .. } = brush;
    std::mem::forget(texture);
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
    let brush = &mut canvas.text_brush;
    let str = "Hello glyph!";
    brush.queue(Section::new().add_text(Text::default().with_text(str)));

    let mut frame = canvas.next_frame();
    let vertices = brush.process_queued(&mut frame);
    assert_eq!(vertices.len(), str.chars().count());

    unimplemented!();
  }
}
