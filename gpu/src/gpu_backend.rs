use self::textures_mgr::{valid_atlas_item, TextureID, TexturesMgr};
use crate::{
  gpu_backend::textures_mgr::TextureRect, ColorPrimitive, DrawIndices, GPUBackendImpl,
  TexturePrimitive,
};
use ribir_painter::{
  image::ColorFormat, AntiAliasing, Brush, DevicePoint, DeviceRect, ImageFuture, PaintCommand,
  PaintPath, PainterBackend, Texture, Transform, Vertex, VertexBuffers,
};

mod atlas;
mod textures_mgr;

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

struct Layer {
  viewport: DeviceRect,
  texture: TextureRect,
  commands: Vec<DrawIndices>,
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
  fn set_anti_aliasing(&mut self, anti_aliasing: AntiAliasing) {
    self.gpu_impl.set_anti_aliasing(anti_aliasing);
  }

  fn begin_frame(&mut self) { self.gpu_impl.begin_frame() }

  fn draw_commands(&mut self, view_port: DeviceRect, commands: Vec<PaintCommand>) -> ImageFuture {
    self.gpu_impl.start_draw_phase();
    self.prim_ids_map.new_phase();
    self.buffers.vertices.clear();
    self.buffers.indices.clear();

    let output = self
      .tex_mgr
      .alloc(view_port.size, ColorFormat::Rgba8, &mut self.gpu_impl);

    let layer = Layer::new(output, view_port);
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
    self.layers_submit();

    self.gpu_impl.end_draw_phase();
    let texture = self.tex_mgr.texture(output.tex_id);
    texture.copy_as_image(&view_port, &mut self.gpu_impl)
  }

  fn end_frame(&mut self) {
    self.tex_mgr.end_frame();
    self.gpu_impl.end_frame();
    self.color_primitives.clear();
    self.texture_primitives.clear();
  }
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
          self.add_rect_triangles(intersect_view, prim_id);
        }
      }
      PaintCommand::Clip(path) => {
        if self.skip_clip_cnt > 0 {
          self.skip_clip_cnt += 1;
        } else if let Some(view_port) = path.bounds.intersection(&self.viewport()) {
          let clip_mask = self.fill_alpha_path(view_port, path);
          let texture = self
            .tex_mgr
            .alloc(view_port.size, ColorFormat::Rgba8, &mut self.gpu_impl);

          let layer = Layer::new(texture, view_port);
          let prim_id = self.add_texture_primitive(
            view_port.origin,
            texture,
            1.,
            clip_mask,
            &Transform::default(),
          );
          self.add_rect_triangles(view_port, prim_id);
          self.layer_stack.push(layer);
          self.layer_stack_idx += 1;
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

  fn add_primitive(&mut self, content_origin: DevicePoint, brush: Brush, mask: TextureRect) -> u32 {
    self.update_indices_range(&brush);
    let TextureRect { tex_id, rect } = mask;
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
      Brush::Color(_) => DrawIndices::Color(rg),
      Brush::Image { .. } => DrawIndices::Texture(rg),
      Brush::Gradient => todo!(),
    };
    self.current_layer().commands.push(rg)
  }

  fn expand_indices_range(&mut self) -> Option<&DrawIndices> {
    let end = self.buffers.indices.len() as u32;
    let cmd = self.current_layer().commands.last_mut()?;
    let rg = match cmd {
      DrawIndices::Color(rg) => rg,
      DrawIndices::Texture(rg) => rg,
      DrawIndices::Gradient(_) => todo!(),
    };
    rg.end = end;

    Some(&*cmd)
  }

  fn add_texture_primitive(
    &mut self,
    content_origin: DevicePoint,
    texture: TextureRect,
    opacity: f32,
    mask: TextureRect,
    transform: &Transform,
  ) -> u32 {
    let TextureRect { tex_id: mask_id, rect: mask_rect } = mask;
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
  fn add_rect_triangles(&mut self, rect: DeviceRect, prim_id: u32) {
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
  fn fill_alpha_path(&mut self, intersect_viewport: DeviceRect, path: PaintPath) -> TextureRect {
    if valid_atlas_item(&path.bounds.size) || intersect_viewport.contains_rect(&path.bounds) {
      let offset = intersect_viewport.origin - path.bounds.origin;
      let TextureRect { tex_id, rect } = self.tex_mgr.store_alpha_path(path, &mut self.gpu_impl);
      let origin = rect.origin + offset;
      let rect = DeviceRect::new(origin, intersect_viewport.size);
      TextureRect { tex_id, rect }
    } else {
      self
        .tex_mgr
        .alloc_path_without_cache(intersect_viewport, path, &mut self.gpu_impl)
    }
  }

  fn layers_submit(&mut self) {
    self.layer_stack.drain(..).rev().for_each(|layer| {
      let Layer { texture, commands, .. } = layer;
      let rect = texture.rect;
      let texture = self.tex_mgr.texture_mut(texture.tex_id);
      self.gpu_impl.draw_triangles(texture, rect, &commands);
    });
  }
}

impl DrawIndices {
  pub fn is_same_primitive(&self, brush: &Brush) -> bool {
    matches!(
      (self, brush),
      (DrawIndices::Color(_), Brush::Color(_))
        | (DrawIndices::Texture(_), Brush::Image { .. })
        | (DrawIndices::Gradient(_), Brush::Gradient)
    )
  }
}

impl Layer {
  fn new(texture: TextureRect, viewport: DeviceRect) -> Self {
    Layer { texture, viewport, commands: vec![] }
  }
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
    Painter::new(1., store)
  }

  fn commands_to_image(commands: Vec<PaintCommand>) -> PixelImage {
    let gpu_impl = block_on(WgpuImpl::new(AntiAliasing::Msaa4X));
    let mut gpu_backend = GPUBackend::new(gpu_impl);
    gpu_backend.begin_frame();
    gpu_backend.gpu_impl.start_capture();
    let rect = DeviceRect::from_size(DeviceSize::new(1024, 512));
    let img = gpu_backend.draw_commands(rect, commands);
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
