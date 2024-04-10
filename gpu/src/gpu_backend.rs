use std::error::Error;

use ribir_geom::{rect_corners, DeviceRect, DeviceSize, Point};
use ribir_painter::{
  image::ColorFormat, AntiAliasing, Color, PaintCommand, PaintPath, PainterBackend, PixelImage,
  Vertex, VertexBuffers,
};

use crate::{
  ColorAttr, GPUBackendImpl, GradientStopPrimitive, ImagePrimIndex, ImgPrimitive,
  LinearGradientPrimIndex, LinearGradientPrimitive, MaskLayer, RadialGradientPrimIndex,
  RadialGradientPrimitive,
};

mod atlas;

mod textures_mgr;
use textures_mgr::*;

pub struct GPUBackend<Impl: GPUBackendImpl> {
  gpu_impl: Impl,
  tex_mgr: TexturesMgr<Impl::Texture>,
  color_vertices_buffer: VertexBuffers<ColorAttr>,
  img_vertices_buffer: VertexBuffers<ImagePrimIndex>,
  img_prims: Vec<ImgPrimitive>,
  radial_gradient_vertices_buffer: VertexBuffers<RadialGradientPrimIndex>,
  radial_gradient_stops: Vec<GradientStopPrimitive>,
  radial_gradient_prims: Vec<RadialGradientPrimitive>,
  linear_gradient_prims: Vec<LinearGradientPrimitive>,
  linear_gradient_stops: Vec<GradientStopPrimitive>,
  linear_gradient_vertices_buffer: VertexBuffers<LinearGradientPrimIndex>,
  current_phase: CurrentPhase,
  tex_ids_map: TextureIdxMap,
  viewport: DeviceRect,
  mask_layers: Vec<MaskLayer>,
  clip_layer_stack: Vec<ClipLayer>,
  skip_clip_cnt: usize,
  surface_color: Option<Color>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum CurrentPhase {
  None,
  Color,
  Img,
  RadialGradient,
  LinearGradient,
}

struct ClipLayer {
  viewport: DeviceRect,
  mask_idx: i32,
}

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
  fn copy_as_image(
    &self, rect: &DeviceRect, host: &mut Self::Host,
  ) -> impl std::future::Future<Output = Result<PixelImage, Box<dyn Error>>> + 'static;

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

  fn begin_frame(&mut self, surface: Color) {
    self.surface_color = Some(surface);
    self.gpu_impl.begin_frame();
  }

  fn draw_commands(
    &mut self, viewport: DeviceRect, commands: Vec<PaintCommand>, output: &mut Self::Texture,
  ) {
    self.viewport = viewport;
    self.begin_draw_phase();
    let output_size = output.size();
    for cmd in commands.into_iter() {
      let maybe_used = match cmd {
        PaintCommand::ImgPath { .. } => 2,
        PaintCommand::PopClip => 0,
        _ => 1,
      };
      if self.tex_ids_map.all_textures().len() + maybe_used
        >= self.gpu_impl.load_tex_limit_per_draw()
        || !self.continues_cmd(&cmd)
      {
        // if the next command may hit the texture limit, submit the current draw phase.
        // And start a new draw phase.
        self.draw_triangles(output);
        self.end_draw_phase();
        self.begin_draw_phase();

        assert!(
          self.tex_ids_map.all_textures().len() + maybe_used
            < self.gpu_impl.load_tex_limit_per_draw(),
          "The GPUBackend implementation does not provide a sufficient texture limit per draw."
        )
      }
      self.draw_command(cmd, output_size);
    }
    self.draw_triangles(output);
    self.end_draw_phase();

    assert!(self.clip_layer_stack.is_empty());
  }

