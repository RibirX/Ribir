use std::{
  error::Error,
  mem::{MaybeUninit, size_of},
  ops::Range,
};

use ahash::HashSet;
use ribir_geom::{DevicePoint, DeviceRect, DeviceSize};
use ribir_painter::{Color, ColorFormat, PixelImage, VertexBuffers};
use tokio::sync::oneshot;
use tracing::debug;
use wgpu::TextureFormat;
use zerocopy::AsBytes;

use self::{
  draw_alpha_triangles_pass::DrawAlphaTrianglesPass,
  draw_color_triangles_pass::DrawColorTrianglesPass,
  draw_img_triangles_pass::DrawImgTrianglesPass,
  draw_linear_gradient_pass::DrawLinearGradientTrianglesPass,
  draw_radial_gradient_pass::DrawRadialGradientTrianglesPass,
  primitive_pool::{PrimitivePool, PrimitivePoolMode},
  texture_pass::{ClearTexturePass, CopyTexturePass},
  uniform::Uniform,
};
use crate::{
  ColorAttr, DrawPhaseLimits, FilterPrimitive, GPUBackendImpl, GradientStopPrimitive,
  ImagePrimIndex, ImgPrimitive, LinearGradientPrimIndex, LinearGradientPrimitive, MaskLayer,
  RadialGradientPrimIndex, RadialGradientPrimitive, TexturePrimIndex, TexturePrimitive,
  gpu_backend::Texture,
  wgpu_impl::{draw_filter_pass::DrawFilterPass, draw_texture_pass::DrawTexturePass},
};
mod primitive_pool;
mod shaders;
mod uniform;
mod vertex_buffer;

mod draw_alpha_triangles_pass;
mod draw_color_triangles_pass;
mod draw_filter_pass;
mod draw_img_triangles_pass;
mod draw_linear_gradient_pass;
mod draw_radial_gradient_pass;
mod draw_texture_pass;
mod texture_pass;

pub const TEX_PER_DRAW: usize = 8;

pub struct WgpuImpl {
  device: wgpu::Device,
  queue: wgpu::Queue,

  command_encoder: Option<wgpu::CommandEncoder>,
  command_buffers: Vec<wgpu::CommandBuffer>,

  sampler: wgpu::Sampler,
  clear_tex_pass: ClearTexturePass,
  alpha_triangles_pass: DrawAlphaTrianglesPass,
  copy_tex_pass: Option<CopyTexturePass>,
  color_triangles_pass: Option<DrawColorTrianglesPass>,
  img_triangles_pass: Option<DrawImgTrianglesPass>,
  radial_gradient_pass: Option<DrawRadialGradientTrianglesPass>,
  linear_gradient_pass: Option<DrawLinearGradientTrianglesPass>,
  filter_pass: Option<DrawFilterPass>,
  draw_texture_pass: Option<DrawTexturePass>,
  texs_layout: wgpu::BindGroupLayout,
  textures_bind: Option<wgpu::BindGroup>,
  mask_layers_uniform: Uniform<MaskLayer>,
  slot0_pool: PrimitivePool,
  slot1_pool: PrimitivePool,
  /// Mode for draw passes that use both slot0 (primitives) and slot1 (stops):
  /// color, radial-gradient, linear-gradient.
  dual_slot_mode: PrimitivePoolMode,
  /// Mode for draw passes that use only slot0: img, filter, texture.
  slot0_only_mode: PrimitivePoolMode,
  limits: DrawPhaseLimits,
  surface_format: Option<wgpu::TextureFormat>,
}

macro_rules! command_encoder {
  ($backend:ident) => {
    $backend.command_encoder.get_or_insert_with(|| {
      $backend
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Frame Encoder") })
    })
  };
}
macro_rules! color_pass {
  ($backend:ident) => {{
    let slot0_layout = $backend.slot0_pool.layout();
    let slot1_layout = $backend.slot1_pool.layout();

    $backend
      .color_triangles_pass
      .get_or_insert_with(|| {
        DrawColorTrianglesPass::new(
          &$backend.device,
          $backend.mask_layers_uniform.layout(),
          &$backend.texs_layout,
          slot0_layout,
          slot1_layout,
          $backend.limits.max_mask_layers,
        )
      })
  }};
}

macro_rules! img_pass {
  ($backend:ident) => {{
    let slot0_layout = $backend.slot0_pool.layout();

    $backend
      .img_triangles_pass
      .get_or_insert_with(|| {
        DrawImgTrianglesPass::new(
          &$backend.device,
          $backend.mask_layers_uniform.layout(),
          &$backend.texs_layout,
          slot0_layout,
          $backend.slot1_pool.layout(),
          $backend.slot0_only_mode,
          &$backend.limits,
        )
      })
  }};
}

macro_rules! radial_gradient_pass {
  ($backend:ident) => {{
    let slot0_layout = $backend.slot0_pool.layout();
    let slot1_layout = $backend.slot1_pool.layout();

    $backend
      .radial_gradient_pass
      .get_or_insert_with(|| {
        DrawRadialGradientTrianglesPass::new(
          &$backend.device,
          $backend.mask_layers_uniform.layout(),
          &$backend.texs_layout,
          slot0_layout,
          slot1_layout,
          $backend.dual_slot_mode,
          &$backend.limits,
        )
      })
  }};
}

macro_rules! linear_gradient_pass {
  ($backend:ident) => {{
    let slot0_layout = $backend.slot0_pool.layout();
    let slot1_layout = $backend.slot1_pool.layout();

    $backend
      .linear_gradient_pass
      .get_or_insert_with(|| {
        DrawLinearGradientTrianglesPass::new(
          &$backend.device,
          $backend.mask_layers_uniform.layout(),
          &$backend.texs_layout,
          slot0_layout,
          slot1_layout,
          $backend.dual_slot_mode,
          &$backend.limits,
        )
      })
  }};
}

