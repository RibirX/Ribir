use self::textures_mgr::{valid_atlas_item, TextureID, TexturesMgr};
use crate::{ColorPrimitive, GPUBackendImpl, IndicesRange, TexturePrimitive};
use ribir_painter::{
  image::ColorFormat, AntiAliasing, Brush, DevicePoint, DeviceRect, DeviceSize, PaintCommand,
  PaintPath, PainterBackend, PixelImage, Transform, Vertex, VertexBuffers,
};
use std::{error::Error, future::Future, pin::Pin};

mod atlas;
mod textures_mgr;
use textures_mgr::*;

pub struct GPUBackend<Impl: GPUBackendImpl> {
  gpu_impl: Impl,
  tex_mgr: TexturesMgr<Impl::Texture>,
  color_primitives: Vec<ColorPrimitive>,
  texture_primitives: Vec<TexturePrimitive>,
  prim_ids_map: TexturePrimIdsMap,
  buffers: VertexBuffers<u32>,
  layer_stack: Vec<Layer>,
  layer_stack_idx: usize,
  skip_clip_cnt: usize,
}

enum LayerDrawCmd {
  Draw(IndicesRange),
  ComposeLayerFrom {
    clip_mask: TextureSlice,
    src_layer: usize,
  },
}
struct Layer {
  viewport: DeviceRect,
  commands: Vec<LayerDrawCmd>,
}

pub type ImageFuture =
  Pin<Box<dyn Future<Output = Result<PixelImage, Box<dyn Error>>> + Send + Sync>>;
/// Texture use to display.
pub trait Texture {
  type Host;

  /// write data to the texture.
  fn write_data(&mut self, dist: &DeviceRect, data: &[u8], host: &mut Self::Host);

  /// Return a image future of the texture area.
  /// - you should poll the image future after the `end_frame` is called to
  ///   ensure all content had been submitted, because the PainterBackend does
  ///   not be required to draw synchronization
  fn copy_as_image(&self, rect: &DeviceRect, host: &mut Self::Host) -> ImageFuture;

  fn color_format(&self) -> ColorFormat;

  fn size(&self) -> DeviceSize;
}

#[derive(Default)]
struct TexturePrimIdsMap {
  texture_map: ahash::HashMap<TextureID, u32>,
  textures: Vec<TextureID>,
}

impl<Impl> PainterBackend for GPUBackend<Impl>
where
  Impl: GPUBackendImpl,
  Impl::Texture: Texture<Host = Impl>,
{
  type Texture = Impl::Texture;
  fn set_anti_aliasing(&mut self, anti_aliasing: AntiAliasing) {
    self.gpu_impl.set_anti_aliasing(anti_aliasing);
  }

  fn begin_frame(&mut self) {
    self.tex_mgr.end_frame();
    self.gpu_impl.begin_frame();
  }

  fn draw_commands(
    &mut self,
    viewport: DeviceRect,
    commands: Vec<PaintCommand>,
    output: &mut Self::Texture,
  ) {
    self.prim_ids_map.new_phase();
    self.buffers.vertices.clear();
    self.buffers.indices.clear();
    self.color_primitives.clear();
    self.texture_primitives.clear();

    let layer = Layer::new(viewport);
    self.layer_stack.push(layer);

    let mut commands = commands.into_iter();
    loop {
      let Some(cmd) = commands.next() else { break; };
      self.draw_command(cmd);
    }
    self.expand_indices_range();

    assert_eq!(self.layer_stack_idx, 0);

    let textures = self
      .prim_ids_map
      .all_textures()
      .into_iter()
      .map(|id| self.tex_mgr.texture(*id));
    self.gpu_impl.load_textures(textures);
    self.gpu_impl.load_color_primitives(&self.color_primitives);
    self
      .gpu_impl
      .load_texture_primitives(&self.texture_primitives);
    self.gpu_impl.load_triangles_vertices(&self.buffers);

    self.tex_mgr.submit(&mut self.gpu_impl);
    self.layers_submit(output);
  }

  fn end_frame(&mut self) { self.gpu_impl.end_frame(); }
}

