use self::{
  draw_alpha_triangles_pass::DrawAlphaTrianglesPass,
  draw_color_triangles_pass::DrawColorTrianglesPass, draw_img_triangles_pass::DrawImgTrianglesPass,
  draw_linear_gradient_pass::DrawLinearGradientTrianglesPass,
  draw_radial_gradient_pass::DrawRadialGradientTrianglesPass, draw_texture_pass::DrawTexturePass,
  storage::Storage,
};
use crate::{
  gpu_backend::Texture, ColorAttr, GPUBackendImpl, GradientStopPrimitive, ImagePrimIndex,
  ImgPrimitive, LinearGradientPrimIndex, LinearGradientPrimitive, MaskLayer,
  RadialGradientPrimIndex, RadialGradientPrimitive,
};
use futures::{channel::oneshot, Future};
use ribir_geom::{DevicePoint, DeviceRect, DeviceSize};
use ribir_painter::{image::ColorFormat, AntiAliasing, Color, PixelImage, VertexBuffers};
use std::{error::Error, num::NonZeroU32, ops::Range, pin::Pin};
mod buffer_pool;
mod storage;
mod vertex_buffer;

mod draw_alpha_triangles_pass;
mod draw_color_triangles_pass;
mod draw_img_triangles_pass;
mod draw_linear_gradient_pass;
mod draw_radial_gradient_pass;
mod draw_texture_pass;

pub struct WgpuImpl {
  device: wgpu::Device,
  queue: wgpu::Queue,

  command_encoder: Option<wgpu::CommandEncoder>,
  command_buffers: Vec<wgpu::CommandBuffer>,

  sampler: wgpu::Sampler,
  draw_tex_pass: DrawTexturePass,
  draw_alpha_triangles_pass: DrawAlphaTrianglesPass,
  draw_color_triangles_pass: DrawColorTrianglesPass,
  draw_img_triangles_pass: DrawImgTrianglesPass,
  draw_radial_gradient_pass: DrawRadialGradientTrianglesPass,
  draw_linear_gradient_pass: DrawLinearGradientTrianglesPass,

  textures_bind: TexturesBind,
  mask_layers_storage: Storage<MaskLayer>,
}

#[derive(Default)]
pub struct TexturesBind {
  texture_cnt: usize,
  textures_bind: Option<wgpu::BindGroup>,
  textures_layout: Option<wgpu::BindGroupLayout>,
}

macro_rules! command_encoder {
  ($backend: ident) => {
    $backend.command_encoder.get_or_insert_with(|| {
      $backend
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Frame Encoder") })
    })
  };
}
pub(crate) use command_encoder;

impl GPUBackendImpl for WgpuImpl {
  type Texture = WgpuTexture;

  fn begin_frame(&mut self) {
    if self.command_encoder.is_none() {
      let encoder = self
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Frame Encoder") });
      self.command_encoder = Some(encoder);
      #[cfg(debug_assertions)]
      self.start_capture();
    }
  }

  fn new_texture(
    &mut self,
    size: DeviceSize,
    anti_aliasing: AntiAliasing,
    format: ColorFormat,
  ) -> Self::Texture {
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
    let mut tex = WgpuTexture::from_tex(tex);
    tex.set_anti_aliasing(anti_aliasing, self);
    tex
  }

  fn load_textures(&mut self, textures: &[&Self::Texture]) {
    self
      .textures_bind
      .load_textures(&self.device, &self.sampler, textures);
  }

  fn load_alpha_vertices(&mut self, buffers: &VertexBuffers<f32>) {
    self
      .draw_alpha_triangles_pass
      .load_alpha_vertices(buffers, &self.device, &self.queue);
  }

  fn load_color_vertices(&mut self, buffers: &VertexBuffers<ColorAttr>) {
    self
      .draw_color_triangles_pass
      .load_triangles_vertices(buffers, &self.device, &self.queue);
  }

