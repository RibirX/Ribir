use self::textures_mgr::{TextureID, TexturesMgr};
use crate::{
  ColorAttr, GPUBackendImpl, GradientStopPrimitive, ImgPrimitive, MaskLayer, RadialGradientAttr,
  RadialGradientPrimitive,
};
use ribir_geom::{rect_corners, DeviceRect, DeviceSize, Point};
use ribir_painter::{
  image::ColorFormat, AntiAliasing, Color, PaintCommand, PaintPath, PainterBackend, PixelImage,
  Vertex, VertexBuffers,
};
use std::{error::Error, future::Future, ops::Range, pin::Pin};

mod atlas;

mod textures_mgr;
use textures_mgr::*;

pub struct GPUBackend<Impl: GPUBackendImpl> {
  gpu_impl: Impl,
  tex_mgr: TexturesMgr<Impl::Texture>,
  color_vertices_buffer: VertexBuffers<ColorAttr>,
  img_vertices_buffer: VertexBuffers<u32>,
  radial_gradient_vertices_buffer: VertexBuffers<RadialGradientAttr>,
  gradient_stops: Vec<GradientStopPrimitive>,
  radial_gradient_prims: Vec<RadialGradientPrimitive>,
  img_prims: Vec<ImgPrimitive>,
  draw_indices: Vec<DrawIndices>,
  tex_ids_map: TextureIdxMap,
  viewport: DeviceRect,
  mask_layers: Vec<MaskLayer>,
  clip_layer_stack: Vec<ClipLayer>,
  skip_clip_cnt: usize,
}

#[derive(Clone)]
enum DrawIndices {
  Color(Range<u32>),
  Img(Range<u32>),
  RadialGradient(Range<u32>),
}

struct ClipLayer {
  viewport: DeviceRect,
  mask_idx: i32,
}

pub type ImageFuture =
  Pin<Box<dyn Future<Output = Result<PixelImage, Box<dyn Error>>> + Send + Sync>>;
/// Texture use to display.
pub trait Texture {
  type Host;

  fn anti_aliasing(&self) -> AntiAliasing;

  fn set_anti_aliasing(&mut self, anti_aliasing: AntiAliasing, host: &mut Self::Host);

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
struct TextureIdxMap {
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
    self
      .tex_mgr
      .set_anti_aliasing(anti_aliasing, &mut self.gpu_impl);
  }

  fn begin_frame(&mut self) {
    self.tex_mgr.end_frame();
    self.gpu_impl.begin_frame();
  }

  fn draw_commands(
    &mut self,
    viewport: DeviceRect,
    commands: Vec<PaintCommand>,
    surface: Color,
    output: &mut Self::Texture,
  ) {
    // todo: batch multi `draw_commands`
    self.begin_draw_phase(viewport);

    let mut commands = commands.into_iter();
    let output_size = output.size();
    loop {
      let Some(cmd) = commands.next() else {
        break;
      };
      self.draw_command(cmd, output_size);
    }
    self.expand_indices_range();

    assert!(self.clip_layer_stack.is_empty());

    self.gpu_impl.load_mask_layers(&self.mask_layers);
    let all_tex_ids = self.tex_ids_map.all_textures();
    let mut textures = Vec::with_capacity(all_tex_ids.len());
    for id in all_tex_ids {
      textures.push(self.tex_mgr.texture(*id));
    }

    self.gpu_impl.load_textures(&textures);
    if !self.color_vertices_buffer.indices.is_empty() {
      self
        .gpu_impl
        .load_color_vertices(&self.color_vertices_buffer);
    }
    if !self.img_vertices_buffer.indices.is_empty() {
      self.gpu_impl.load_img_primitives(&self.img_prims);
      self.gpu_impl.load_img_vertices(&self.img_vertices_buffer);
    }
    if !self.radial_gradient_vertices_buffer.indices.is_empty() {
      self
        .gpu_impl
        .load_radial_gradient_primitives(&self.radial_gradient_prims);
      self
        .gpu_impl
        .load_radial_gradient_stops(&self.gradient_stops);
      self
        .gpu_impl
        .load_radial_gradient_vertices(&self.radial_gradient_vertices_buffer);
    }

    self.tex_mgr.submit(&mut self.gpu_impl);
    self.layers_submit(output, surface);

    self.end_draw_phase();
  }