macro_rules! filter_pass {
  ($backend:ident) => {{
    let slot0_layout = $backend.slot0_pool.layout();

    $backend.filter_pass.get_or_insert_with(|| {
      DrawFilterPass::new(
        &$backend.device,
        $backend.mask_layers_uniform.layout(),
        &$backend.texs_layout,
        slot0_layout,
        $backend.slot0_only_mode,
        &$backend.limits,
      )
    })
  }};
}

macro_rules! draw_texture_pass {
  ($backend:ident) => {{
    let slot0_layout = $backend.slot0_pool.layout();

    $backend.draw_texture_pass.get_or_insert_with(|| {
      DrawTexturePass::new(
        &$backend.device,
        $backend.mask_layers_uniform.layout(),
        &$backend.texs_layout,
        slot0_layout,
        $backend.slot0_only_mode,
        &$backend.limits,
      )
    })
  }};
}

pub(crate) use command_encoder;

pub struct Surface<'a> {
  surface: wgpu::Surface<'a>,
  config: wgpu::SurfaceConfiguration,
  current_texture: Option<WgpuTexture>,
}

impl WgpuImpl {
  pub(crate) fn create_texture(&mut self, size: DeviceSize, format: TextureFormat) -> WgpuTexture {
    let size = wgpu::Extent3d {
      width: size.width as u32,
      height: size.height as u32,
      depth_or_array_layers: 1,
    };
    let texture_descriptor = &wgpu::TextureDescriptor {
      label: Some("Create wgpu texture"),
      size,
      dimension: wgpu::TextureDimension::D2,
      format,
      usage: wgpu::TextureUsages::COPY_SRC
        | wgpu::TextureUsages::COPY_DST
        | wgpu::TextureUsages::TEXTURE_BINDING
        | wgpu::TextureUsages::RENDER_ATTACHMENT,
      mip_level_count: 1,
      sample_count: 1,
      view_formats: &[],
    };
    let tex = self.device.create_texture(texture_descriptor);
    WgpuTexture::from_tex(tex)
  }

  fn atomic_flush(&mut self) { self.flush_and_reset(); }

  fn load_or_atomic_flush<T, F>(&mut self, mut load: F) -> T
  where
    F: FnMut(&mut Self) -> Option<T>,
  {
    if let Some(v) = load(self) {
      return v;
    }

    self.atomic_flush();
    let retried = load(self);
    debug_assert!(retried.is_some(), "load_or_atomic_flush retry failed after flush");
    retried.unwrap_or_else(|| panic!("load_or_atomic_flush retry failed after flush"))
  }

  /// Reset offsets for draw passes and primitive pools.
  /// Note: does NOT reset `mask_layers_uniform`, which is managed
  /// separately by `load_mask_layers` and `begin_frame`.
  fn reset_draw_offsets(&mut self) {
    self.reset_core_offsets();
    self.reset_optional_pass_offsets();
  }

  fn flush_and_reset(&mut self) {
    self.submit();
    self.reset_draw_offsets();
  }

  fn reset_core_offsets(&mut self) {
    self.alpha_triangles_pass.reset();
    self.slot0_pool.reset();
    self.slot1_pool.reset();
  }

  fn reset_optional_pass_offsets(&mut self) {
    if let Some(p) = self.color_triangles_pass.as_mut() {
      p.reset();
    }
    if let Some(p) = self.img_triangles_pass.as_mut() {
      p.reset();
    }
    if let Some(p) = self.radial_gradient_pass.as_mut() {
      p.reset();
    }
    if let Some(p) = self.linear_gradient_pass.as_mut() {
      p.reset();
    }
    if let Some(p) = self.filter_pass.as_mut() {
      p.reset();
    }
    if let Some(p) = self.draw_texture_pass.as_mut() {
      p.reset();
    }
  }

  fn try_load_alpha_vertices(&mut self, buffers: &VertexBuffers<()>) -> Option<()> {
    self
      .alpha_triangles_pass
      .load_alpha_vertices(buffers, &self.device, &self.queue)
  }

  fn try_load_alpha_size(&mut self, size: DeviceSize) -> Option<u32> {
    self
      .alpha_triangles_pass
      .load_size(&self.queue, size.to_u32().to_array())
  }

  fn try_load_color_vertices(&mut self, buffers: &VertexBuffers<ColorAttr>) -> Option<()> {
    color_pass!(self).load_triangles_vertices(buffers, &self.device, &self.queue)
  }

  fn try_load_img_vertices(&mut self, buffers: &VertexBuffers<ImagePrimIndex>) -> Option<()> {
    img_pass!(self).load_triangles_vertices(buffers, &self.device, &self.queue)
  }

  fn try_load_img_primitives(&mut self, primitives: &[ImgPrimitive]) -> Option<u32> {
    self
      .slot0_pool
      .write_typed_slice(&self.queue, primitives)
      .map(|slice| {
        let _ = self.slot0_pool.index_base(slice);
        self.slot0_pool.resolve_load_offset(slice)
      })
  }

  fn try_load_slot0_img_primitives(&mut self, primitives: &[ImgPrimitive]) -> Option<u32> {
    self.try_load_img_primitives(primitives)
  }

  fn try_load_filter_vertices(&mut self, buffers: &VertexBuffers<()>) -> Option<()> {
    filter_pass!(self).load_triangles_vertices(buffers, &self.device, &self.queue)
  }

  fn try_load_radial_gradient_primitives(
    &mut self, primitives: &[RadialGradientPrimitive],
  ) -> Option<u32> {
    self
      .slot0_pool
      .write_typed_slice(&self.queue, primitives)
      .map(|slice| {
        let _ = self.slot0_pool.index_base(slice);
        self.slot0_pool.resolve_load_offset(slice)
      })
  }

  fn try_load_slot0_radial_gradient_primitives(
    &mut self, primitives: &[RadialGradientPrimitive],
  ) -> Option<u32> {
    self.try_load_radial_gradient_primitives(primitives)
  }

