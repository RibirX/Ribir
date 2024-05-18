use std::{
  error::Error,
  mem::{size_of, MaybeUninit},
  ops::Range,
};

use futures::channel::oneshot;
use ribir_geom::{DevicePoint, DeviceRect, DeviceSize};
use ribir_painter::{image::ColorFormat, Color, PixelImage, VertexBuffers};

use self::{
  draw_alpha_triangles_pass::DrawAlphaTrianglesPass,
  draw_color_triangles_pass::DrawColorTrianglesPass,
  draw_img_triangles_pass::DrawImgTrianglesPass,
  draw_linear_gradient_pass::DrawLinearGradientTrianglesPass,
  draw_radial_gradient_pass::DrawRadialGradientTrianglesPass,
  texture_pass::{ClearTexturePass, CopyTexturePass},
  uniform::Uniform,
};
use crate::{
  gpu_backend::Texture, ColorAttr, DrawPhaseLimits, GPUBackendImpl, GradientStopPrimitive,
  ImagePrimIndex, ImgPrimitive, LinearGradientPrimIndex, LinearGradientPrimitive, MaskLayer,
  RadialGradientPrimIndex, RadialGradientPrimitive,
};
mod shaders;
mod uniform;
mod vertex_buffer;

mod draw_alpha_triangles_pass;
mod draw_color_triangles_pass;
mod draw_img_triangles_pass;
mod draw_linear_gradient_pass;
mod draw_radial_gradient_pass;
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
  texs_layout: wgpu::BindGroupLayout,
  textures_bind: Option<wgpu::BindGroup>,
  mask_layers_uniform: Uniform<MaskLayer>,
  limits: DrawPhaseLimits,
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
  ($backend:ident) => {
    $backend
      .color_triangles_pass
      .get_or_insert_with(|| {
        DrawColorTrianglesPass::new(
          &$backend.device,
          $backend.mask_layers_uniform.layout(),
          &$backend.texs_layout,
          $backend.limits.max_mask_layers,
        )
      })
  };
}

macro_rules! img_pass {
  ($backend:ident) => {
    $backend
      .img_triangles_pass
      .get_or_insert_with(|| {
        DrawImgTrianglesPass::new(
          &$backend.device,
          $backend.mask_layers_uniform.layout(),
          &$backend.texs_layout,
          &$backend.limits,
        )
      })
  };
}

macro_rules! radial_gradient_pass {
  ($backend:ident) => {
    $backend
      .radial_gradient_pass
      .get_or_insert_with(|| {
        DrawRadialGradientTrianglesPass::new(
          &$backend.device,
          $backend.mask_layers_uniform.layout(),
          &$backend.texs_layout,
          &$backend.limits,
        )
      })
  };
}

macro_rules! linear_gradient_pass {
  ($backend:ident) => {
    $backend
      .linear_gradient_pass
      .get_or_insert_with(|| {
        DrawLinearGradientTrianglesPass::new(
          &$backend.device,
          $backend.mask_layers_uniform.layout(),
          &$backend.texs_layout,
          &$backend.limits,
        )
      })
  };
}

pub(crate) use command_encoder;

pub struct Surface<'a> {
  surface: wgpu::Surface<'a>,
  config: wgpu::SurfaceConfiguration,
  current_texture: Option<WgpuTexture>,
}

impl GPUBackendImpl for WgpuImpl {
  type Texture = WgpuTexture;

  fn limits(&self) -> &DrawPhaseLimits { &self.limits }

  fn begin_frame(&mut self) {
    if self.command_encoder.is_none() {
      #[cfg(debug_assertions)]
      self.start_capture();
    }
  }