  fn end_frame(&mut self) { self.gpu_impl.end_frame(); }
}

impl<Impl: GPUBackendImpl> GPUBackend<Impl>
where
  Impl::Texture: Texture<Host = Impl>,
{
  pub fn new(mut gpu_impl: Impl, anti_aliasing: AntiAliasing) -> Self {
    let tex_mgr = TexturesMgr::new(&mut gpu_impl, anti_aliasing);
    Self {
      gpu_impl,
      tex_mgr,
      tex_ids_map: <_>::default(),
      mask_layers: vec![],
      clip_layer_stack: vec![],
      radial_gradient_prims: vec![],
      skip_clip_cnt: 0,
      color_vertices_buffer: VertexBuffers::with_capacity(256, 512),
      img_vertices_buffer: VertexBuffers::with_capacity(256, 512),
      radial_gradient_vertices_buffer: VertexBuffers::with_capacity(256, 512),
      gradient_stops: vec![],
      img_prims: vec![],
      draw_indices: vec![],
      viewport: DeviceRect::zero(),
    }
  }

  #[inline]
  pub fn get_impl(&self) -> &Impl { &self.gpu_impl }

  #[inline]
  pub fn get_impl_mut(&mut self) -> &mut Impl { &mut self.gpu_impl }

  fn draw_command(&mut self, cmd: PaintCommand, output_tex_size: DeviceSize) {
    match cmd {
      PaintCommand::ColorPath { path, color } => {
        if let Some((rect, mask_head)) = self.new_mask_layer(path) {
          self.update_to_color_indices();
          let color = color.into_components();
          let color_attr = ColorAttr { color, mask_head };
          let buffer = &mut self.color_vertices_buffer;
          add_draw_rect_vertices(rect, output_tex_size, color_attr, buffer);
        }
      }
      PaintCommand::ImgPath { path, img, opacity } => {
        let ts = path.transform;
        if let Some((rect, mask_head)) = self.new_mask_layer(path) {
          self.update_to_img_indices();

          let img_slice = self.tex_mgr.store_image(&img, &mut self.gpu_impl);
          let img_start = img_slice.rect.origin.to_f32().to_array();
          let img_size = img_slice.rect.size.to_f32().to_array();
          let img_tex_idx = self.tex_ids_map.tex_idx(img_slice.tex_id);
          let prim_idx = self.img_prims.len() as u32;
          let prim = ImgPrimitive {
            transform: ts.inverse().unwrap().to_array(),
            img_start,
            img_size,
            img_tex_idx,
            mask_head,
            opacity,
            _dummy: 0,
          };
          self.img_prims.push(prim);
          let buffer = &mut self.img_vertices_buffer;
          add_draw_rect_vertices(rect, output_tex_size, prim_idx, buffer);
        }
      }
      PaintCommand::RadialGradient {
        path,
        stops,
        start,
        start_radius,
        end,
        end_radius,
        spread,
        transform,
      } => {
        let ts = transform.then(&path.transform);
        if let Some((rect, mask_head)) = self.new_mask_layer(path) {
          self.update_to_radial_gradient_indices();
          let prim: RadialGradientPrimitive = RadialGradientPrimitive {
            transform: ts.inverse().unwrap().to_array(),
            stop_start: self.gradient_stops.len() as u32,
            stop_cnt: stops.len() as u32,
            start_center: start.to_array(),
            start_radius,
            end_center: end.to_array(),
            end_radius,
            mask_head,
            spread: spread as u32,
          };
          self.gradient_stops.extend(stops.into_iter().map(|stop| {
            let color = stop.color.into_f32_components();
            GradientStopPrimitive {
              red: color[0],
              green: color[1],
              blue: color[2],
              alpha: color[3],
              offset: stop.offset,
            }
          }));
          let prim_idx = self.radial_gradient_prims.len() as u32;
          self.radial_gradient_prims.push(prim);
          let buffer = &mut self.radial_gradient_vertices_buffer;
          let attr = RadialGradientAttr { prim_idx };
          add_draw_rect_vertices(rect, output_tex_size, attr, buffer);
        }
      }
      PaintCommand::Clip(path) => {
        if self.skip_clip_cnt == 0 {
          if let Some(viewport) = path
            .paint_bounds
            .to_i32()
            .cast_unit()
            .intersection(self.viewport())
          {
            if let Some((_, mask_idx)) = self.new_mask_layer(path) {
              self.clip_layer_stack.push(ClipLayer { viewport, mask_idx });
              return;
            }
          }
        }
        self.skip_clip_cnt += 1;
      }
      PaintCommand::PopClip => {
        if self.skip_clip_cnt > 0 {
          self.skip_clip_cnt -= 1;
        } else {
          self.clip_layer_stack.pop();
        }
      }
    }
  }

  fn begin_draw_phase(&mut self, viewport: DeviceRect) {
    self.tex_ids_map.new_phase();
    self.viewport = viewport;
    self.tex_ids_map.tex_idx(TextureID::Alpha(0));
  }

  fn end_draw_phase(&mut self) {
    self.color_vertices_buffer.vertices.clear();
    self.color_vertices_buffer.indices.clear();
    self.img_vertices_buffer.vertices.clear();
    self.img_vertices_buffer.indices.clear();
    self.img_prims.clear();
    self.mask_layers.clear();
    self.radial_gradient_vertices_buffer.indices.clear();
    self.radial_gradient_vertices_buffer.vertices.clear();
    self.radial_gradient_prims.clear();
    self.gradient_stops.clear();
  }

  fn update_to_color_indices(&mut self) {
    if !matches!(self.draw_indices.last(), Some(DrawIndices::Color(_))) {
      self.expand_indices_range();
      let start = self.color_vertices_buffer.indices.len() as u32;
      self.draw_indices.push(DrawIndices::Color(start..start));
    }
  }

  fn update_to_img_indices(&mut self) {
    if !matches!(self.draw_indices.last(), Some(DrawIndices::Img(_))) {
      self.expand_indices_range();
      let start = self.img_vertices_buffer.indices.len() as u32;
      self.draw_indices.push(DrawIndices::Img(start..start));
    }
  }

  fn update_to_radial_gradient_indices(&mut self) {
    if !matches!(
      self.draw_indices.last(),
      Some(DrawIndices::RadialGradient(_))
    ) {
      self.expand_indices_range();
      let start = self.radial_gradient_vertices_buffer.indices.len() as u32;
      self
        .draw_indices
        .push(DrawIndices::RadialGradient(start..start));
    }
  }

  fn expand_indices_range(&mut self) -> Option<&DrawIndices> {
    let cmd = self.draw_indices.last_mut()?;
    match cmd {
      DrawIndices::Color(rg) => rg.end = self.color_vertices_buffer.indices.len() as u32,
      DrawIndices::Img(rg) => rg.end = self.img_vertices_buffer.indices.len() as u32,
      DrawIndices::RadialGradient(rg) => {
        rg.end = self.radial_gradient_vertices_buffer.indices.len() as u32
      }
    };

    Some(&*cmd)
  }

  fn current_clip_mask_index(&self) -> i32 {
    self.clip_layer_stack.last().map_or(-1, |l| l.mask_idx)
  }

  fn viewport(&self) -> &DeviceRect {
    self
      .clip_layer_stack
      .last()
      .map_or(&self.viewport, |l| &l.viewport)
  }

  fn new_mask_layer(&mut self, path: PaintPath) -> Option<([Point; 4], i32)> {
    let paint_bounds = path.paint_bounds.round_out().to_i32().cast_unit();
    let view = paint_bounds.intersection(self.viewport())?;
    let prefer_cache_size = prefer_cache_size(&path.path, &path.transform);

    let (mask, mask_to_view) =
      if valid_cache_item(&prefer_cache_size) || view.contains_rect(&paint_bounds) {
        self
          .tex_mgr
          .store_alpha_path(path.path, &path.transform, &mut self.gpu_impl)
      } else {
        self
          .tex_mgr
          .store_clipped_path(view, path, &mut self.gpu_impl)
      };

    let mut points = rect_corners(&mask.rect.to_f32().cast_unit());
    for p in points.iter_mut() {
      *p = mask_to_view.transform_point(*p);
    }

    let index = self.mask_layers.len();
    let min_max = mask.rect.to_box2d().to_f32();
    self.mask_layers.push(MaskLayer {
      // view to mask transform.
      transform: mask_to_view.inverse().unwrap().to_array(),
      min: min_max.min.to_array(),
      max: min_max.max.to_array(),
      mask_tex_idx: self.tex_ids_map.tex_idx(mask.tex_id),
      prev_mask_idx: self.current_clip_mask_index(),
    });
    Some((points, index as i32))
  }

  fn layers_submit(&mut self, output: &mut Impl::Texture, surface: Color) {
    let mut color = Some(surface);
    if self.draw_indices.is_empty() {
      self
        .gpu_impl
        .draw_color_triangles(output, 0..0, color.take())
    } else {
      self
        .draw_indices
        .drain(..)
        .for_each(|indices| match indices {
          DrawIndices::Color(rg) => self.gpu_impl.draw_color_triangles(output, rg, color.take()),
          DrawIndices::Img(rg) => self.gpu_impl.draw_img_triangles(output, rg, color.take()),
          DrawIndices::RadialGradient(rg) => {
            self
              .gpu_impl
              .draw_radial_gradient_triangles(output, rg, color.take())
          }
        });
    }
  }
}

