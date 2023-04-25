use crate::{gpu_backend::Texture, ColorPrimitive, GPUBackendImpl, IndicesRange, TexturePrimitive};
use draw_alpha_triangles_pass::DrawAlphaTrianglesPass;
use draw_text_pass::DrawTextPass;
use draw_text_with_mask_pass::DrawTexWithMaskPass;
use draw_triangles_pass::DrawTrianglesPass;
use futures::{channel::oneshot, Future};
use ribir_painter::{
  image::ColorFormat, AntiAliasing, DevicePoint, DeviceRect, DeviceSize, PixelImage, VertexBuffers,
};
use std::{any::type_name, error::Error, mem::size_of, num::NonZeroU32, ops::Range, pin::Pin};

use buffer_pool::BufferPool;
mod buffer_pool;

mod draw_alpha_triangles_pass;
mod draw_text_pass;
mod draw_text_with_mask_pass;
mod draw_triangles_pass;

const COORDINATE_3D_POOL_SIZE: usize = 512;

pub struct WgpuImpl {
  device: wgpu::Device,
  queue: wgpu::Queue,

  command_encoder: Option<wgpu::CommandEncoder>,
  command_buffers: Vec<wgpu::CommandBuffer>,

  coordinate_pool: BufferPool<[[f32; 4]; 4]>,
  coordinate_uniform: wgpu::Buffer,
  coordinate_layout: wgpu::BindGroupLayout,
  coordinate_bind: wgpu::BindGroup,

  sampler: wgpu::Sampler,
  draw_tex_pass: DrawTextPass,
  draw_text_with_mask_pass: DrawTexWithMaskPass,
  draw_alpha_triangles_pass: DrawAlphaTrianglesPass,
  draw_triangles_pass: DrawTrianglesPass,
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
  type Texture = wgpu::Texture;

  fn map_x(x: f32, width: f32) -> f32 { 2. * x / width - 1. }

  fn map_y(y: f32, height: f32) -> f32 { 2. * y / height + 1. }

  fn set_anti_aliasing(&mut self, anti_aliasing: AntiAliasing) {
    self
      .draw_alpha_triangles_pass
      .set_anti_aliasing(anti_aliasing, &self.device);
  }

  fn begin_frame(&mut self) {
    assert!(self.command_encoder.is_none());
    let encoder = self
      .device
      .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Frame Encoder") });
    self.command_encoder = Some(encoder);
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
    self.device.create_texture(texture_descriptor)
  }

  fn load_color_primitives(&mut self, primitives: &[ColorPrimitive]) {
    self
      .draw_triangles_pass
      .load_color_primitives(primitives, &self.device, &mut self.queue);
  }

  fn load_texture_primitives(&mut self, primitives: &[TexturePrimitive]) {
    self
      .draw_triangles_pass
      .load_texture_primitives(primitives, &self.device, &mut self.queue);
  }

  fn load_textures<'a, Iter>(&mut self, textures: Iter)
  where
    Iter: Iterator<Item = &'a Self::Texture> + 'a,
    Self::Texture: 'a,
  {
    self.draw_triangles_pass.load_textures(
      textures,
      &self.device,
      &self.coordinate_layout,
      &self.sampler,
    );
  }

  fn load_alpha_vertices(&mut self, buffers: &VertexBuffers<()>) {
    self
      .draw_alpha_triangles_pass
      .load_alpha_vertices(buffers, &self.device, &mut self.queue);
  }

  fn draw_alpha_triangles(&mut self, indices: &Range<u32>, texture: &mut Self::Texture) {
    self.write_coordinate(&texture);
    let encoder = command_encoder!(self);
    self.draw_alpha_triangles_pass.draw_alpha_triangles(
      indices,
      texture,
      None,
      encoder,
      &self.coordinate_bind,
      &self.device,
    );
  }

  fn draw_alpha_triangles_with_scissor(
    &mut self,
    indices: &Range<u32>,
    texture: &mut Self::Texture,
    scissor: DeviceRect,
  ) {
    self.write_coordinate(&texture);
    let encoder = command_encoder!(self);
    self.draw_alpha_triangles_pass.draw_alpha_triangles(
      indices,
      texture,
      Some(scissor),
      encoder,
      &self.coordinate_bind,
      &self.device,
    );
  }