  fn try_load_radial_gradient_stops(&mut self, stops: &[GradientStopPrimitive]) -> Option<u32> {
    self
      .slot1_pool
      .write_typed_slice(&self.queue, stops)
      .map(|slice| {
        let _ = self.slot1_pool.index_base(slice);
        self.slot1_pool.resolve_load_offset(slice)
      })
  }

  fn try_load_slot1_radial_gradient_stops(
    &mut self, stops: &[GradientStopPrimitive],
  ) -> Option<u32> {
    self.try_load_radial_gradient_stops(stops)
  }

  fn try_load_radial_gradient_vertices(
    &mut self, buffers: &VertexBuffers<RadialGradientPrimIndex>,
  ) -> Option<()> {
    radial_gradient_pass!(self).load_triangles_vertices(buffers, &self.device, &self.queue)
  }

  fn try_load_linear_gradient_primitives(
    &mut self, primitives: &[LinearGradientPrimitive],
  ) -> Option<u32> {
    self
      .slot0_pool
      .write_typed_slice(&self.queue, primitives)
      .map(|slice| {
        let _ = self.slot0_pool.index_base(slice);
        self.slot0_pool.resolve_load_offset(slice)
      })
  }

  fn try_load_slot0_linear_gradient_primitives(
    &mut self, primitives: &[LinearGradientPrimitive],
  ) -> Option<u32> {
    self.try_load_linear_gradient_primitives(primitives)
  }

  fn try_load_linear_gradient_stops(&mut self, stops: &[GradientStopPrimitive]) -> Option<u32> {
    self
      .slot1_pool
      .write_typed_slice(&self.queue, stops)
      .map(|slice| {
        let _ = self.slot1_pool.index_base(slice);
        self.slot1_pool.resolve_load_offset(slice)
      })
  }

  fn try_load_slot1_linear_gradient_stops(
    &mut self, stops: &[GradientStopPrimitive],
  ) -> Option<u32> {
    self.try_load_linear_gradient_stops(stops)
  }

  fn try_load_linear_gradient_vertices(
    &mut self, buffers: &VertexBuffers<LinearGradientPrimIndex>,
  ) -> Option<()> {
    linear_gradient_pass!(self).load_triangles_vertices(buffers, &self.device, &self.queue)
  }

  fn try_load_filter_primitive(
    &mut self, primitive: &FilterPrimitive, kernel_matrix: &[f32],
  ) -> Option<u32> {
    let total_bytes = (size_of::<FilterPrimitive>() + std::mem::size_of_val(kernel_matrix)) as u64;
    if self.slot0_pool.needs_flush(total_bytes) {
      return None;
    }
    self
      .slot0_pool
      .write_at(&self.queue, 0, primitive.as_bytes());
    self.slot0_pool.write_at(
      &self.queue,
      size_of::<FilterPrimitive>() as u64,
      kernel_matrix.as_bytes(),
    );
    Some(self.slot0_pool.advance(total_bytes))
  }

  fn try_load_slot0_filter_primitive(
    &mut self, primitive: &FilterPrimitive, kernel_matrix: &[f32],
  ) -> Option<u32> {
    self.try_load_filter_primitive(primitive, kernel_matrix)
  }

  fn try_load_mask_layers(&mut self, layers: &[crate::MaskLayer]) -> Option<u32> {
    self
      .mask_layers_uniform
      .write_buffer(&self.queue, layers)
  }

  fn try_load_group0_mask_layers(&mut self, layers: &[crate::MaskLayer]) -> Option<u32> {
    self.try_load_mask_layers(layers)
  }

  fn try_load_texture_vertices(&mut self, buffers: &VertexBuffers<TexturePrimIndex>) -> Option<()> {
    draw_texture_pass!(self).load_triangles_vertices(buffers, &self.device, &self.queue)
  }

  fn try_load_texture_primitives(&mut self, primitives: &[TexturePrimitive]) -> Option<u32> {
    self
      .slot0_pool
      .write_typed_slice(&self.queue, primitives)
      .map(|slice| {
        let _ = self.slot0_pool.index_base(slice);
        self.slot0_pool.resolve_load_offset(slice)
      })
  }

  fn try_load_slot0_texture_primitives(&mut self, primitives: &[TexturePrimitive]) -> Option<u32> {
    self.try_load_texture_primitives(primitives)
  }
}

impl GPUBackendImpl for WgpuImpl {
  type Texture = WgpuTexture;

  fn limits(&self) -> &DrawPhaseLimits { &self.limits }

  fn begin_frame(&mut self) {
    self.reset_draw_offsets();
    self.mask_layers_uniform.reset();

    if self.command_encoder.is_none() {
      #[cfg(debug_assertions)]
      self.start_capture();
    }
  }

  fn new_texture(&mut self, size: DeviceSize, format: ColorFormat) -> Self::Texture {
    let mut wgpu_format = into_wgpu_format(format);
    if let Some(s_fmt) = self.surface_format
      && try_into_color_format(s_fmt) == Some(format)
    {
      wgpu_format = s_fmt;
    }
    self.create_texture(size, wgpu_format)
  }

  fn load_textures(&mut self, textures: &[&Self::Texture]) {
    self.textures_bind =
      Some(textures_bind(&self.device, &self.sampler, &self.texs_layout, textures));
  }

  fn load_alpha_vertices(&mut self, buffers: &VertexBuffers<()>) {
    self.load_or_atomic_flush(|backend| backend.try_load_alpha_vertices(buffers));
  }

  fn load_alpha_size(&mut self, size: DeviceSize) -> u32 {
    self.load_or_atomic_flush(|backend| backend.try_load_alpha_size(size))
  }

  fn load_color_vertices(&mut self, buffers: &VertexBuffers<ColorAttr>) {
    self.load_or_atomic_flush(|backend| backend.try_load_color_vertices(buffers));
  }

  fn load_img_data(
    &mut self, primitives: &[ImgPrimitive], buffers: &VertexBuffers<ImagePrimIndex>,
  ) -> u32 {
    let load = |b: &mut Self| {
      let offset = b.try_load_slot0_img_primitives(primitives)?;
      b.try_load_img_vertices(buffers)?;
      Some(offset)
    };
    if let Some(offset) = load(self) {
      return offset;
    }
    self.atomic_flush();
    load(self).unwrap()
  }