  fn new_texture(&mut self, size: DeviceSize, format: ColorFormat) -> Self::Texture {
    let format = into_wgpu_format(format);
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

  fn load_textures(&mut self, textures: &[&Self::Texture]) {
    self.textures_bind =
      Some(textures_bind(&self.device, &self.sampler, &self.texs_layout, textures));
  }

  fn load_alpha_vertices(&mut self, buffers: &VertexBuffers<()>) {
    self
      .alpha_triangles_pass
      .load_alpha_vertices(buffers, &self.device, &self.queue);
  }

  fn load_color_vertices(&mut self, buffers: &VertexBuffers<ColorAttr>) {
    color_pass!(self).load_triangles_vertices(buffers, &self.device, &self.queue);
  }

  fn load_img_vertices(&mut self, buffers: &VertexBuffers<ImagePrimIndex>) {
    img_pass!(self).load_triangles_vertices(buffers, &self.device, &self.queue);
  }

  fn load_img_primitives(&mut self, primitives: &[ImgPrimitive]) {
    img_pass!(self).load_img_primitives(&self.queue, primitives);
  }

  fn load_radial_gradient_primitives(&mut self, primitives: &[RadialGradientPrimitive]) {
    radial_gradient_pass!(self).load_radial_gradient_primitives(&self.queue, primitives);
  }

  fn load_radial_gradient_stops(&mut self, stops: &[GradientStopPrimitive]) {
    radial_gradient_pass!(self).load_gradient_stops(&self.queue, stops);
  }

  fn load_radial_gradient_vertices(&mut self, buffers: &VertexBuffers<RadialGradientPrimIndex>) {
    radial_gradient_pass!(self).load_triangles_vertices(buffers, &self.device, &self.queue);
  }

  fn load_linear_gradient_primitives(&mut self, primitives: &[LinearGradientPrimitive]) {
    linear_gradient_pass!(self).load_linear_gradient_primitives(&self.queue, primitives);
  }

  fn load_linear_gradient_stops(&mut self, stops: &[GradientStopPrimitive]) {
    linear_gradient_pass!(self).load_gradient_stops(&self.queue, stops);
  }

  fn load_linear_gradient_vertices(&mut self, buffers: &VertexBuffers<LinearGradientPrimIndex>) {
    linear_gradient_pass!(self).load_triangles_vertices(buffers, &self.device, &self.queue);
  }

  fn load_mask_layers(&mut self, layers: &[crate::MaskLayer]) {
    self
      .mask_layers_uniform
      .write_buffer(&self.queue, layers);
  }

  fn draw_alpha_triangles(&mut self, indices: &Range<u32>, texture: &mut Self::Texture) {
    let encoder = command_encoder!(self);
    self
      .alpha_triangles_pass
      .draw_alpha_triangles(indices, texture, None, &self.queue, encoder);
  }

  fn draw_radial_gradient_triangles(
    &mut self, texture: &mut Self::Texture, indices: Range<u32>, clear: Option<Color>,
  ) {
    let encoder = command_encoder!(self);

    radial_gradient_pass!(self).draw_triangles(
      texture,
      indices,
      clear,
      &self.device,
      encoder,
      self.textures_bind.as_ref().unwrap(),
      &self.mask_layers_uniform,
    );

    self.submit()
  }

  fn draw_linear_gradient_triangles(
    &mut self, texture: &mut Self::Texture, indices: Range<u32>, clear: Option<Color>,
  ) {
    let encoder = command_encoder!(self);

    linear_gradient_pass!(self).draw_triangles(
      texture,
      indices,
      clear,
      &self.device,
      encoder,
      self.textures_bind.as_ref().unwrap(),
      &self.mask_layers_uniform,
    );

    self.submit()
  }

  fn draw_alpha_triangles_with_scissor(
    &mut self, indices: &Range<u32>, texture: &mut Self::Texture, scissor: DeviceRect,
  ) {
    let encoder = command_encoder!(self);
    self.alpha_triangles_pass.draw_alpha_triangles(
      indices,
      texture,
      Some(scissor),
      &self.queue,
      encoder,
    );
  }

  fn draw_color_triangles(
    &mut self, texture: &mut Self::Texture, indices: Range<u32>, clear: Option<Color>,
  ) {
    let encoder = command_encoder!(self);
    color_pass!(self).draw_triangles(
      texture,
      indices,
      clear,
      &self.device,
      encoder,
      self.textures_bind.as_ref().unwrap(),
      &self.mask_layers_uniform,
    );
    self.submit()
  }

  fn draw_img_triangles(
    &mut self, texture: &mut Self::Texture, indices: Range<u32>, clear: Option<Color>,
  ) {
    let encoder = command_encoder!(self);
    img_pass!(self).draw_triangles(
      texture,
      indices,
      clear,
      &self.device,
      encoder,
      self.textures_bind.as_ref().unwrap(),
      &self.mask_layers_uniform,
    );
    self.submit()
  }

  fn copy_texture_from_texture(
    &mut self, dist_tex: &mut Self::Texture, dist_pos: DevicePoint, from_tex: &Self::Texture,
    from_rect: &DeviceRect,
  ) {
    if dist_tex.format() == from_tex.format() {
      self.copy_same_format_texture(
        dist_tex.inner_tex.texture(),
        dist_pos,
        from_tex.inner_tex.texture(),
        from_rect,
      );
    } else {
      self.draw_texture_to_texture(dist_tex, dist_pos, from_tex, from_rect)
    }
  }

  fn end_frame(&mut self) {
    self.submit();
    self.device.poll(wgpu::Maintain::Wait);
    #[cfg(debug_assertions)]
    self.stop_capture();
  }
}

impl<'a> Surface<'a> {
  /// Resize the surface to the given size.
  pub fn resize(&mut self, size: DeviceSize, backend: &WgpuImpl) {
    self.config.width = size.width as u32;
    self.config.height = size.height as u32;
    if !size.is_empty() {
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

  pub(crate) fn color_attachments(&self, clear: Option<Color>) -> wgpu::RenderPassColorAttachment {
    let load = match clear {
      Some(c) => {
        let [r, g, b, a] = c.into_f32_components();
        wgpu::LoadOp::Clear(wgpu::Color { r: r as f64, g: g as f64, b: b as f64, a: a as f64 })
      }
      None => wgpu::LoadOp::Load,
    };

    let view = self.view();
    let ops = wgpu::Operations { load, store: wgpu::StoreOp::Store };
    wgpu::RenderPassColorAttachment { view, resolve_target: None, ops }
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
    let bytes_per_pixel = self.color_format().pixel_per_bytes();

    backend.queue.write_texture(
      wgpu::ImageCopyTexture {
        texture: self.inner_tex.texture(),
        mip_level: 0,
        origin,
        aspect: wgpu::TextureAspect::All,
      },
      data,
      // The layout of the texture
      wgpu::ImageDataLayout {
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
    let pixel_bytes = format.pixel_per_bytes();
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
      wgpu::ImageCopyTexture {
        texture: self.inner_tex.texture(),
        mip_level: 0,
        origin,
        aspect: wgpu::TextureAspect::All,
      },
      wgpu::ImageCopyBuffer {
        buffer: &buffer,
        layout: wgpu::ImageDataLayout {
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
    slice.map_async(wgpu::MapMode::Read, move |v| sender.send(v).unwrap());

    async move {
      let _ = receiver.await?;

      let row_bytes = width as usize * pixel_bytes as usize;
      let mut data = vec![0; row_bytes * height as usize];

      let slice = buffer.slice(..).get_mapped_range();
      (0..height as usize).for_each(|r| {
        let padded_start = r * padded_row_bytes as usize;
        let row_start = r * row_bytes;
        data[row_start..row_start + row_bytes]
          .copy_from_slice(&slice[padded_start..padded_start + row_bytes]);
      });

      Ok(PixelImage::new(data.into(), width as u32, height as u32, format))
    }
  }

  fn color_format(&self) -> ColorFormat {
    match self.format() {
      wgpu::TextureFormat::R8Unorm => ColorFormat::Alpha8,
      wgpu::TextureFormat::Rgba8Unorm => ColorFormat::Rgba8,
      _ => panic!("not a valid texture as image"),
    }
  }

  fn size(&self) -> DeviceSize { self.size() }

  fn clear_areas(&mut self, areas: &[DeviceRect], backend: &mut Self::Host) {
    backend.clear_tex_areas(areas, self);
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
    let mut instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
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
      .is_none()
    {
      instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
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
      .request_device(
        &wgpu::DeviceDescriptor {
          required_limits: adapter.limits(),
          required_features: wgpu::Features::CLEAR_TEXTURE,
          ..Default::default()
        },
        None,
      )
      .await
      .expect("Unable to find a suitable GPU adapter!");

    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
      address_mode_u: wgpu::AddressMode::ClampToEdge,
      address_mode_v: wgpu::AddressMode::ClampToEdge,
      address_mode_w: wgpu::AddressMode::ClampToEdge,
      mag_filter: wgpu::FilterMode::Linear,
      min_filter: wgpu::FilterMode::Linear,
      mipmap_filter: wgpu::FilterMode::Linear,
      lod_min_clamp: 0.0,
      lod_max_clamp: 0.0,
      label: Some("texture sampler"),
      ..Default::default()
    });

    let alpha_triangles_pass = DrawAlphaTrianglesPass::new(&device);

    let limits = device.limits();
    let uniform_bytes = limits
      .max_uniform_buffer_binding_size
      .min(1024 * 1024) as usize;
    let texture_size_limit = DeviceSize::new(
      limits.max_texture_dimension_2d as i32,
      limits.max_texture_dimension_2d as i32,
    );

    let limits = DrawPhaseLimits {
      max_tex_load: TEX_PER_DRAW,
      texture_size: texture_size_limit,
      max_image_primitives: uniform_bytes / size_of::<ImgPrimitive>(),
      max_radial_gradient_primitives: uniform_bytes / size_of::<RadialGradientPrimitive>(),
      max_linear_gradient_primitives: uniform_bytes / size_of::<LinearGradientPrimitive>(),
      max_gradient_stop_primitives: uniform_bytes / size_of::<GradientStopPrimitive>(),
      max_mask_layers: uniform_bytes / size_of::<MaskLayer>(),
    };

    let mask_layers_uniform =
      Uniform::new(&device, wgpu::ShaderStages::FRAGMENT, limits.max_mask_layers);
    let clear_tex_pass = ClearTexturePass::new(&device);
    let texs_layout = textures_layout(&device);
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
      texs_layout,
      textures_bind: None,
      mask_layers_uniform,
      limits,
    };

    let surface = surface.map(|surface| {
      use wgpu::TextureFormat::*;
      let format = surface
        .get_capabilities(&adapter)
        .formats
        .into_iter()
        .find(|&f| f == Rgba8Unorm || f == Bgra8Unorm)
        .expect("No suitable format found for the surface!");

      let config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
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

  pub fn start_capture(&self) { self.device.start_capture(); }

  pub fn stop_capture(&self) { self.device.stop_capture(); }

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
      wgpu::ImageCopyTexture {
        texture: from_tex,
        mip_level: 0,
        origin: src_origin,
        aspect: wgpu::TextureAspect::All,
      },
      wgpu::ImageCopyTexture {
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