  fn load_triangles_vertices(&mut self, buffers: &VertexBuffers<u32>) {
    self
      .draw_triangles_pass
      .load_triangles_vertices(buffers, &self.device, &mut self.queue);
  }

  fn draw_triangles(
    &mut self,
    texture: &mut Self::Texture,
    scissor: &DeviceRect,
    range: IndicesRange,
  ) {
    self.write_coordinate(&texture);
    let encoder = command_encoder!(self);
    self.draw_triangles_pass.draw_triangles(
      texture,
      scissor,
      range,
      &self.device,
      encoder,
      &self.coordinate_bind,
    );
  }

  fn draw_texture_with_mask(
    &mut self,
    dist_tex: &mut Self::Texture,
    dist_start_at: DevicePoint,
    src_tex: &Self::Texture,
    src_start_at: DevicePoint,
    mask: &Self::Texture,
    mask_rect: &DeviceRect,
  ) {
    self.draw_texture_with_mask(
      dist_tex,
      dist_start_at,
      src_tex,
      src_start_at,
      mask,
      mask_rect,
    );
  }

  fn end_frame(&mut self) {
    self.submit();
    self.device.poll(wgpu::Maintain::Wait);
  }

  fn copy_texture_to_texture(
    &mut self,
    dist_tex: &mut Self::Texture,
    copy_to: DevicePoint,
    from_tex: &Self::Texture,
    from_rect: &DeviceRect,
  ) {
    if dist_tex.format() == from_tex.format() {
      self.copy_same_format_texture(dist_tex, copy_to, from_tex, from_rect);
    } else {
      self.draw_texture_to_texture(dist_tex, copy_to, from_tex, from_rect)
    }
  }
}

impl Texture for wgpu::Texture {
  type Host = WgpuImpl;

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
        texture: &self,
        mip_level: 0,
        origin,
        aspect: wgpu::TextureAspect::All,
      },
      data,
      // The layout of the texture
      wgpu::ImageDataLayout {
        offset: 0,
        bytes_per_row: std::num::NonZeroU32::new(bytes_per_pixel as u32 * size.width),
        rows_per_image: std::num::NonZeroU32::new(size.height),
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
    let align_width = align(width as u32, wgpu::COPY_BYTES_PER_ROW_ALIGNMENT / 4);
    let pixel_bytes = format.pixel_per_bytes();
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
        texture: &self,
        mip_level: 0,
        origin,
        aspect: wgpu::TextureAspect::All,
      },
      wgpu::ImageCopyBuffer {
        buffer: &buffer,
        layout: wgpu::ImageDataLayout {
          offset: 0,
          bytes_per_row: NonZeroU32::new(padded_row_bytes),
          rows_per_image: NonZeroU32::new(0),
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
        data[row_start..row_start + row_bytes as usize]
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
      wgpu::TextureFormat::Stencil8 => ColorFormat::Alpha8,
      wgpu::TextureFormat::Rgba8Unorm => ColorFormat::Rgba8,
      _ => panic!("not a valid texture as image"),
    }
  }

  fn size(&self) -> DeviceSize {
    let wgpu::Extent3d { width, height, .. } = self.size();
    DeviceSize::new(width as i32, height as i32)
  }
}

impl WgpuImpl {
  pub async fn headless(anti_aliasing: AntiAliasing) -> Self {
    let instance = wgpu::Instance::new(<_>::default());
    Self::new(anti_aliasing, instance, None).await
  }