  fn load_img_vertices(&mut self, buffers: &VertexBuffers<ImagePrimIndex>) {
    self
      .draw_img_triangles_pass
      .load_triangles_vertices(buffers, &self.device, &self.queue);
  }

  fn load_img_primitives(&mut self, primitives: &[ImgPrimitive]) {
    self
      .draw_img_triangles_pass
      .load_img_primitives(&self.device, &self.queue, primitives);
  }

  fn load_radial_gradient_primitives(&mut self, primitives: &[RadialGradientPrimitive]) {
    self
      .draw_radial_gradient_pass
      .load_radial_gradient_primitives(&self.device, &self.queue, primitives);
  }

  fn load_radial_gradient_stops(&mut self, stops: &[GradientStopPrimitive]) {
    self
      .draw_radial_gradient_pass
      .load_gradient_stops(&self.device, &self.queue, stops);
  }

  fn load_radial_gradient_vertices(&mut self, buffers: &VertexBuffers<RadialGradientPrimIndex>) {
    self
      .draw_radial_gradient_pass
      .load_triangles_vertices(buffers, &self.device, &self.queue);
  }

  fn load_linear_gradient_primitives(&mut self, primitives: &[LinearGradientPrimitive]) {
    self
      .draw_linear_gradient_pass
      .load_linear_gradient_primitives(&self.device, &self.queue, primitives);
  }

  fn load_linear_gradient_stops(&mut self, stops: &[GradientStopPrimitive]) {
    self
      .draw_linear_gradient_pass
      .load_gradient_stops(&self.device, &self.queue, stops);
  }

  fn load_linear_gradient_vertices(&mut self, buffers: &VertexBuffers<LinearGradientPrimIndex>) {
    self
      .draw_linear_gradient_pass
      .load_triangles_vertices(buffers, &self.device, &self.queue);
  }

  fn load_mask_layers(&mut self, layers: &[crate::MaskLayer]) {
    self
      .mask_layers_storage
      .write_buffer(&self.device, &self.queue, layers);
  }

  fn draw_alpha_triangles(&mut self, indices: &Range<u32>, texture: &mut Self::Texture) {
    let encoder = command_encoder!(self);
    self.draw_alpha_triangles_pass.draw_alpha_triangles(
      indices,
      texture,
      None,
      encoder,
      &self.device,
    );
  }

  fn draw_radial_gradient_triangles(
    &mut self,
    texture: &mut Self::Texture,
    indices: Range<u32>,
    clear: Option<Color>,
  ) {
    let encoder = command_encoder!(self);

    self.draw_radial_gradient_pass.draw_triangles(
      texture,
      indices,
      clear,
      &self.device,
      encoder,
      &self.textures_bind,
      &self.mask_layers_storage,
    );
  }

  fn draw_linear_gradient_triangles(
    &mut self,
    texture: &mut Self::Texture,
    indices: Range<u32>,
    clear: Option<Color>,
  ) {
    let encoder = command_encoder!(self);

    self.draw_linear_gradient_pass.draw_triangles(
      texture,
      indices,
      clear,
      &self.device,
      encoder,
      &self.textures_bind,
      &self.mask_layers_storage,
    );
  }

  fn draw_alpha_triangles_with_scissor(
    &mut self,
    indices: &Range<u32>,
    texture: &mut Self::Texture,
    scissor: DeviceRect,
  ) {
    let encoder = command_encoder!(self);
    self.draw_alpha_triangles_pass.draw_alpha_triangles(
      indices,
      texture,
      Some(scissor),
      encoder,
      &self.device,
    );
  }

  fn draw_color_triangles(
    &mut self,
    texture: &mut Self::Texture,
    indices: Range<u32>,
    clear: Option<Color>,
  ) {
    let encoder = command_encoder!(self);
    self.draw_color_triangles_pass.draw_triangles(
      texture,
      indices,
      clear,
      &self.device,
      encoder,
      &self.textures_bind,
      &self.mask_layers_storage,
    );
  }