  fn load_radial_gradient_data(
    &mut self, primitives: &[RadialGradientPrimitive], stops: &[GradientStopPrimitive],
    buffers: &VertexBuffers<RadialGradientPrimIndex>,
  ) -> (u32, u32) {
    let load = |b: &mut Self| {
      let p_offset = b.try_load_slot0_radial_gradient_primitives(primitives)?;
      let s_offset = b.try_load_slot1_radial_gradient_stops(stops)?;
      b.try_load_radial_gradient_vertices(buffers)?;
      Some((p_offset, s_offset))
    };
    if let Some(offsets) = load(self) {
      return offsets;
    }
    self.atomic_flush();
    load(self).unwrap()
  }

  fn load_linear_gradient_data(
    &mut self, primitives: &[LinearGradientPrimitive], stops: &[GradientStopPrimitive],
    buffers: &VertexBuffers<LinearGradientPrimIndex>,
  ) -> (u32, u32) {
    let load = |b: &mut Self| {
      let p_offset = b.try_load_slot0_linear_gradient_primitives(primitives)?;
      let s_offset = b.try_load_slot1_linear_gradient_stops(stops)?;
      b.try_load_linear_gradient_vertices(buffers)?;
      Some((p_offset, s_offset))
    };
    if let Some(offsets) = load(self) {
      return offsets;
    }
    self.atomic_flush();
    load(self).unwrap()
  }

  fn load_filter_data(
    &mut self, primitive: &FilterPrimitive, kernel_matrix: &[f32], buffers: &VertexBuffers<()>,
  ) -> u32 {
    let load = |b: &mut Self| {
      let offset = b.try_load_slot0_filter_primitive(primitive, kernel_matrix)?;
      b.try_load_filter_vertices(buffers)?;
      Some(offset)
    };
    if let Some(offset) = load(self) {
      return offset;
    }
    self.atomic_flush();
    load(self).unwrap()
  }

  fn load_mask_layers(&mut self, layers: &[crate::MaskLayer]) -> u32 {
    let load = |b: &mut Self| b.try_load_group0_mask_layers(layers);
    if let Some(offset) = load(self) {
      return offset;
    }

    self.atomic_flush();
    self.mask_layers_uniform.reset();
    load(self).unwrap()
  }

  fn draw_alpha_triangles(
    &mut self, indices: &Range<u32>, texture: &mut Self::Texture, size_offset: u32,
  ) {
    let encoder = command_encoder!(self);
    self.alpha_triangles_pass.draw_alpha_triangles(
      indices,
      texture,
      None,
      &self.queue,
      encoder,
      size_offset,
    );
  }

  fn draw_radial_gradient_triangles(
    &mut self, texture: &mut Self::Texture, indices: Range<u32>, clear: Option<Color>,
    mask_offset: u32, prims_offset: u32, stops_offset: u32,
  ) {
    let slot0_bind = self.slot0_pool.bind_group();
    let slot1_bind = self.slot1_pool.bind_group();
    let encoder = command_encoder!(self);

    radial_gradient_pass!(self).draw_triangles(
      texture,
      indices,
      clear,
      &self.device,
      encoder,
      self.textures_bind.as_ref().unwrap(),
      &self.mask_layers_uniform,
      slot0_bind,
      slot1_bind,
      mask_offset,
      self.slot0_pool.bind_offset(prims_offset),
      self.slot1_pool.bind_offset(stops_offset),
    );
  }

  fn draw_linear_gradient_triangles(
    &mut self, texture: &mut Self::Texture, indices: Range<u32>, clear: Option<Color>,
    mask_offset: u32, prims_offset: u32, stops_offset: u32,
  ) {
    let slot0_bind = self.slot0_pool.bind_group();
    let slot1_bind = self.slot1_pool.bind_group();
    let encoder = command_encoder!(self);

    linear_gradient_pass!(self).draw_triangles(
      texture,
      indices,
      clear,
      &self.device,
      encoder,
      self.textures_bind.as_ref().unwrap(),
      &self.mask_layers_uniform,
      slot0_bind,
      slot1_bind,
      mask_offset,
      self.slot0_pool.bind_offset(prims_offset),
      self.slot1_pool.bind_offset(stops_offset),
    );
  }

  fn draw_alpha_triangles_with_scissor(
    &mut self, indices: &Range<u32>, texture: &mut Self::Texture, scissor: DeviceRect,
    size_offset: u32,
  ) {
    let encoder = command_encoder!(self);
    self.alpha_triangles_pass.draw_alpha_triangles(
      indices,
      texture,
      Some(scissor),
      &self.queue,
      encoder,
      size_offset,
    );
  }

  fn draw_color_triangles(
    &mut self, texture: &mut Self::Texture, indices: Range<u32>, clear: Option<Color>,
    mask_offset: u32,
  ) {
    let slot0_bind = self.slot0_pool.bind_group();
    let slot1_bind = self.slot1_pool.bind_group();
    let encoder = command_encoder!(self);

    color_pass!(self).draw_triangles(
      texture,
      indices,
      clear,
      &self.device,
      encoder,
      self.textures_bind.as_ref().unwrap(),
      &self.mask_layers_uniform,
      slot0_bind,
      slot1_bind,
      mask_offset,
    );
  }

  fn draw_img_triangles(
    &mut self, texture: &mut Self::Texture, indices: Range<u32>, clear: Option<Color>,
    mask_offset: u32, prims_offset: u32,
  ) {
    let slot0_bind = self.slot0_pool.bind_group();
    let encoder = command_encoder!(self);

    img_pass!(self).draw_triangles(
      texture,
      indices,
      clear,
      &self.device,
      encoder,
      self.textures_bind.as_ref().unwrap(),
      &self.mask_layers_uniform,
      slot0_bind,
      self.slot1_pool.bind_group(),
      mask_offset,
      self.slot0_pool.bind_offset(prims_offset),
    );
  }