impl<Impl: GPUBackendImpl> GPUBackend<Impl>
where
  Impl::Texture: Texture<Host = Impl>,
{
  pub fn new(mut gpu_impl: Impl) -> Self {
    let tex_mgr = TexturesMgr::new(&mut gpu_impl);
    Self {
      gpu_impl,
      tex_mgr,
      color_primitives: vec![],
      texture_primitives: vec![],
      prim_ids_map: <_>::default(),
      buffers: <_>::default(),
      layer_stack: vec![],
      layer_stack_idx: 0,
      skip_clip_cnt: 0,
    }
  }

  #[inline]
  pub fn get_impl(&self) -> &Impl { &self.gpu_impl }

  fn draw_command(&mut self, cmd: PaintCommand) {
    match cmd {
      PaintCommand::Fill { brush, paint_path } => {
        if let Some(intersect_view) = paint_path.bounds.intersection(&self.viewport()) {
          let mask = self.fill_alpha_path(intersect_view, paint_path);
          let prim_id = self.add_primitive(intersect_view.origin, brush, mask);
          self.draw_rect_triangles(intersect_view, prim_id);
        }
      }
      PaintCommand::Clip(path) => {
        if self.skip_clip_cnt > 0 {
          self.skip_clip_cnt += 1;
        } else if let Some(viewport) = path.bounds.intersection(&self.viewport()) {
          let clip_mask = self.fill_alpha_path(viewport, path);
          let compose_cmd = LayerDrawCmd::ComposeLayerFrom {
            clip_mask,
            src_layer: self.layer_stack.len(),
          };
          self.expand_indices_range();
          self.current_layer().commands.push(compose_cmd);
          self.new_layer(viewport);
        } else {
          self.skip_clip_cnt += 1;
        }
      }

      PaintCommand::PopClip => {
        if self.skip_clip_cnt > 0 {
          self.skip_clip_cnt -= 1;
        } else {
          self.layer_stack_idx -= 1;
          assert_eq!(self.skip_clip_cnt, 0)
        }
      }
    }
  }

  fn add_primitive(
    &mut self,
    content_origin: DevicePoint,
    brush: Brush,
    mask: TextureSlice,
  ) -> u32 {
    self.update_indices_range(&brush);
    let TextureSlice { tex_id, rect } = mask;
    let mask_id = self.prim_ids_map.prim_id(tex_id);
    match brush {
      Brush::Color(color) => {
        self.color_primitives.push(ColorPrimitive {
          mask_id,
          mask_offset: (rect.origin - content_origin).to_f32().to_array(),
          color: color.into_f32_components(),
          _dummy: 0,
        });
        (self.color_primitives.len() - 1) as u32
      }
      Brush::Image { img, opacity, transform } => {
        let texture = self.tex_mgr.store_image(&img, &mut self.gpu_impl);
        self.add_texture_primitive(content_origin, texture, opacity, mask, &transform)
      }
      Brush::Gradient => todo!(),
    }
  }

  fn update_indices_range(&mut self, brush: &Brush) {
    let last = self.expand_indices_range();

    if last.map_or(true, |last| !last.is_same_primitive(brush)) {
      self.start_new_primitive_range(brush);
    }
  }

  fn start_new_primitive_range(&mut self, brush: &Brush) {
    let start = self.buffers.indices.len() as u32;
    let rg = start..start;

    let rg = match brush {
      Brush::Color(_) => IndicesRange::Color(rg),
      Brush::Image { .. } => IndicesRange::Texture(rg),
      Brush::Gradient => todo!(),
    };
    self.current_layer().commands.push(LayerDrawCmd::Draw(rg))
  }

  fn expand_indices_range(&mut self) -> Option<&IndicesRange> {
    let end = self.buffers.indices.len() as u32;
    let cmd = self.current_layer().commands.last_mut()?;

    let LayerDrawCmd::Draw(cmd) = cmd else { return None };
    let rg = match cmd {
      IndicesRange::Color(rg) => rg,
      IndicesRange::Texture(rg) => rg,
      IndicesRange::Gradient(_) => todo!(),
    };
    rg.end = end;

    Some(&*cmd)
  }

  fn add_texture_primitive(
    &mut self,
    content_origin: DevicePoint,
    texture: TextureSlice,
    opacity: f32,
    mask: TextureSlice,
    transform: &Transform,
  ) -> u32 {
    let TextureSlice { tex_id: mask_id, rect: mask_rect } = mask;
    let brush_tex_idx = self.prim_ids_map.prim_id(texture.tex_id) as u16;
    let mask_idx = self.prim_ids_map.prim_id(mask_id) as u16;
    self.texture_primitives.push(TexturePrimitive {
      brush_tex_idx,
      mask_idx,
      opacity,
      content_origin: content_origin.to_f32().to_array(),
      brush_origin: texture.rect.origin.to_f32().to_array(),
      brush_size: texture.rect.size.to_f32().to_array(),
      mask_offset: (mask_rect.origin - content_origin).to_f32().to_array(),
      transform: transform.to_arrays(),
    });
    (self.texture_primitives.len() - 1) as u32
  }

  fn draw_rect_triangles(&mut self, rect: DeviceRect, prim_id: u32) {
    let lt = rect.min().to_f32();
    let rb = rect.max().to_f32();
    let rt = [rb.x, lt.y];
    let lb = [lt.x, rb.y];
    let VertexBuffers { vertices, indices } = &mut self.buffers;
    let index_offset = vertices.len() as u32;
    vertices.push(Vertex::new(lt.to_array(), prim_id));
    vertices.push(Vertex::new(rt, prim_id));
    vertices.push(Vertex::new(rb.to_array(), prim_id));
    vertices.push(Vertex::new(lb, prim_id));
    indices.push(index_offset);
    indices.push(index_offset + 3);
    indices.push(index_offset + 2);
    indices.push(index_offset + 2);
    indices.push(index_offset + 1);
    indices.push(index_offset);
  }

  fn current_layer(&mut self) -> &mut Layer { self.layer_stack.last_mut().unwrap() }

  fn viewport(&self) -> &DeviceRect { &self.layer_stack.last().unwrap().viewport }

  /// fill an alpha path in the viewport and return the texture of the viewport.
  fn fill_alpha_path(&mut self, intersect_viewport: DeviceRect, path: PaintPath) -> TextureSlice {
    if valid_atlas_item(&path.bounds.size) || intersect_viewport.contains_rect(&path.bounds) {
      let offset = intersect_viewport.origin - path.bounds.origin;
      let TextureSlice { tex_id, rect } = self.tex_mgr.store_alpha_path(path, &mut self.gpu_impl);
      let origin = rect.origin + offset;
      let rect = DeviceRect::new(origin, intersect_viewport.size);
      TextureSlice { tex_id, rect }
    } else {
      self
        .tex_mgr
        .alloc_path_without_cache(intersect_viewport, path, &mut self.gpu_impl)
    }
  }

  fn new_layer(&mut self, viewport: DeviceRect) {
    self.layer_stack.push(Layer::new(viewport));
    self.layer_stack_idx += 1;
  }

  fn layers_submit(&mut self, output: &mut Impl::Texture) {
    let invalid_texture = TextureSlice {
      tex_id: TextureID::Extra(u32::MAX),
      rect: DeviceRect::zero(),
    };
    let mut layers_texture = vec![invalid_texture; self.layer_stack.len()];
    let mut idx = self.layer_stack.len() - 1;
    loop {
      let Layer { viewport, commands } = &self.layer_stack[idx];
      commands.iter().for_each(|cmd| match cmd {
        LayerDrawCmd::Draw(indices) => {
          self
            .gpu_impl
            .draw_triangles(output, viewport, indices.clone())
        }
        LayerDrawCmd::ComposeLayerFrom { clip_mask, src_layer } => {
          let texture_slice = &layers_texture[*src_layer];
          let texture = self.tex_mgr.texture(texture_slice.tex_id);
          let mask = self.tex_mgr.texture(clip_mask.tex_id);

          self.gpu_impl.draw_texture_with_mask(
            output,
            self.layer_stack[*src_layer].viewport.origin,
            texture,
            texture_slice.rect.origin,
            mask,
            &clip_mask.rect,
          );
        }
      });

      if idx == 0 {
        break;
      } else {
        let slice = self
          .tex_mgr
          .alloc(viewport.size, ColorFormat::Rgba8, &mut self.gpu_impl);
        let texture = self.tex_mgr.texture_mut(slice.tex_id);
        self
          .gpu_impl
          .copy_texture_to_texture(texture, slice.rect.origin, output, viewport);
        layers_texture[idx] = slice;

        idx -= 1;
      }
    }
    self.layer_stack.clear();
  }
}