  fn draw_img_triangles(
    &mut self,
    texture: &mut Self::Texture,
    indices: Range<u32>,
    clear: Option<Color>,
  ) {
    let encoder = command_encoder!(self);
    self.draw_img_triangles_pass.draw_triangles(
      texture,
      indices,
      clear,
      &self.device,
      encoder,
      &self.textures_bind,
      &self.mask_layers_storage,
    );
  }

  fn copy_texture_from_texture(
    &mut self,
    dist_tex: &mut Self::Texture,
    dist_pos: DevicePoint,
    from_tex: &Self::Texture,
    from_rect: &DeviceRect,
  ) {
    if dist_tex.format() == from_tex.format() {
      self.copy_same_format_texture(
        dist_tex.inner_tex.texture(),
        dist_pos,
        from_tex.inner_tex.texture(),
        from_rect,
      );
      if let Some(multi_sampler) = dist_tex.multisampler.as_ref() {
        let from = from_tex
          .multisampler
          .as_ref()
          .expect("multisampler texture must copy from a multismapler texture.");
        self.copy_same_format_texture(multi_sampler, dist_pos, from, from_rect);
      }
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

pub struct WgpuTexture {
  inner_tex: InnerTexture,
  view: wgpu::TextureView,
  anti_aliasing: AntiAliasing,
  multisampler: Option<wgpu::Texture>,
  multisampler_view: Option<wgpu::TextureView>,
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
  pub fn from_tex(tex: wgpu::Texture) -> Self { Self::new(InnerTexture::Texture(tex)) }

  pub fn from_surface_tex(tex: wgpu::SurfaceTexture) -> Self {
    Self::new(InnerTexture::SurfaceTexture(tex))
  }

  pub fn into_texture(self) -> Option<wgpu::Texture> {
    match self.inner_tex {
      InnerTexture::Texture(texture) => Some(texture),
      InnerTexture::SurfaceTexture(_) => None,
    }
  }

  pub fn into_surface_texture(self) -> Option<wgpu::SurfaceTexture> {
    match self.inner_tex {
      InnerTexture::Texture(_) => None,
      InnerTexture::SurfaceTexture(tex) => Some(tex),
    }
  }

  pub(crate) fn color_attachments(&self, clear: Option<Color>) -> wgpu::RenderPassColorAttachment {
    let load = match clear {
      Some(c) => {
        let [r, g, b, a] = c.into_f32_components();
        wgpu::LoadOp::Clear(wgpu::Color {
          r: r as f64,
          g: g as f64,
          b: b as f64,
          a: a as f64,
        })
      }
      None => wgpu::LoadOp::Load,
    };

    let view = self.view();
    let ops = wgpu::Operations { load, store: true };

    if let Some(multi_sample) = &self.multisampler_view {
      wgpu::RenderPassColorAttachment {
        view: multi_sample,
        resolve_target: Some(view),
        ops,
      }
    } else {
      wgpu::RenderPassColorAttachment { view, resolve_target: None, ops }
    }
  }

  fn new(inner_tex: InnerTexture) -> Self {
    let view = inner_tex.texture().create_view(&<_>::default());
    Self {
      inner_tex,
      view,
      anti_aliasing: AntiAliasing::None,
      multisampler: None,
      multisampler_view: None,
    }
  }

  fn width(&self) -> u32 { self.inner_tex.texture().width() }

  fn height(&self) -> u32 { self.inner_tex.texture().height() }

  fn size(&self) -> DeviceSize {
    let size = self.inner_tex.texture().size();
    DeviceSize::new(size.width as i32, size.height as i32)
  }

  fn format(&self) -> wgpu::TextureFormat { self.inner_tex.texture().format() }

  fn view(&self) -> &wgpu::TextureView { &self.view }
}

impl Texture for WgpuTexture {
  type Host = WgpuImpl;

  fn anti_aliasing(&self) -> AntiAliasing { self.anti_aliasing }

  fn set_anti_aliasing(&mut self, anti_aliasing: AntiAliasing, host: &mut Self::Host) {
    if anti_aliasing == self.anti_aliasing {
      return;
    }
    self.anti_aliasing = anti_aliasing;

    if anti_aliasing == AntiAliasing::None {
      self.multisampler.take();
      return;
    }

    if self.multisampler.is_none() {
      let m_desc = &wgpu::TextureDescriptor {
        size: wgpu::Extent3d {
          width: self.width(),
          height: self.height(),
          depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: anti_aliasing as u32,
        dimension: wgpu::TextureDimension::D2,
        format: self.format(),
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
          | wgpu::TextureUsages::COPY_SRC
          | wgpu::TextureUsages::COPY_DST,
        label: None,
        view_formats: &[],
      };
      let m_sampler = host.device.create_texture(m_desc);
      self.anti_aliasing = anti_aliasing;
      self.multisampler_view = Some(m_sampler.create_view(&<_>::default()));
      self.multisampler = Some(m_sampler);
    }
  }

  fn write_data(&mut self, dist: &DeviceRect, data: &[u8], backend: &mut Self::Host) {
    let size = wgpu::Extent3d {
      width: dist.width() as u32,
      height: dist.height() as u32,
      depth_or_array_layers: 1,
    };
    let origin = wgpu::Origin3d {
      x: dist.min_x() as u32,
      y: dist.min_y() as u32,
      z: 0,
    };
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
    &self,
    rect: &DeviceRect,
    backend: &mut Self::Host,
  ) -> Pin<Box<dyn Future<Output = Result<PixelImage, Box<dyn Error>>> + Send + Sync>> {
    let width = rect.width();
    let height = rect.height();
    let format = self.color_format();
    let pixel_bytes = format.pixel_per_bytes();
    let align_width = align(
      width as u32,
      wgpu::COPY_BYTES_PER_ROW_ALIGNMENT / pixel_bytes as u32,
    );
    let padded_row_bytes = pixel_bytes as u32 * align_width;

    // The output buffer lets us retrieve the data as an array
    let buffer = backend.device.create_buffer(&wgpu::BufferDescriptor {
      size: padded_row_bytes as u64 * height as u64,
      usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
      mapped_at_creation: false,
      label: None,
    });

    let origin = wgpu::Origin3d {
      x: rect.min_x() as u32,
      y: rect.min_y() as u32,
      z: 0,
    };

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
      wgpu::Extent3d {
        width: width as u32,
        height: height as u32,
        depth_or_array_layers: 1,
      },
    );

    backend.submit();

    let (sender, receiver) = oneshot::channel();
    let slice = buffer.slice(..);
    slice.map_async(wgpu::MapMode::Read, move |v| sender.send(v).unwrap());

    let res = async move {
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

      Ok(PixelImage::new(
        data.into(),
        width as u32,
        height as u32,
        format,
      ))
    };

    Box::pin(res)
  }

  fn color_format(&self) -> ColorFormat {
    match self.format() {
      wgpu::TextureFormat::R8Unorm => ColorFormat::Alpha8,
      wgpu::TextureFormat::Rgba8Unorm => ColorFormat::Rgba8,
      _ => panic!("not a valid texture as image"),
    }
  }

  fn size(&self) -> DeviceSize { self.size() }
}

impl WgpuImpl {
  pub async fn headless() -> Self {
    let instance = wgpu::Instance::new(<_>::default());
    Self::new(instance, None).await
  }

  pub async fn new(instance: wgpu::Instance, surface: Option<&wgpu::Surface>) -> WgpuImpl {
    let adapter = instance
      .request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::default(),
        compatible_surface: surface,
        force_fallback_adapter: false,
      })
      .await
      .unwrap();

    let (device, queue) = adapter
      .request_device(
        &wgpu::DeviceDescriptor {
          label: Some("Request device"),
          features: wgpu::Features::TEXTURE_BINDING_ARRAY
            | wgpu::Features::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING,
          limits: Default::default(),
        },
        None,
      )
      .await
      .unwrap();

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

    let draw_tex_pass = DrawTexturePass::new(&device);
    let draw_alpha_triangles_pass = DrawAlphaTrianglesPass::new(&device);

    let draw_color_triangles_pass = DrawColorTrianglesPass::new(&device);
    let draw_img_triangles_pass = DrawImgTrianglesPass::new(&device);
    let draw_radial_gradient_pass = DrawRadialGradientTrianglesPass::new(&device);
    let draw_linear_gradient_pass = DrawLinearGradientTrianglesPass::new(&device);
    let mask_layers_storage = Storage::new(&device, wgpu::ShaderStages::FRAGMENT, 512);
    WgpuImpl {
      device,
      queue,
      command_encoder: None,
      command_buffers: vec![],
      sampler,

      draw_tex_pass,
      draw_alpha_triangles_pass,
      draw_color_triangles_pass,
      draw_img_triangles_pass,
      draw_radial_gradient_pass,
      draw_linear_gradient_pass,
      textures_bind: TexturesBind::default(),
      mask_layers_storage,
    }
  }

  pub fn start_capture(&self) { self.device.start_capture(); }

  pub fn stop_capture(&self) { self.device.stop_capture(); }

  pub fn device(&self) -> &wgpu::Device { &self.device }

  pub(crate) fn submit(&mut self) {
    if let Some(encoder) = self.command_encoder.take() {
      self.command_buffers.push(encoder.finish());
    }
    if !self.command_buffers.is_empty() {
      self.draw_tex_pass.submit(&self.queue);
      self.queue.submit(self.command_buffers.drain(..));
    } else {
      self.draw_tex_pass.clear();
    }
  }

  fn copy_same_format_texture(
    &mut self,
    dist_tex: &wgpu::Texture,
    copy_to: DevicePoint,
    from_tex: &wgpu::Texture,
    from_rect: &DeviceRect,
  ) {
    assert_eq!(dist_tex.format(), from_tex.format());

    let encoder = command_encoder!(self);
    let src_origin = wgpu::Origin3d {
      x: from_rect.min_x() as u32,
      y: from_rect.min_y() as u32,
      z: 0,
    };
    let dst_origin = wgpu::Origin3d {
      x: copy_to.x as u32,
      y: copy_to.y as u32,
      z: 0,
    };
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
}

impl TexturesBind {
  pub fn textures_count(&self) -> usize { self.texture_cnt }

  pub fn assert_layout(&self) -> &wgpu::BindGroupLayout { self.textures_layout.as_ref().unwrap() }

  pub fn assert_bind(&self) -> &wgpu::BindGroup { self.textures_bind.as_ref().unwrap() }

  fn load_textures(
    &mut self,
    device: &wgpu::Device,
    sampler: &wgpu::Sampler,
    textures: &[&WgpuTexture],
  ) {
    let mut views = Vec::with_capacity(textures.len());
    for t in textures.iter() {
      views.push(t.view());
    }

    if self.texture_cnt != views.len() {
      self.texture_cnt = views.len();
      let texture_size = NonZeroU32::new(self.texture_cnt as u32);
      let textures_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        entries: &[
          wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
              sample_type: wgpu::TextureSampleType::Float { filterable: true },
              view_dimension: wgpu::TextureViewDimension::D2,
              multisampled: false,
            },
            count: texture_size,
          },
          wgpu::BindGroupLayoutEntry {
            binding: 1,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            count: texture_size,
          },
        ],
        label: Some("Textures layout"),
      });

      self.textures_layout = Some(textures_layout);
    }

    let samplers = vec![sampler; views.len()];

    let texture_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
      layout: self.textures_layout.as_ref().unwrap(),
      entries: &[
        wgpu::BindGroupEntry {
          binding: 0,
          resource: wgpu::BindingResource::TextureViewArray(&views),
        },
        wgpu::BindGroupEntry {
          binding: 1,
          resource: wgpu::BindingResource::SamplerArray(&samplers),
        },
      ],
      label: Some("textures bind group"),
    });

    self.textures_bind = Some(texture_bind);
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