  fn draw_filter_triangles(
    &mut self, texture: &mut Self::Texture, origin: &Self::Texture, indices: Range<u32>,
    clear: Option<Color>, mask_offset: u32, prims_offset: u32,
  ) {
    let slot0_bind = self.slot0_pool.bind_group();
    let encoder = command_encoder!(self);

    filter_pass!(self).draw_triangles(
      texture,
      origin,
      indices,
      clear,
      &self.device,
      encoder,
      self.textures_bind.as_ref().unwrap(),
      &self.mask_layers_uniform,
      slot0_bind,
      mask_offset,
      self.slot0_pool.bind_offset(prims_offset),
    );
  }

  fn flush_draw_commands(&mut self) { self.submit(); }

  fn copy_texture_from_texture(
    &mut self, dest_tex: &mut Self::Texture, dist_pos: DevicePoint, from_tex: &Self::Texture,
    from_rect: &DeviceRect,
  ) {
    if dest_tex.format() == from_tex.format() {
      self.copy_same_format_texture(
        dest_tex.inner_tex.texture(),
        dist_pos,
        from_tex.inner_tex.texture(),
        from_rect,
      );
    } else {
      self.copy_diff_format_texture(dest_tex, dist_pos, from_tex, from_rect);
    }
  }

  fn load_texture_data(
    &mut self, primitives: &[TexturePrimitive], buffers: &VertexBuffers<TexturePrimIndex>,
  ) -> u32 {
    let load = |b: &mut Self| {
      let offset = b.try_load_slot0_texture_primitives(primitives)?;
      b.try_load_texture_vertices(buffers)?;
      Some(offset)
    };
    if let Some(offset) = load(self) {
      return offset;
    }
    self.atomic_flush();
    load(self).unwrap()
  }

  fn draw_texture_triangles(
    &mut self, texture: &mut Self::Texture, indices: Range<u32>, clear: Option<Color>,
    from_texture: &Self::Texture, mask_offset: u32, prims_offset: u32,
  ) {
    let slot0_bind = self.slot0_pool.bind_group();
    let encoder = command_encoder!(self);

    draw_texture_pass!(self).draw_triangles(
      texture,
      indices,
      clear,
      &self.device,
      encoder,
      self.textures_bind.as_ref().unwrap(),
      &self.mask_layers_uniform,
      slot0_bind,
      mask_offset,
      self.slot0_pool.bind_offset(prims_offset),
      from_texture,
    );
  }

  fn end_frame(&mut self) {
    self.submit();
    let _ = self
      .device
      .poll(wgpu::PollType::Wait { timeout: None, submission_index: None });
    #[cfg(debug_assertions)]
    self.stop_capture();
  }
}

impl<'a> Surface<'a> {
  /// Resize the surface to the given size.
  pub fn resize(&mut self, size: DeviceSize, backend: &WgpuImpl) {
    if !size.is_empty() && size != self.size() {
      self.config.width = size.width as u32;
      self.config.height = size.height as u32;
      self
        .surface
        .configure(backend.device(), &self.config);
    }
  }

  /// Get the size of the surface.
  pub fn size(&self) -> DeviceSize {
    DeviceSize::new(self.config.width as i32, self.config.height as i32)
  }

  pub fn get_current_texture(&mut self) -> &mut WgpuTexture {
    self.current_texture.get_or_insert_with(|| {
      let tex = self.surface.get_current_texture().unwrap();
      WgpuTexture::new(InnerTexture::SurfaceTexture(tex))
    })
  }

  /// Present the current texture to the surface.
  pub fn present(&mut self) {
    if let Some(tex) = self.current_texture.take() {
      let InnerTexture::SurfaceTexture(tex) = tex.inner_tex else { unreachable!() };
      tex.present()
    }
  }
}
pub struct WgpuTexture {
  inner_tex: InnerTexture,
  view: wgpu::TextureView,
}

enum InnerTexture {
  Texture(wgpu::Texture),
  SurfaceTexture(wgpu::SurfaceTexture),
}

impl InnerTexture {
  fn texture(&self) -> &wgpu::Texture {
    match self {
      InnerTexture::Texture(texture)
      | InnerTexture::SurfaceTexture(wgpu::SurfaceTexture { texture, .. }) => texture,
    }
  }
}

impl WgpuTexture {
  fn from_tex(tex: wgpu::Texture) -> Self { Self::new(InnerTexture::Texture(tex)) }

  pub(crate) fn color_attachments(
    &self, clear: Option<Color>,
  ) -> wgpu::RenderPassColorAttachment<'_> {
    let load = match clear {
      Some(c) => {
        let [r, g, b, a] = c.into_f32_components();
        wgpu::LoadOp::Clear(wgpu::Color { r: r as f64, g: g as f64, b: b as f64, a: a as f64 })
      }
      None => wgpu::LoadOp::Load,
    };

    let view = self.view();
    let ops = wgpu::Operations { load, store: wgpu::StoreOp::Store };
    wgpu::RenderPassColorAttachment { view, resolve_target: None, ops, depth_slice: None }
  }

  fn new(inner_tex: InnerTexture) -> Self {
    let view = inner_tex.texture().create_view(&<_>::default());
    Self { inner_tex, view }
  }

  pub fn width(&self) -> u32 { self.inner_tex.texture().width() }

  pub fn height(&self) -> u32 { self.inner_tex.texture().height() }

  fn size(&self) -> DeviceSize {
    let size = self.inner_tex.texture().size();
    DeviceSize::new(size.width as i32, size.height as i32)
  }

  fn format(&self) -> wgpu::TextureFormat { self.inner_tex.texture().format() }

  fn view(&self) -> &wgpu::TextureView { &self.view }

  fn usage(&self) -> wgpu::TextureUsages { self.inner_tex.texture().usage() }
}