impl IndicesRange {
  pub fn is_same_primitive(&self, brush: &Brush) -> bool {
    matches!(
      (self, brush),
      (IndicesRange::Color(_), Brush::Color(_))
        | (IndicesRange::Texture(_), Brush::Image { .. })
        | (IndicesRange::Gradient(_), Brush::Gradient)
    )
  }
}

impl Layer {
  fn new(viewport: DeviceRect) -> Self { Layer { viewport, commands: vec![] } }
}

impl TexturePrimIdsMap {
  fn new_phase(&mut self) {
    self.texture_map.clear();
    self.textures.clear();
  }

  fn prim_id(&mut self, id: TextureID) -> u32 {
    *self.texture_map.entry(id).or_insert_with(|| {
      let idx = self.textures.len();
      self.textures.push(id);
      idx as u32
    })
  }

  fn all_textures(&self) -> &[TextureID] { &self.textures }
}

#[cfg(test)]
mod tests {
  use crate::WgpuImpl;

  use super::*;
  use futures::executor::block_on;
  use ribir_algo::ShareResource;
  use ribir_painter::{
    font_db::FontDB, shaper::TextShaper, Brush, Color, DeviceSize, Painter, PixelImage, Point,
    Rect, Size, TypographyStore,
  };
  use std::sync::{Arc, RwLock};

