use std::error::Error;

use guillotiere::euclid::Vector2D;
use ribir_geom::{
  DeviceRect, DeviceSize, Point, Transform, rect_corners, transform_to_device_rect,
};
use ribir_painter::{
  Color, ColorMatrix, CommandBrush, FilterType, FlattenMatrix, PaintCommand, PaintPath,
  PaintPathAction, PainterBackend, PaintingStyle, PathCommand, PixelImage, Vertex, VertexBuffers,
  color::ColorFilterMatrix, image::ColorFormat,
};

use crate::{
  ColorAttr, FilterPrimitive, GPUBackendImpl, GradientStopPrimitive, ImagePrimIndex, ImgPrimitive,
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
  filter_vertices_buffer: VertexBuffers<()>,
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

#[derive(Clone, Debug)]
enum CurrentPhase {
  None,
  Color,
  Img,
  RadialGradient,
  LinearGradient,
  Filter(Box<FilterPhase>),
}

#[derive(Clone, Debug)]
struct FilterPhase {
  view_rect: DeviceRect,
  mask_head: i32,
  filters: Vec<FilterType>,
}

impl FilterPhase {
  fn primitive_iter(&self) -> impl Iterator<Item = (FilterPrimitive, Vec<f32>)> + '_ {
    let mut origin_it = self.filters.iter().peekable();
    std::iter::from_fn(move || {
      let filter = origin_it.next()?;
      let (mut color_matrix, mut matrix) = match filter {
        FilterType::Color(color_matrix) => (Some(*color_matrix), None),
        FilterType::Convolution(matrix) => (None, Some(matrix)),
      };
      while let Some(next) = origin_it.peek() {
        match next {
          FilterType::Color(matrix) => {
            if let Some(color_matrix) = &mut color_matrix {
              color_matrix.chains(matrix);
            } else {
              color_matrix = Some(*matrix);
            }
          }
          FilterType::Convolution(m) if matrix.is_none() => {
            matrix = Some(m);
          }
          _ => break,
        };
        origin_it.next();
      }
      let matrix =
        matrix
          .cloned()
          .unwrap_or_else(|| FlattenMatrix { width: 1, height: 1, matrix: vec![1.] });

      if color_matrix.is_none() {
        color_matrix = Some(ColorMatrix::identity().to_matrix());
      }
      let ColorFilterMatrix { matrix: color_matrix, base_color } = color_matrix.unwrap();
      let FlattenMatrix { width, height, matrix, .. } = matrix;
      Some((
        FilterPrimitive {
          sample_offset: [0.; 2],
          mask_offset: [0.; 2],
          dummy: 0,
          color_matrix,
          base_color: base_color.map_or_else(|| [0.; 4], |c| c.into_f32_components()),
          kernel_size: [width as i32, height as i32],
          mask_head: self.mask_head,
        },
        matrix,
      ))
    })
  }
}

struct ClipLayer {
  viewport: DeviceRect,
  mask_head: i32,
}

/// Texture use to display.
pub trait Texture {
  type Host;
  /// clear a list of areas in the texture with zero.
  fn clear_areas(&mut self, areas: &[DeviceRect], backend: &mut Self::Host);

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

  fn begin_frame(&mut self, surface: Color) {
    self.surface_color = Some(surface);
    self.gpu_impl.begin_frame();
  }