impl Texture for WgpuTexture {
  type Host = WgpuImpl;

  fn write_data(&mut self, dist: &DeviceRect, data: &[u8], backend: &mut Self::Host) {
    let size = wgpu::Extent3d {
      width: dist.width() as u32,
      height: dist.height() as u32,
      depth_or_array_layers: 1,
    };
    let origin = wgpu::Origin3d { x: dist.min_x() as u32, y: dist.min_y() as u32, z: 0 };
    let bytes_per_pixel = self.color_format().bytes_per_pixel();

    let data_encoded;
    let data = if self.format() == wgpu::TextureFormat::Bgra8Unorm {
      let len = data.len();
      let mut new_data = vec![0; len];
      copy_row_data(data, &mut new_data, self.format());
      data_encoded = Some(new_data);
      data_encoded.as_ref().unwrap()
    } else {
      data
    };

    backend.queue.write_texture(
      wgpu::TexelCopyTextureInfo {
        texture: self.inner_tex.texture(),
        mip_level: 0,
        origin,
        aspect: wgpu::TextureAspect::All,
      },
      data,
      // The layout of the texture
      wgpu::TexelCopyBufferLayout {
        offset: 0,
        bytes_per_row: Some(bytes_per_pixel as u32 * size.width),
        rows_per_image: Some(size.height),
      },
      size,
    );
  }

  fn copy_as_image(
    &self, rect: &DeviceRect, backend: &mut Self::Host,
  ) -> impl std::future::Future<Output = Result<PixelImage, Box<dyn Error>>> + 'static {
    let width = rect.width();
    let height = rect.height();
    let format = self.color_format();
    let pixel_bytes = format.bytes_per_pixel();
    let align_width = align(width as u32, wgpu::COPY_BYTES_PER_ROW_ALIGNMENT / pixel_bytes as u32);
    let padded_row_bytes = pixel_bytes as u32 * align_width;

    // The output buffer lets us retrieve the data as an array
    let buffer = backend
      .device
      .create_buffer(&wgpu::BufferDescriptor {
        size: padded_row_bytes as u64 * height as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
        label: None,
      });

    let origin = wgpu::Origin3d { x: rect.min_x() as u32, y: rect.min_y() as u32, z: 0 };

    let encoder = command_encoder!(backend);

    encoder.copy_texture_to_buffer(
      wgpu::TexelCopyTextureInfo {
        texture: self.inner_tex.texture(),
        mip_level: 0,
        origin,
        aspect: wgpu::TextureAspect::All,
      },
      wgpu::TexelCopyBufferInfo {
        buffer: &buffer,
        layout: wgpu::TexelCopyBufferLayout {
          offset: 0,
          bytes_per_row: Some(padded_row_bytes),
          rows_per_image: Some(height as u32),
        },
      },
      wgpu::Extent3d { width: width as u32, height: height as u32, depth_or_array_layers: 1 },
    );

    backend.submit();

    let (sender, receiver) = oneshot::channel();
    let slice = buffer.slice(..);
    slice.map_async(wgpu::MapMode::Read, move |v| {
      let _ = sender.send(v);
    });

    backend
      .device
      .poll(wgpu::PollType::Wait { submission_index: None, timeout: None })
      .unwrap();

    // Capture texture format for color conversion logic
    let texture_format = self.format();

    async move {
      let _ = receiver.await?;

      let row_bytes = width as usize * pixel_bytes as usize;
      let mut data = vec![0; row_bytes * height as usize];

      let slice = buffer.slice(..).get_mapped_range();
      (0..height as usize).for_each(|r| {
        let padded_start = r * padded_row_bytes as usize;
        let row_start = r * row_bytes;
        let src = &slice[padded_start..padded_start + row_bytes];
        let dst = &mut data[row_start..row_start + row_bytes];
        copy_row_data(src, dst, texture_format);
      });

      Ok(PixelImage::new(data.into(), width as u32, height as u32, format))
    }
  }

  fn color_format(&self) -> ColorFormat {
    try_into_color_format(self.format()).expect("not a valid texture as image")
  }

  fn size(&self) -> DeviceSize { self.size() }

  fn clear_areas(&mut self, areas: &[DeviceRect], backend: &mut Self::Host) {
    backend.clear_tex_areas(areas, self);
  }
}

fn copy_row_data(src: &[u8], dst: &mut [u8], format: wgpu::TextureFormat) {
  if format == wgpu::TextureFormat::Bgra8Unorm {
    // Attempt to use u32 for faster copy and swap
    let (pre_s, mid_s, _) = unsafe { src.align_to::<u32>() };
    let (pre_d, mid_d, _) = unsafe { dst.align_to_mut::<u32>() };

    if pre_s.is_empty() && pre_d.is_empty() {
      for (s, d) in mid_s.iter().zip(mid_d.iter_mut()) {
        #[cfg(target_endian = "little")]
        {
          // 0xAARRGGBB -> 0xAABBGGRR
          *d = (*s & 0xFF00FF00) | ((*s & 0x00FF0000) >> 16) | ((*s & 0x000000FF) << 16);
        }
        #[cfg(target_endian = "big")]
        {
          // 0xBBGGRRAA -> 0xRRGGBBAA
          *d = (*s & 0x00FF00FF) | ((*s & 0xFF000000) >> 16) | ((*s & 0x0000FF00) << 16);
        }
      }
    } else {
      for (s, d) in src.chunks_exact(4).zip(dst.chunks_exact_mut(4)) {
        d[0] = s[2];
        d[1] = s[1];
        d[2] = s[0];
        d[3] = s[3];
      }
    }
  } else {
    dst.copy_from_slice(src);
  }
}

impl WgpuImpl {
  /// Create a new instance of `WgpuImpl` with a headless surface.
  pub async fn headless() -> Self { Self::create(None).await.0 }