  pub async fn new(
    anti_aliasing: AntiAliasing,
    instance: wgpu::Instance,
    surface: Option<&wgpu::Surface>,
  ) -> WgpuImpl {
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

    let coordinate_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
      entries: &[wgpu::BindGroupLayoutEntry {
        binding: 0,
        visibility: wgpu::ShaderStages::VERTEX,
        ty: wgpu::BindingType::Buffer {
          ty: wgpu::BufferBindingType::Uniform,
          has_dynamic_offset: false,
          min_binding_size: None,
        },
        count: None,
      }],
      label: Some("Coordinate uniform layout"),
    });

    let coordinate_pool = BufferPool::new(
      COORDINATE_3D_POOL_SIZE,
      wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
      &device,
    );
    let coordinate_uniform = device.create_buffer(&wgpu::BufferDescriptor {
      label: Some("Coordinate uniform"),
      size: (size_of::<[[f32; 4]; 4]>()) as u64,
      usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
      mapped_at_creation: false,
    });
    let coordinate_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
      layout: &coordinate_layout,
      entries: &[wgpu::BindGroupEntry {
        binding: 0,
        resource: coordinate_uniform.as_entire_binding(),
      }],
      label: Some("Coordinate uniform bind group"),
    });

    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
      address_mode_u: wgpu::AddressMode::ClampToEdge,
      address_mode_v: wgpu::AddressMode::ClampToEdge,
      address_mode_w: wgpu::AddressMode::ClampToEdge,
      mag_filter: wgpu::FilterMode::Nearest,
      min_filter: wgpu::FilterMode::Nearest,
      mipmap_filter: wgpu::FilterMode::Nearest,
      lod_min_clamp: 0.0,
      lod_max_clamp: 0.0,
      label: Some("texture sampler"),
      ..Default::default()
    });

    let draw_tex_pass = DrawTextPass::new(&device);
    let draw_text_with_mask_pass = DrawTexWithMaskPass::new(&device);
    let draw_alpha_triangles_pass =
      DrawAlphaTrianglesPass::new(anti_aliasing, &device, &coordinate_layout);
    let draw_triangles_pass = DrawTrianglesPass::new(&device);

    WgpuImpl {
      device,
      queue,
      command_encoder: None,
      command_buffers: vec![],

      coordinate_pool,
      coordinate_uniform,
      coordinate_layout,
      coordinate_bind,

      sampler,

      draw_tex_pass,
      draw_text_with_mask_pass,
      draw_alpha_triangles_pass,
      draw_triangles_pass,
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
      self.coordinate_pool.submit_buffer(&mut self.queue);
      self.draw_tex_pass.submit(&mut self.queue);
      self.draw_text_with_mask_pass.submit(&mut self.queue);
      self.queue.submit(self.command_buffers.drain(..));
    } else {
      self.coordinate_pool.clear();
      self.draw_tex_pass.clear();
      self.draw_text_with_mask_pass.clear();
    }
  }

  fn new_storage<T>(device: &wgpu::Device, len: usize) -> wgpu::Buffer {
    let label = format!("{} storage buffer", type_name::<T>());
    device.create_buffer(&wgpu::BufferDescriptor {
      label: Some(&label),
      size: (len * size_of::<T>()) as u64,
      usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
      mapped_at_creation: false,
    })
  }

  fn new_vertices<P>(device: &wgpu::Device, len: usize) -> wgpu::Buffer {
    let label = format!("{} vertices buffer", type_name::<P>());
    device.create_buffer(&wgpu::BufferDescriptor {
      label: Some(&label),
      size: (len * size_of::<P>()) as u64,
      usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
      mapped_at_creation: false,
    })
  }

  fn new_indices(device: &wgpu::Device, len: usize) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
      label: Some("indices buffer"),
      size: (len * size_of::<u32>()) as u64,
      usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
      mapped_at_creation: false,
    })
  }

  fn write_coordinate(&mut self, main_texture: &wgpu::Texture) {
    if self.coordinate_pool.is_full() {
      self.submit();
    }
    let address = self
      .coordinate_pool
      .push_value([
        [2. / main_texture.width() as f32, 0., 0., 0.],
        [0., -2. / main_texture.height() as f32, 0., 0.],
        [0., 0., 1., 0.],
        [-1., 1., 0., 1.],
      ])
      .unwrap();

    let coordinate_bytes = size_of::<[[f32; 4]; 4]>() as u64;
    let encoder = command_encoder!(self);

    // The `coordinate_data` will write to `coordinate_pool` before encoder submit.
    encoder.copy_buffer_to_buffer(
      &self.coordinate_pool.buffer(),
      address,
      &self.coordinate_uniform,
      0,
      coordinate_bytes,
    );
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