impl TextureIdxMap {
  fn new_phase(&mut self) {
    self.texture_map.clear();
    self.textures.clear();
  }

  fn tex_idx(&mut self, id: TextureID) -> u32 {
    *self.texture_map.entry(id).or_insert_with(|| {
      let idx = self.textures.len();
      self.textures.push(id);
      idx as u32
    })
  }

  fn all_textures(&self) -> &[TextureID] { &self.textures }
}

pub fn vertices_coord(pos: Point, tex_size: DeviceSize) -> [f32; 2] {
  [
    pos.x / tex_size.width as f32,
    pos.y / tex_size.height as f32,
  ]
}

pub fn add_draw_rect_vertices<Attr: Copy>(
  [lt, rt, rb, lb]: [Point; 4],
  tex_size: DeviceSize,
  attr: Attr,
  buffer: &mut VertexBuffers<Attr>,
) {
  let VertexBuffers { vertices, indices } = buffer;

  let vertex_start = vertices.len() as u32;
  vertices.push(Vertex::new(vertices_coord(lt, tex_size), attr));
  vertices.push(Vertex::new(vertices_coord(rt, tex_size), attr));
  vertices.push(Vertex::new(vertices_coord(rb, tex_size), attr));
  vertices.push(Vertex::new(vertices_coord(lb, tex_size), attr));

  indices.push(vertex_start);
  indices.push(vertex_start + 3);
  indices.push(vertex_start + 2);
  indices.push(vertex_start + 2);
  indices.push(vertex_start + 1);
  indices.push(vertex_start);
}