  /// Create a new instance of `WgpuImpl` with a surface target and also return
  /// the surface.
  pub async fn new<'a>(target: impl Into<wgpu::SurfaceTarget<'a>>) -> (Self, Surface<'a>) {
    let (gpu_impl, surface) = Self::create(Some(target.into())).await;
    (gpu_impl, surface.unwrap())
  }

  #[allow(clippy::needless_lifetimes)]
  async fn create<'a>(target: Option<wgpu::SurfaceTarget<'a>>) -> (WgpuImpl, Option<Surface<'a>>) {
    let mut instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
      backends: wgpu::Backends::PRIMARY,
      ..<_>::default()
    });

    // This detection mechanism might be deprecated in the future. Ideally, we
    // should be able to create instances with `wgpu::Backends::all()`. However,
    // this currently may not correctly on browsers when WebGPU is insufficient.
    // See https://github.com/gfx-rs/wgpu/issues/5332 for more details.
    if instance
      .request_adapter(&<_>::default())
      .await
      .is_err()
    {
      instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::SECONDARY,
        ..<_>::default()
      });
    }

    let surface = target.map(|t| instance.create_surface(t).unwrap());
    let adapter = instance
      .request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::default(),
        compatible_surface: surface.as_ref(),
        force_fallback_adapter: false,
      })
      .await
      .expect("No suitable GPU adapters found on the system!");

    let (device, queue) = adapter
      .request_device(&wgpu::DeviceDescriptor {
        required_limits: adapter.limits(),
        required_features: wgpu::Features::CLEAR_TEXTURE,
        memory_hints: wgpu::MemoryHints::MemoryUsage,
        experimental_features: wgpu::ExperimentalFeatures::disabled(),
        ..Default::default()
      })
      .await
      .expect("Unable to find a suitable GPU adapter!");

    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
      address_mode_u: wgpu::AddressMode::ClampToEdge,
      address_mode_v: wgpu::AddressMode::ClampToEdge,
      address_mode_w: wgpu::AddressMode::ClampToEdge,
      mag_filter: wgpu::FilterMode::Linear,
      min_filter: wgpu::FilterMode::Linear,
      mipmap_filter: wgpu::MipmapFilterMode::Linear,
      lod_min_clamp: 0.0,
      lod_max_clamp: 0.0,
      label: Some("texture sampler"),
      ..Default::default()
    });

    let alpha_triangles_pass = DrawAlphaTrianglesPass::new(&device);

    let device_limits = device.limits();
    let uniform_bytes = device_limits
      .max_uniform_buffer_binding_size
      .min(1024 * 1024) as usize;
    let texture_size_limit = DeviceSize::new(
      device_limits.max_texture_dimension_2d as i32,
      device_limits.max_texture_dimension_2d as i32,
    );

    let max_filter_matrix_len = (((uniform_bytes - size_of::<FilterPrimitive>()) & 0xFFFFFF00)
      / size_of::<f32>()) // the matrix size must be aligned to 16 bytes
    .min(128 * 128);

    let limits = DrawPhaseLimits {
      max_tex_load: TEX_PER_DRAW,
      texture_size: texture_size_limit,
      max_image_primitives: uniform_bytes / size_of::<ImgPrimitive>(),
      max_radial_gradient_primitives: uniform_bytes / size_of::<RadialGradientPrimitive>(),
      max_linear_gradient_primitives: uniform_bytes / size_of::<LinearGradientPrimitive>(),
      max_gradient_stop_primitives: uniform_bytes / size_of::<GradientStopPrimitive>(),
      max_mask_layers: uniform_bytes / size_of::<MaskLayer>(),
      max_filter_matrix_len,
      max_texture_primitives: uniform_bytes / size_of::<TexturePrimitive>(),
    };

    let mask_layers_uniform =
      Uniform::new(&device, wgpu::ShaderStages::FRAGMENT, limits.max_mask_layers);
    let can_prepare_storage_pool =
      device_limits.max_storage_buffer_binding_size >= uniform_bytes as u32;
    debug!("primitive pool mode: can_use_storage={can_prepare_storage_pool}");
    let (slot0_pool, slot1_pool) = if can_prepare_storage_pool {
      (
        PrimitivePool::new_storage(
          &device,
          wgpu::ShaderStages::FRAGMENT,
          uniform_bytes,
          uniform_bytes * 16,
        ),
        PrimitivePool::new_storage(
          &device,
          wgpu::ShaderStages::FRAGMENT,
          uniform_bytes,
          uniform_bytes * 16,
        ),
      )
    } else {
      (
        PrimitivePool::new_uniform(
          &device,
          wgpu::ShaderStages::FRAGMENT,
          uniform_bytes,
          uniform_bytes * 16,
        ),
        PrimitivePool::new_uniform(
          &device,
          wgpu::ShaderStages::FRAGMENT,
          uniform_bytes,
          uniform_bytes * 16,
        ),
      )
    };
    let dual_slot_mode = slot0_pool.mode();
    let slot0_only_mode = slot0_pool.mode();
    let clear_tex_pass = ClearTexturePass::new(&device);
    let texs_layout = textures_layout(&device);

    let surface_caps = surface
      .as_ref()
      .map(|surface| surface.get_capabilities(&adapter));
    let surface_format = surface_caps.as_ref().map(|caps| {
      use wgpu::TextureFormat::*;
      let formats = HashSet::from_iter(caps.formats.clone().into_iter());
      *formats
        .get(&Rgba8Unorm)
        .or_else(|| formats.get(&Bgra8Unorm))
        .expect("No suitable format found for the surface!")
    });

    let gpu_impl = WgpuImpl {
      device,
      queue,
      command_encoder: None,
      command_buffers: vec![],
      sampler,
      alpha_triangles_pass,
      clear_tex_pass,
      copy_tex_pass: None,
      color_triangles_pass: None,
      img_triangles_pass: None,
      radial_gradient_pass: None,
      linear_gradient_pass: None,
      filter_pass: None,
      draw_texture_pass: None,
      texs_layout,
      textures_bind: None,
      mask_layers_uniform,
      slot0_pool,
      slot1_pool,
      dual_slot_mode,
      slot0_only_mode,
      limits,
      surface_format,
    };

    debug!(
      "primitive pool mode: dual_slot={:?}, slot0_only={:?}",
      gpu_impl.dual_slot_mode, gpu_impl.slot0_only_mode
    );

    let surface = surface
      .zip(surface_caps)
      .zip(surface_format)
      .map(|((surface, caps), format)| {
        let mut usage = wgpu::TextureUsages::RENDER_ATTACHMENT
          | wgpu::TextureUsages::COPY_SRC
          | wgpu::TextureUsages::COPY_DST;
        usage &= caps.usages;

        let config = wgpu::SurfaceConfiguration {
          usage,
          format,
          width: 0,
          height: 0,
          present_mode: wgpu::PresentMode::Fifo,
          alpha_mode: wgpu::CompositeAlphaMode::Auto,
          view_formats: vec![format],
          desired_maximum_frame_latency: 2,
        };

        Surface { surface, config, current_texture: None }
      });

    (gpu_impl, surface)
  }

  pub fn start_capture(&self) {
    unsafe {
      self.device.start_graphics_debugger_capture();
    }
  }

  pub fn stop_capture(&self) {
    unsafe {
      self.device.stop_graphics_debugger_capture();
    }
  }

  pub fn device(&self) -> &wgpu::Device { &self.device }

  fn submit(&mut self) {
    self.finish_command();
    if !self.command_buffers.is_empty() {
      self.queue.submit(self.command_buffers.drain(..));
    }
  }

  fn copy_same_format_texture(
    &mut self, dist_tex: &wgpu::Texture, copy_to: DevicePoint, from_tex: &wgpu::Texture,
    from_rect: &DeviceRect,
  ) {
    assert_eq!(dist_tex.format(), from_tex.format());

    let encoder = command_encoder!(self);
    let src_origin =
      wgpu::Origin3d { x: from_rect.min_x() as u32, y: from_rect.min_y() as u32, z: 0 };
    let dst_origin = wgpu::Origin3d { x: copy_to.x as u32, y: copy_to.y as u32, z: 0 };
    let copy_size = wgpu::Extent3d {
      width: from_rect.width() as u32,
      height: from_rect.height() as u32,
      depth_or_array_layers: 1,
    };
    encoder.copy_texture_to_texture(
      wgpu::TexelCopyTextureInfo {
        texture: from_tex,
        mip_level: 0,
        origin: src_origin,
        aspect: wgpu::TextureAspect::All,
      },
      wgpu::TexelCopyTextureInfo {
        texture: dist_tex,
        mip_level: 0,
        origin: dst_origin,
        aspect: wgpu::TextureAspect::All,
      },
      copy_size,
    );
  }

  pub(crate) fn finish_command(&mut self) {
    if let Some(encoder) = self.command_encoder.take() {
      self.command_buffers.push(encoder.finish());
    }
  }
}