  fn end_frame(&mut self) {
    self.mask_layers.clear();
    self.tex_mgr.end_frame();
    self.gpu_impl.end_frame();
  }
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
      skip_clip_cnt: 0,
      color_vertices_buffer: VertexBuffers::with_capacity(256, 512),
      img_vertices_buffer: VertexBuffers::with_capacity(256, 512),
      radial_gradient_vertices_buffer: VertexBuffers::with_capacity(256, 512),
      radial_gradient_prims: vec![],
      radial_gradient_stops: vec![],
      linear_gradient_vertices_buffer: VertexBuffers::with_capacity(256, 512),
      linear_gradient_stops: vec![],
      linear_gradient_prims: vec![],
      img_prims: vec![],
      current_phase: CurrentPhase::None,
      viewport: DeviceRect::zero(),
      surface_color: Some(Color::WHITE),
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
          let color = color.into_components();
          let color_attr = ColorAttr { color, mask_head };
          let buffer = &mut self.color_vertices_buffer;
          add_draw_rect_vertices(rect, output_tex_size, color_attr, buffer);
          self.current_phase = CurrentPhase::Color;
        }
      }
      PaintCommand::ImgPath { path, img, opacity } => {
        let ts = path.transform;
        if let Some((rect, mask_head)) = self.new_mask_layer(path) {
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
          add_draw_rect_vertices(rect, output_tex_size, ImagePrimIndex(prim_idx), buffer);
          self.current_phase = CurrentPhase::Img;
        }
      }
      PaintCommand::RadialGradient { path, radial_gradient } => {
        let ts = path.transform;
        if let Some((rect, mask_head)) = self.new_mask_layer(path) {
          let prim: RadialGradientPrimitive = RadialGradientPrimitive {
            transform: ts.inverse().unwrap().to_array(),
            stop_start: self.radial_gradient_stops.len() as u32,
            stop_cnt: radial_gradient.stops.len() as u32,
            start_center: radial_gradient.start_center.to_array(),
            start_radius: radial_gradient.start_radius,
            end_center: radial_gradient.end_center.to_array(),
            end_radius: radial_gradient.end_radius,
            mask_head,
            spread: radial_gradient.spread_method as u32,
          };
          self.radial_gradient_stops.extend(
            radial_gradient
              .stops
              .into_iter()
              .map(Into::<GradientStopPrimitive>::into),
          );
          let prim_idx = self.radial_gradient_prims.len() as u32;
          self.radial_gradient_prims.push(prim);
          let buffer = &mut self.radial_gradient_vertices_buffer;

          add_draw_rect_vertices(rect, output_tex_size, RadialGradientPrimIndex(prim_idx), buffer);
          self.current_phase = CurrentPhase::RadialGradient;
        }
      }
      PaintCommand::LinearGradient { path, linear_gradient } => {
        let ts = path.transform;
        if let Some((rect, mask_head)) = self.new_mask_layer(path) {
          let prim: LinearGradientPrimitive = LinearGradientPrimitive {
            transform: ts.inverse().unwrap().to_array(),
            stop_start: self.linear_gradient_stops.len() as u32,
            stop_cnt: linear_gradient.stops.len() as u32,
            start_position: linear_gradient.start.to_array(),
            end_position: linear_gradient.end.to_array(),
            mask_head,
            spread: linear_gradient.spread_method as u32,
          };
          self.linear_gradient_stops.extend(
            linear_gradient
              .stops
              .into_iter()
              .map(Into::<GradientStopPrimitive>::into),
          );
          let prim_idx = self.linear_gradient_prims.len() as u32;
          self.linear_gradient_prims.push(prim);
          let buffer = &mut self.linear_gradient_vertices_buffer;
          add_draw_rect_vertices(rect, output_tex_size, LinearGradientPrimIndex(prim_idx), buffer);
          self.current_phase = CurrentPhase::LinearGradient;
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
              self
                .clip_layer_stack
                .push(ClipLayer { viewport, mask_idx });
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

  fn begin_draw_phase(&mut self) {
    self.current_phase = CurrentPhase::None;
    if !self.clip_layer_stack.is_empty() {
      // clear unused mask layers and update mask index.
      let mut retain_masks = Vec::with_capacity(self.clip_layer_stack.len());
      for s in self.clip_layer_stack.iter_mut() {
        retain_masks.push(s.mask_idx);
        s.mask_idx = retain_masks.len() as i32 - 1;
      }
      self.mask_layers = retain_masks
        .iter()
        .map(|&idx| self.mask_layers[idx as usize].clone())
        .collect();

      // update the texture index of mask layers in new draw phase.
      let tex_map = self.tex_ids_map.textures.clone();
      self.tex_ids_map.reset();
      for l in self.mask_layers.iter_mut() {
        let tex_id = tex_map[l.mask_tex_idx as usize];
        l.mask_tex_idx = self.tex_ids_map.tex_idx(tex_id);
      }
    } else {
      self.tex_ids_map.reset();
      self.mask_layers.clear();
    }
  }

  fn end_draw_phase(&mut self) {
    self.color_vertices_buffer.vertices.clear();
    self.color_vertices_buffer.indices.clear();
    self.img_vertices_buffer.vertices.clear();
    self.img_vertices_buffer.indices.clear();
    self.img_prims.clear();
    self
      .radial_gradient_vertices_buffer
      .indices
      .clear();
    self
      .radial_gradient_vertices_buffer
      .vertices
      .clear();
    self.radial_gradient_prims.clear();
    self.radial_gradient_stops.clear();
    self.linear_gradient_prims.clear();
    self
      .linear_gradient_vertices_buffer
      .indices
      .clear();
    self.linear_gradient_stops.clear();
  }

  fn continues_cmd(&self, cmd: &PaintCommand) -> bool {
    match (self.current_phase, cmd) {
      (CurrentPhase::None, _) => true,
      (_, PaintCommand::Clip(_))
      | (_, PaintCommand::PopClip)
      | (CurrentPhase::Color, PaintCommand::ColorPath { .. })
      | (CurrentPhase::Img, PaintCommand::ImgPath { .. })
      | (CurrentPhase::RadialGradient, PaintCommand::RadialGradient { .. })
      | (CurrentPhase::LinearGradient, PaintCommand::LinearGradient { .. }) => true,
      _ => false,
    }
  }

  fn current_clip_mask_index(&self) -> i32 {
    self
      .clip_layer_stack
      .last()
      .map_or(-1, |l| l.mask_idx)
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

  fn draw_triangles(&mut self, output: &mut Impl::Texture) {
    let mut color = self.surface_color.take();
    let gpu_impl = &mut self.gpu_impl;

    self.tex_mgr.draw_alpha_textures(gpu_impl);
    gpu_impl.load_mask_layers(&self.mask_layers);

    let textures = self.tex_ids_map.all_textures();
    let max_textures = gpu_impl.load_tex_limit_per_draw();
    let mut tex_buffer = Vec::with_capacity(max_textures);
    textures
      .into_iter()
      .take(max_textures)
      .for_each(|id| {
        tex_buffer.push(self.tex_mgr.texture(*id));
      });

    gpu_impl.load_textures(&tex_buffer);

    match self.current_phase {
      CurrentPhase::None => gpu_impl.draw_color_triangles(output, 0..0, color.take()),
      CurrentPhase::Color => {
        gpu_impl.load_color_vertices(&self.color_vertices_buffer);
        let rg = 0..self.color_vertices_buffer.indices.len() as u32;
        gpu_impl.draw_color_triangles(output, rg, color.take())
      }
      CurrentPhase::Img => {
        gpu_impl.load_img_primitives(&self.img_prims);
        gpu_impl.load_img_vertices(&self.img_vertices_buffer);
        let rg = 0..self.img_vertices_buffer.indices.len() as u32;
        gpu_impl.draw_img_triangles(output, rg, color.take())
      }
      CurrentPhase::RadialGradient => {
        gpu_impl.load_radial_gradient_primitives(&self.radial_gradient_prims);
        gpu_impl.load_radial_gradient_stops(&self.radial_gradient_stops);
        gpu_impl.load_radial_gradient_vertices(&self.radial_gradient_vertices_buffer);
        let rg = 0..self.radial_gradient_vertices_buffer.indices.len() as u32;
        gpu_impl.draw_radial_gradient_triangles(output, rg, color.take())
      }
      CurrentPhase::LinearGradient => {
        gpu_impl.load_linear_gradient_primitives(&self.linear_gradient_prims);
        gpu_impl.load_linear_gradient_stops(&self.linear_gradient_stops);
        gpu_impl.load_linear_gradient_vertices(&self.linear_gradient_vertices_buffer);
        let rg = 0..self.linear_gradient_vertices_buffer.indices.len() as u32;
        gpu_impl.draw_linear_gradient_triangles(output, rg, color.take())
      }
    }
  }
}

impl TextureIdxMap {
  fn tex_idx(&mut self, id: TextureID) -> u32 {
    *self.texture_map.entry(id).or_insert_with(|| {
      let idx = self.textures.len();
      self.textures.push(id);
      idx as u32
    })
  }

  fn all_textures(&self) -> &[TextureID] { &self.textures }

  fn reset(&mut self) {
    self.texture_map.clear();
    self.textures.clear();
  }
}

pub fn vertices_coord(pos: Point, tex_size: DeviceSize) -> [f32; 2] {
  [pos.x / tex_size.width as f32, pos.y / tex_size.height as f32]
}

pub fn add_draw_rect_vertices<Attr: Copy>(
  [lt, rt, rb, lb]: [Point; 4], tex_size: DeviceSize, attr: Attr, buffer: &mut VertexBuffers<Attr>,
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
  use ribir_algo::ShareResource;
  use ribir_dev_helper::*;
  use ribir_geom::*;
  use ribir_painter::{Brush, Painter, Path, Radius, Svg};

  use super::*;

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
    painter
      .set_brush(Color::RED)
      .set_line_width(5.)
      .stroke();

    painter.translate(-260., 250.);
    draw_arrow_path(&mut painter);
    painter.set_brush(img_brush.clone()).fill();

    painter.translate(260., 0.);
    draw_arrow_path(&mut painter);
    painter
      .set_brush(img_brush)
      .set_line_width(5.)
      .stroke();

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
    let mut painter = painter(Size::new(120., 340.));
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

  painter_backend_eq_image_test!(draw_svg_gradient);
  fn draw_svg_gradient() -> Painter {
    let mut painter = painter(Size::new(64., 64.));
    let svg =
      Svg::parse_from_bytes(include_bytes!("../../tests/assets/fill_with_gradient.svg")).unwrap();

    painter.draw_svg(&svg);
    painter
  }

  fn multi_draw_phase() -> Painter {
    let mut painter = painter(Size::new(1048., 1048.));

    let rect = Rect::from_size(Size::new(1024., 1024.));
    for i in 0..100 {
      let mut painter = painter.save_guard();
      painter.translate(i as f32 * 10., i as f32 * 10.);
      let color = if i % 2 == 0 { Color::GREEN } else { Color::RED };
      painter
        .set_brush(color)
        .rect_round(&rect, &Radius::all(i as f32))
        .fill();
    }
    painter
  }
  painter_backend_eq_image_test!(multi_draw_phase);
}