  fn painter() -> Painter {
    let font_db = Arc::new(RwLock::new(FontDB::default()));
    let store = TypographyStore::new(<_>::default(), font_db.clone(), TextShaper::new(font_db));
    Painter::new(1., Rect::from_size(Size::new(1024., 1024.)), store)
  }

  fn commands_to_image(commands: Vec<PaintCommand>) -> PixelImage {
    let rect = DeviceRect::from_size(DeviceSize::new(1024, 512));
    let mut gpu_impl = block_on(WgpuImpl::headless(AntiAliasing::Msaa4X));
    let mut texture = gpu_impl.new_texture(rect.size, ColorFormat::Rgba8);
    let mut gpu_backend = GPUBackend::new(gpu_impl);
    gpu_backend.begin_frame();
    gpu_backend.gpu_impl.start_capture();
    gpu_backend.draw_commands(rect, commands, &mut texture);
    let img = texture.copy_as_image(&rect, &mut gpu_backend.gpu_impl);
    gpu_backend.end_frame();
    let img = block_on(img).unwrap();
    gpu_backend.gpu_impl.stop_capture();

    img
  }

  #[test]
  fn smoke() {
    fn draw_arrow_path(painter: &mut Painter) {
      painter
        .begin_path((0., 70.).into())
        .line_to((100.0, 70.0).into())
        .line_to((100.0, 0.0).into())
        .line_to((250.0, 100.0).into())
        .line_to((100.0, 200.0).into())
        .line_to((100.0, 130.0).into())
        .line_to((0.0, 130.0).into())
        .end_path(true);
    }

    let mut painter = painter();

    let img = PixelImage::from_png(include_bytes!("../test_imgs/leaves.png"));
    let share_img = ShareResource::new(img);

    let img_brush = Brush::Image {
      img: share_img,
      opacity: 1.,
      transform: <_>::default(),
    };

    draw_arrow_path(&mut painter);
    painter.set_brush(Color::RED).fill();

    painter.translate(300., 0.);
    draw_arrow_path(&mut painter);
    painter.set_brush(Color::RED).set_line_width(5.).stroke();

    painter.translate(-300., 250.);
    draw_arrow_path(&mut painter);
    painter.set_brush(img_brush.clone()).fill();

    painter.translate(300., 0.);
    draw_arrow_path(&mut painter);
    painter.set_brush(img_brush).set_line_width(5.).stroke();

    let img = commands_to_image(painter.finish());
    let expect = PixelImage::from_png(include_bytes!("../test_imgs/smoke_arrow.png"));
    assert!(img == expect);
  }

  #[test]
  fn transform_img_brush() {
    let mut painter = painter();

    let transform = Transform::new(1., 1., 2., 1., 0., 0.);
    let rect: Rect = Rect::new(Point::new(10., 10.), Size::new(100., 100.));
    painter
      .set_brush(Color::RED)
      .set_transform(transform)
      .rect(&rect)
      .fill();

    let leaves_brush = ShareResource::new(PixelImage::from_png(include_bytes!(
      "../test_imgs/leaves.png"
    )));

    painter
      .set_brush(leaves_brush)
      .set_transform(transform.then_translate((400., 0.).into()))
      .rect(&rect)
      .fill();

    let img = commands_to_image(painter.finish());
    let expect = PixelImage::from_png(include_bytes!("../test_imgs/transform_brush.png"));
    assert!(img == expect);
  }
}