fn into_wgpu_format(format: ColorFormat) -> wgpu::TextureFormat {
  match format {
    ColorFormat::Rgba8 => wgpu::TextureFormat::Rgba8Unorm,
    ColorFormat::Alpha8 => wgpu::TextureFormat::R8Unorm,
  }
}

fn try_into_color_format(format: wgpu::TextureFormat) -> Option<ColorFormat> {
  match format {
    wgpu::TextureFormat::R8Unorm => Some(ColorFormat::Alpha8),
    wgpu::TextureFormat::Rgba8Unorm | wgpu::TextureFormat::Bgra8Unorm => Some(ColorFormat::Rgba8),
    _ => None,
  }
}

fn align(width: u32, align: u32) -> u32 {
  match width % align {
    0 => width,
    other => width - other + align,
  }
}

fn textures_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
  let mut entries: [MaybeUninit<wgpu::BindGroupLayoutEntry>; 1 + TEX_PER_DRAW] =
    unsafe { MaybeUninit::uninit().assume_init() };
  entries[0].write(wgpu::BindGroupLayoutEntry {
    binding: 0,
    visibility: wgpu::ShaderStages::FRAGMENT,
    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
    count: None,
  });

  for (i, entry) in entries.iter_mut().enumerate().skip(1) {
    entry.write(wgpu::BindGroupLayoutEntry {
      binding: i as u32,
      visibility: wgpu::ShaderStages::FRAGMENT,
      ty: wgpu::BindingType::Texture {
        sample_type: wgpu::TextureSampleType::Float { filterable: true },
        view_dimension: wgpu::TextureViewDimension::D2,
        multisampled: false,
      },
      count: None,
    });
  }
  let entries: [wgpu::BindGroupLayoutEntry; 1 + TEX_PER_DRAW] =
    unsafe { std::mem::transmute(entries) };

  device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
    entries: &entries,
    label: Some("Textures layout"),
  })
}

fn textures_bind(
  device: &wgpu::Device, sampler: &wgpu::Sampler, layout: &wgpu::BindGroupLayout,
  textures: &[&WgpuTexture],
) -> wgpu::BindGroup {
  assert!(!textures.is_empty());
  assert!(textures.len() <= TEX_PER_DRAW);

  let mut entries: [MaybeUninit<wgpu::BindGroupEntry>; 1 + TEX_PER_DRAW] =
    unsafe { MaybeUninit::uninit().assume_init() };
  entries[0]
    .write(wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::Sampler(sampler) });
  for (i, entry) in entries.iter_mut().enumerate().skip(1) {
    // if the texture is not enough, use the first texture to fill the gap
    let view = textures.get(i - 1).unwrap_or(&textures[0]).view();
    entry.write(wgpu::BindGroupEntry {
      binding: i as u32,
      resource: wgpu::BindingResource::TextureView(view),
    });
  }
  let entries: [wgpu::BindGroupEntry; 1 + TEX_PER_DRAW] = unsafe { std::mem::transmute(entries) };

  device.create_bind_group(&wgpu::BindGroupDescriptor {
    layout,
    entries: &entries,
    label: Some("textures bind group"),
  })
}