  fn draw_commands(
    &mut self, viewport: DeviceRect, commands: &[PaintCommand], global_matrix: &Transform,
    output: &mut Self::Texture,
  ) {
    let clips = self.clip_layer_stack.len();
    self.viewport = viewport;
    self.begin_draw_phase();
    let output_size = output.size();
    for cmd in commands {
      self.draw_command(cmd, global_matrix, output_size, output);
    }
    self.draw_triangles(output);
    self.end_draw_phase();

    assert_eq!(self.clip_layer_stack.len(), clips);
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
  pub fn new(mut gpu_impl: Impl) -> Self {
    let tex_mgr = TexturesMgr::new(&mut gpu_impl);
    Self {
      gpu_impl,
      tex_mgr,
      tex_ids_map: <_>::default(),
      mask_layers: vec![],
      clip_layer_stack: vec![],
      skip_clip_cnt: 0,
      color_vertices_buffer: VertexBuffers::with_capacity(256, 512),
      img_vertices_buffer: VertexBuffers::with_capacity(256, 512),
      filter_vertices_buffer: VertexBuffers::with_capacity(16, 32),
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

  #[inline]
  pub fn into_impl(self) -> Impl { self.gpu_impl }

  fn draw_command(
    &mut self, cmd: &PaintCommand, global_matrix: &Transform, output_tex_size: DeviceSize,
    output: &mut Impl::Texture,
  ) {
    match cmd {
      PaintCommand::Path(cmd @ PathCommand { path, paint_bounds, transform, action }) => {
        if self.skip_clip_cnt > 0 {
          if matches!(action, PaintPathAction::Clip) {
            self.skip_clip_cnt += 1;
          }
          // Skip the commands if the clip layer is not visible.
          return;
        }
        let bounds = transform_to_device_rect(paint_bounds, global_matrix);

        let Some(viewport) = self.viewport().intersection(&bounds) else {
          if matches!(action, PaintPathAction::Clip) {
            self.skip_clip_cnt += 1;
          }
          // Skip the command if it is not visible.
          return;
        };

        if !self.can_batch_path_command(cmd) {
          self.new_draw_phase(output);
        }

        let matrix = transform.then(global_matrix);
        let (rect, mask_head) = match action {
          PaintPathAction::Clip => {
            self.new_mask_layer(&viewport, &matrix, path, &PaintingStyle::Fill)
          }
          PaintPathAction::Paint { painting_style, .. } => {
            self.new_mask_layer(&viewport, &matrix, path, painting_style)
          }
        };
        match action {
          PaintPathAction::Clip => self
            .clip_layer_stack
            .push(ClipLayer { viewport, mask_head }),
          PaintPathAction::Paint { brush, .. } => match brush {
            CommandBrush::Color(color) => {
              let color = color.into_components();
              let color_attr = ColorAttr { color, mask_head };
              let buffer = &mut self.color_vertices_buffer;
              add_rect_vertices(rect, output_tex_size, color_attr, buffer);
              self.current_phase = CurrentPhase::Color;
            }
            CommandBrush::Image { img, color_filter } => {
              let slice = self.tex_mgr.store_image(img, &mut self.gpu_impl);
              let ts = matrix.inverse().unwrap();
              self.draw_img_slice(slice, &ts, mask_head, color_filter, output_tex_size, rect);
            }
            CommandBrush::Radial(radial) => {
              let prim: RadialGradientPrimitive = RadialGradientPrimitive {
                transform: matrix.inverse().unwrap().to_array(),
                stop_start: self.radial_gradient_stops.len() as u32,
                stop_cnt: radial.stops.len() as u32,
                start_center: radial.start_center.to_array(),
                start_radius: radial.start_radius,
                end_center: radial.end_center.to_array(),
                end_radius: radial.end_radius,
                mask_head,
                spread: radial.spread_method as u32,
              };
              let stops = radial
                .stops
                .iter()
                .map(GradientStopPrimitive::new);
              self.radial_gradient_stops.extend(stops);
              let prim_idx = self.radial_gradient_prims.len() as u32;
              self.radial_gradient_prims.push(prim);
              let buffer = &mut self.radial_gradient_vertices_buffer;

              add_rect_vertices(rect, output_tex_size, RadialGradientPrimIndex(prim_idx), buffer);
              self.current_phase = CurrentPhase::RadialGradient;
            }
            CommandBrush::Linear(linear) => {
              let stop = (self.linear_gradient_stops.len() << 16 | linear.stops.len()) as u32;
              let mask_head_and_spread = mask_head << 16 | linear.spread_method as i32;
              let prim: LinearGradientPrimitive = LinearGradientPrimitive {
                transform: matrix.inverse().unwrap().to_array(),
                stop,
                start_position: linear.start.to_array(),
                end_position: linear.end.to_array(),
                mask_head_and_spread,
              };
              let stops = linear
                .stops
                .iter()
                .map(GradientStopPrimitive::new);
              self.linear_gradient_stops.extend(stops);
              let prim_idx = self.linear_gradient_prims.len() as u32;
              self.linear_gradient_prims.push(prim);
              let buffer = &mut self.linear_gradient_vertices_buffer;
              add_rect_vertices(rect, output_tex_size, LinearGradientPrimIndex(prim_idx), buffer);
              self.current_phase = CurrentPhase::LinearGradient;
            }
          },
        }
      }
      PaintCommand::PopClip => {
        if self.skip_clip_cnt > 0 {
          self.skip_clip_cnt -= 1;
        } else {
          self.clip_layer_stack.pop();
        }
      }
      PaintCommand::Bundle { transform, color_filter, bounds, cmds } => {
        let matrix = transform.then(global_matrix);
        let scale = self.tex_mgr.cache_scale(&bounds.size, &matrix);
        let cache_size = bounds.size * scale;

        let this = self as *mut Self;
        let (cache_scale, slice) = self.tex_mgr.store_commands(
          cache_size.to_i32().cast_unit(),
          cmds.clone().into_any(),
          scale,
          &mut self.gpu_impl,
          |slice, tex, _| {
            // SAFETY: We already hold a mut reference to the texture in the texture
            // manager, so we cant use `self` here, but this texture should always exist
            // within the frame, and no modifications will be made to the slice
            // that has already been allocated.
            let this = unsafe { &mut *this };

            // Initiate a new drawing phase to ensure a clean state for rendering in a new
            // texture.
            this.new_draw_phase(output);

            // store the viewport
            let viewport = self.viewport;
            // Overwrite the viewport to the slice bounds.
            self
              .clip_layer_stack
              .push(ClipLayer { viewport, mask_head: -1 });

            let matrix = Transform::translation(bounds.origin.x, bounds.origin.y)
              .then_scale(scale, scale)
              .then_translate(slice.origin.to_f32().cast_unit().to_vector());
            this.draw_commands(*slice, cmds, &matrix, tex);

            // restore the clip layer and viewport
            self.clip_layer_stack.pop();
            this.viewport = viewport;
            this.begin_draw_phase();
          },
        );

        let mut points: [_; 4] = rect_corners(&bounds.to_f32().cast_unit());
        for p in points.iter_mut() {
          *p = matrix.transform_point(*p);
        }

        let view_to_slice = matrix
          // point back to the bundle commands axis.
          .inverse()
          .unwrap()
          // align to the zero point, draw image slice is start from zero.
          .then_translate(Vector2D::new(-bounds.origin.x, -bounds.origin.y))
          // scale to the cache size.
          .then_scale(cache_scale, cache_scale);

        if !self.can_batch_img_path() {
          self.new_draw_phase(output);
        }
        let mask_head = self
          .clip_layer_stack
          .last()
          .map_or(-1, |l| l.mask_head);
        self.draw_img_slice(
          slice,
          &view_to_slice,
          mask_head,
          color_filter,
          output_tex_size,
          points,
        );
      }
      PaintCommand::Filter { path, path_bounds, transform, filters } => {
        let bounds = transform_to_device_rect(path_bounds, global_matrix);
        let Some(view_rect) = self.viewport().intersection(&bounds) else {
          return;
        };

        self.new_draw_phase(output);

        let ts = transform.then(global_matrix);
        let (_, mask_head) = self.new_mask_layer(&view_rect, &ts, path, &PaintingStyle::Fill);

        self.current_phase = CurrentPhase::Filter(Box::new(FilterPhase {
          view_rect,
          mask_head,
          filters: filters.clone(),
        }));
      }
    }
  }

  fn can_batch_img_path(&self) -> bool {
    let limits = self.gpu_impl.limits();
    matches!(self.current_phase, CurrentPhase::None)
      || (matches!(self.current_phase, CurrentPhase::Img)
        && self.tex_ids_map.len() < limits.max_tex_load - 1
        && self.img_prims.len() < limits.max_image_primitives)
  }

  // end current draw phase and start a new draw phase.
  fn new_draw_phase(&mut self, output: &mut Impl::Texture) {
    self.draw_triangles(output);
    self.end_draw_phase();
    self.begin_draw_phase();
  }

  fn begin_draw_phase(&mut self) {
    if !self.clip_layer_stack.is_empty() {
      // clear unused mask layers and update mask index.
      let mut retain_masks = Vec::with_capacity(self.clip_layer_stack.len());
      let mut mask_new_idx = vec![-1; self.mask_layers.len()];
      for s in self.clip_layer_stack.iter_mut() {
        if s.mask_head != -1 {
          retain_masks.push(s.mask_head);
          mask_new_idx[s.mask_head as usize] = retain_masks.len() as i32 - 1;
          s.mask_head = retain_masks.len() as i32 - 1;
        }
      }
      self.mask_layers = retain_masks
        .iter()
        .map(|&idx| {
          let mut mask = self.mask_layers[idx as usize].clone();
          if mask.prev_mask_idx != -1 {
            mask.prev_mask_idx = mask_new_idx[mask.prev_mask_idx as usize];
          }
          mask
        })
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
    self.current_phase = CurrentPhase::None;
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
    self.filter_vertices_buffer.vertices.clear();
    self.filter_vertices_buffer.indices.clear();
  }

  fn draw_img_slice(
    &mut self, img_slice: TextureSlice, transform: &Transform, mask_head: i32,
    color_filter: &ColorMatrix, output_tex_size: DeviceSize, rect: [Point; 4],
  ) {
    let img_start = img_slice.rect.origin.to_f32().to_array();
    let img_size = img_slice.rect.size.to_f32().to_array();
    let mask_head_and_tex_idx = mask_head << 16 | self.tex_ids_map.tex_idx(img_slice.tex_id) as i32;
    let prim_idx = self.img_prims.len() as u32;
    let ColorFilterMatrix { matrix, base_color } = color_filter.to_matrix();
    let base = base_color.map_or([0.; 4], |c| c.into_f32_components());

    let prim = ImgPrimitive {
      transform: transform.to_array(),
      img_start,
      img_size,
      mask_head_and_tex_idx,
      color_matrix: matrix,
      base_color: base,
      dummy: 0,
    };
    self.img_prims.push(prim);
    let buffer = &mut self.img_vertices_buffer;
    add_rect_vertices(rect, output_tex_size, ImagePrimIndex(prim_idx), buffer);
    self.current_phase = CurrentPhase::Img;
  }

  fn can_batch_path_command(&self, cmd: &PathCommand) -> bool {
    let limits = self.gpu_impl.limits();
    let tex_used = self.tex_ids_map.len();

    if matches!(self.current_phase, CurrentPhase::Filter(_)) {
      return false;
    }

    let PaintPathAction::Paint { brush, .. } = &cmd.action else {
      return tex_used < limits.max_tex_load;
    };

    match (&self.current_phase, brush) {
      (CurrentPhase::None, _) => true,
      (CurrentPhase::Filter(_), _) => false,
      (CurrentPhase::Color, CommandBrush::Color(_)) => tex_used < limits.max_tex_load,
      (CurrentPhase::Img, CommandBrush::Image { .. }) => {
        tex_used < limits.max_tex_load - 1 && self.img_prims.len() < limits.max_image_primitives
      }
      (CurrentPhase::RadialGradient, CommandBrush::Radial(_)) => {
        tex_used < limits.max_tex_load
          && self.radial_gradient_prims.len() < limits.max_radial_gradient_primitives
          && self.radial_gradient_stops.len() < limits.max_gradient_stop_primitives
      }
      (CurrentPhase::LinearGradient, CommandBrush::Linear(_)) => {
        tex_used < limits.max_tex_load
          && self.linear_gradient_prims.len() < limits.max_linear_gradient_primitives
          && self.linear_gradient_stops.len() < limits.max_gradient_stop_primitives
      }
      _ => false,
    }
  }

  fn current_clip_mask_index(&self) -> i32 {
    self
      .clip_layer_stack
      .last()
      .map_or(-1, |l| l.mask_head)
  }

  fn viewport(&self) -> &DeviceRect {
    self
      .clip_layer_stack
      .last()
      .map_or(&self.viewport, |l| &l.viewport)
  }

  fn new_mask_layer(
    &mut self, view: &DeviceRect, matrix: &Transform, path: &PaintPath, style: &PaintingStyle,
  ) -> ([Point; 4], i32) {
    let (mask, mask_to_view) =
      self
        .tex_mgr
        .store_alpha_path(path, style, matrix, view, &mut self.gpu_impl);

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
    (points, index as i32)
  }

  fn draw_triangles(&mut self, output: &mut Impl::Texture) {
    let mut color = self.surface_color.take();
    let gpu_impl = &mut self.gpu_impl;

    self.tex_mgr.draw_alpha_textures(gpu_impl);
    if !self.mask_layers.is_empty() {
      gpu_impl.load_mask_layers(&self.mask_layers);
    }

    let textures = self.tex_ids_map.all_textures();
    let max_textures = gpu_impl.limits().max_tex_load;
    let mut tex_buffer = Vec::with_capacity(max_textures);
    if textures.is_empty() {
      tex_buffer.push(self.tex_mgr.texture(TextureID::Alpha(0)));
    } else {
      textures.iter().take(max_textures).for_each(|id| {
        tex_buffer.push(self.tex_mgr.texture(*id));
      });
    }
    gpu_impl.load_textures(&tex_buffer);

    let current_phase = std::mem::replace(&mut self.current_phase, CurrentPhase::None);
    match current_phase {
      CurrentPhase::None => {
        if color.is_some() {
          gpu_impl.draw_color_triangles(output, 0..0, color.take())
        }
      }
      CurrentPhase::Color if !self.color_vertices_buffer.indices.is_empty() => {
        gpu_impl.load_color_vertices(&self.color_vertices_buffer);
        let rg = 0..self.color_vertices_buffer.indices.len() as u32;
        gpu_impl.draw_color_triangles(output, rg, color.take())
      }
      CurrentPhase::Img if !self.img_vertices_buffer.indices.is_empty() => {
        gpu_impl.load_img_primitives(&self.img_prims);
        gpu_impl.load_img_vertices(&self.img_vertices_buffer);
        let rg = 0..self.img_vertices_buffer.indices.len() as u32;
        gpu_impl.draw_img_triangles(output, rg, color.take())
      }
      CurrentPhase::RadialGradient
        if !self
          .radial_gradient_vertices_buffer
          .indices
          .is_empty() =>
      {
        gpu_impl.load_radial_gradient_primitives(&self.radial_gradient_prims);
        gpu_impl.load_radial_gradient_stops(&self.radial_gradient_stops);
        gpu_impl.load_radial_gradient_vertices(&self.radial_gradient_vertices_buffer);
        let rg = 0..self.radial_gradient_vertices_buffer.indices.len() as u32;
        gpu_impl.draw_radial_gradient_triangles(output, rg, color.take())
      }
      CurrentPhase::LinearGradient
        if !self
          .linear_gradient_vertices_buffer
          .indices
          .is_empty() =>
      {
        gpu_impl.load_linear_gradient_primitives(&self.linear_gradient_prims);
        gpu_impl.load_linear_gradient_stops(&self.linear_gradient_stops);
        gpu_impl.load_linear_gradient_vertices(&self.linear_gradient_vertices_buffer);
        let rg = 0..self.linear_gradient_vertices_buffer.indices.len() as u32;
        gpu_impl.draw_linear_gradient_triangles(output, rg, color.take())
      }
      CurrentPhase::Filter(filters) => {
        self.draw_filter(&filters, output);
      }
      _ => {}
    }
  }

  fn draw_filter(&mut self, filter: &FilterPhase, output: &mut Impl::Texture) {
    let mut tex_tmp = self
      .gpu_impl
      .new_texture(filter.view_rect.size, ColorFormat::Rgba8);

    let mut p_src_tex = output as *mut _;
    let mut p_dst_tex = &mut tex_tmp as *mut Impl::Texture;
    let mask_origin = filter.view_rect.origin.cast().cast_unit();
    let mut src_origin = filter.view_rect.origin.cast().cast_unit();
    let mut dst_origin = Point::new(0_f32, 0.);
    let primitives = filter.primitive_iter();

    for (mut prim, kernel) in primitives {
      prim.sample_offset = (src_origin - dst_origin).into();
      prim.mask_offset = (mask_origin - dst_origin).into();

      let dst_tex = unsafe { &mut *p_dst_tex };
      let origin = unsafe { &*p_src_tex };

      self
        .gpu_impl
        .load_filter_primitive(&prim, &kernel);

      self.filter_vertices_buffer.clear();
      let dst_size = dst_tex.size();

      add_rect_vertices(
        [
          dst_origin,
          dst_origin + Vector2D::new(dst_size.width as f32, 0.),
          dst_origin + Vector2D::new(dst_size.width as f32, dst_size.height as f32),
          dst_origin + Vector2D::new(0., dst_size.height as f32),
        ],
        dst_size,
        (),
        &mut self.filter_vertices_buffer,
      );

      self
        .gpu_impl
        .load_filter_vertices(&self.filter_vertices_buffer);
      let indices = 0..self.filter_vertices_buffer.indices.len() as u32;
      self
        .gpu_impl
        .draw_filter_triangles(dst_tex, origin, indices, None);

      std::mem::swap(&mut p_dst_tex, &mut p_src_tex);
      std::mem::swap(&mut dst_origin, &mut src_origin);
    }

    if std::ptr::eq(p_dst_tex, output) {
      self.gpu_impl.copy_texture_from_texture(
        output,
        dst_origin.cast().cast_unit(),
        &tex_tmp,
        &DeviceRect::from_size(tex_tmp.size()),
      );
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

  fn len(&self) -> usize { self.textures.len() }
}

pub fn vertices_coord(pos: Point, tex_size: DeviceSize) -> [f32; 2] {
  [pos.x / tex_size.width as f32, pos.y / tex_size.height as f32]
}

pub fn add_rect_vertices<Attr: Copy>(
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
  use ribir_algo::Resource;
  use ribir_dev_helper::*;
  use ribir_geom::*;
  use ribir_painter::{Brush, Painter, Path, Svg};

  use super::*;

  fn painter(bounds: Size) -> Painter { Painter::new(Rect::from_size(bounds)) }

  painter_backend_eq_image_test!(smoke, comparison = 0.001);
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
    let share_img = Resource::new(img);

    let img_brush = Brush::Image(share_img);

    draw_arrow_path(&mut painter);
    painter.set_fill_brush(Color::RED).fill();

    painter.translate(260., 0.);
    draw_arrow_path(&mut painter);
    painter
      .set_stroke_brush(Color::RED)
      .set_line_width(5.)
      .stroke();

    painter.translate(-260., 250.);
    draw_arrow_path(&mut painter);
    painter.set_fill_brush(img_brush.clone()).fill();

    painter.translate(260., 0.);
    draw_arrow_path(&mut painter);
    painter
      .set_stroke_brush(img_brush)
      .set_line_width(5.)
      .stroke();

    painter
  }

  painter_backend_eq_image_test!(transform_img_brush, comparison = 0.001);
  fn transform_img_brush() -> Painter {
    let mut painter = painter(Size::new(800., 250.));

    let transform = Transform::new(1., 1., 2., 1., 0., 0.);
    let rect: Rect = Rect::new(Point::new(10., 10.), Size::new(100., 100.));
    painter
      .set_fill_brush(Color::RED)
      .set_transform(transform)
      .rect(&rect)
      .fill();

    let leaves_brush = Resource::new(PixelImage::from_png(include_bytes!("../imgs/leaves.png")));

    painter
      .set_fill_brush(leaves_brush)
      .set_transform(transform.then_translate((400., 0.).into()))
      .rect(&rect)
      .fill();

    painter
  }

  painter_backend_eq_image_test!(clip_layers, comparison = 0.0065);
  fn clip_layers() -> Painter {
    let mut painter = painter(Size::new(120., 340.));
    let rect_100x100 = Rect::from_size(Size::new(100., 100.));
    painter
      .set_fill_brush(Color::RED)
      .translate(10., 20.)
      .rect(&rect_100x100)
      .fill()
      .translate(0., 200.)
      .clip(Path::circle(Point::new(50., 50.), 50.).into())
      .rect(&rect_100x100)
      .fill();

    painter
  }

  painter_backend_eq_image_test!(stroke_include_border, comparison = 0.0004);
  fn stroke_include_border() -> Painter {
    let mut painter = painter(Size::new(100., 100.));
    painter
      .set_stroke_brush(Color::RED)
      .begin_path(Point::new(50., 5.))
      .line_to(Point::new(95., 50.))
      .line_to(Point::new(50., 95.))
      .line_to(Point::new(5., 50.))
      .end_path(true)
      .set_line_width(10.)
      .stroke();
    painter
  }

  painter_backend_eq_image_test!(two_img_brush, comparison = 0.006);
  fn two_img_brush() -> Painter {
    let mut painter = painter(Size::new(200., 100.));

    let brush1 = PixelImage::from_png(include_bytes!("../imgs/leaves.png"));
    let brush2 = PixelImage::from_png(include_bytes!("../../examples/attachments/3DDD-1.png"));
    let rect = rect(0., 0., 100., 100.);
    painter
      .set_fill_brush(brush1)
      .rect(&rect)
      .fill()
      .set_fill_brush(brush2)
      .translate(100., 0.)
      .clip(Path::circle(Point::new(50., 50.), 50.).into())
      .rect(&rect)
      .fill();

    painter
  }

  painter_backend_eq_image_test!(draw_partial_img, comparison = 0.0015);
  fn draw_partial_img() -> Painter {
    let img = Resource::new(PixelImage::from_png(include_bytes!("../imgs/leaves.png")));
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

  painter_backend_eq_image_test!(draw_svg_gradient, comparison = 0.0025);
  fn draw_svg_gradient() -> Painter {
    let mut painter = painter(Size::new(64., 64.));
    let svg = Svg::parse_from_bytes(
      include_bytes!("../../tests/assets/fill_with_gradient.svg"),
      true,
      false,
    )
    .unwrap();

    painter.draw_svg(&svg);
    painter
  }

  // This test is disabled on Windows as it fails in the CI environment (exit code
  // 2173), although it passes on a physical Windows machine.
  #[cfg(not(target_os = "windows"))]
  fn multi_draw_phase() -> Painter {
    let mut painter = painter(Size::new(1048., 1048.));

    let rect = Rect::from_size(Size::new(1024., 1024.));
    for i in 0..100 {
      let mut painter = painter.save_guard();
      painter.translate(i as f32 * 10., i as f32 * 10.);
      let color = if i % 2 == 0 { Color::GREEN } else { Color::RED };
      painter
        .set_fill_brush(color)
        .rect_round(&rect, &ribir_painter::Radius::all(i as f32))
        .fill();
    }
    painter
  }
  #[cfg(not(target_os = "windows"))]
  painter_backend_eq_image_test!(multi_draw_phase, comparison = 0.001);

  fn draw_bundle_svg() -> Painter {
    let mut painter = painter(Size::new(512., 512.));
    let circle = Resource::new(Path::circle(Point::new(4., 4.), 100.));
    let commands = (0..64)
      .map(|i| {
        let color = if i % 2 == 0 { Color::GREEN } else { Color::RED };
        PaintCommand::Path(PathCommand {
          paint_bounds: circle.bounds(None),
          path: circle.clone().into(),
          transform: Transform::translation(i as f32 * 8., i as f32 * 8.),
          action: PaintPathAction::Paint {
            brush: CommandBrush::Color(color),
            painting_style: PaintingStyle::Fill,
          },
        })
      })
      .collect();

    painter.draw_bundle_commands(Rect::from_size(Size::new(512., 512.)), Resource::new(commands));
    painter
  }
  painter_backend_eq_image_test!(draw_bundle_svg, comparison = 0.001);
}