#[cfg(feature = "wgpu")]
#[cfg(test)]
mod tests {
  use super::*;
  use ribir_algo::ShareResource;
  use ribir_dev_helper::*;
  use ribir_geom::*;
  use ribir_painter::{Brush, Color, Painter, Path, PixelImage};

  fn painter(bounds: Size) -> Painter { Painter::new(Rect::from_size(bounds)) }

  painter_backend_eq_image_test!(smoke);
  fn smoke() -> Painter {
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

    let mut painter = painter(Size::new(512., 512.));

    let img = PixelImage::from_png(include_bytes!("../imgs/leaves.png"));
    let share_img = ShareResource::new(img);

    let img_brush = Brush::Image(share_img);

    draw_arrow_path(&mut painter);
    painter.set_brush(Color::RED).fill();

    painter.translate(260., 0.);
    draw_arrow_path(&mut painter);
    painter.set_brush(Color::RED).set_line_width(5.).stroke();

    painter.translate(-260., 250.);
    draw_arrow_path(&mut painter);
    painter.set_brush(img_brush.clone()).fill();

    painter.translate(260., 0.);
    draw_arrow_path(&mut painter);
    painter.set_brush(img_brush).set_line_width(5.).stroke();

    painter
  }

  painter_backend_eq_image_test!(transform_img_brush);
  fn transform_img_brush() -> Painter {
    let mut painter = painter(Size::new(800., 250.));

    let transform = Transform::new(1., 1., 2., 1., 0., 0.);
    let rect: Rect = Rect::new(Point::new(10., 10.), Size::new(100., 100.));
    painter
      .set_brush(Color::RED)
      .set_transform(transform)
      .rect(&rect)
      .fill();

    let leaves_brush =
      ShareResource::new(PixelImage::from_png(include_bytes!("../imgs/leaves.png")));

    painter
      .set_brush(leaves_brush)
      .set_transform(transform.then_translate((400., 0.).into()))
      .rect(&rect)
      .fill();

    painter
  }

  painter_backend_eq_image_test!(clip_layers);
  fn clip_layers() -> Painter {
    let mut painter = painter(Size::new(1024., 512.));
    let rect_100x100 = Rect::from_size(Size::new(100., 100.));
    painter
      .set_brush(Color::RED)
      .translate(10., 20.)
      .rect(&rect_100x100)
      .fill()
      .translate(0., 200.)
      .clip(Path::circle(Point::new(50., 50.), 50.))
      .rect(&rect_100x100)
      .fill();

    painter
  }

  painter_backend_eq_image_test!(stroke_include_border);
  fn stroke_include_border() -> Painter {
    let mut painter = painter(Size::new(100., 100.));
    painter
      .set_brush(Color::RED)
      .begin_path(Point::new(50., 5.))
      .line_to(Point::new(95., 50.))
      .line_to(Point::new(50., 95.))
      .line_to(Point::new(5., 50.))
      .end_path(true)
      .set_line_width(10.)
      .stroke();
    painter
  }

  painter_backend_eq_image_test!(two_img_brush);
  fn two_img_brush() -> Painter {
    let mut painter = painter(Size::new(200., 100.));

    let brush1 = PixelImage::from_png(include_bytes!("../imgs/leaves.png"));
    let brush2 = PixelImage::from_png(include_bytes!("../../examples/attachments/3DDD-1.png"));
    let rect = rect(0., 0., 100., 100.);
    painter
      .set_brush(brush1)
      .rect(&rect)
      .fill()
      .set_brush(brush2)
      .translate(100., 0.)
      .clip(Path::circle(Point::new(50., 50.), 50.))
      .rect(&rect)
      .fill();

    painter
  }

  painter_backend_eq_image_test!(draw_partial_img);
  fn draw_partial_img() -> Painter {
    let img = ShareResource::new(PixelImage::from_png(include_bytes!("../imgs/leaves.png")));
    let m_width = img.width() as f32;
    let m_height = img.height() as f32;
    let mut painter = painter(Size::new(m_width * 2., m_height * 2.));

    painter.draw_img(
      img,
      &Rect::new(Point::new(m_width, m_height), Size::new(m_width, m_height)),
      &Some(Rect::new(
        Point::new(m_width / 2., m_height / 2.),
        Size::new(m_width / 2., m_height / 2.),
      )),
    );

    painter
  }
}
