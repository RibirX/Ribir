use futures::{channel::oneshot, Future};
use std::{any::type_name, error::Error, mem::size_of, num::NonZeroU32, ops::Range, pin::Pin};

use crate::{ColorPrimitive, DrawIndices, GPUBackendImpl, TexturePrimitive};
use ribir_painter::{
  image::ColorFormat, AntiAliasing, DevicePoint, DeviceRect, DeviceSize, PixelImage, Texture,
  Vertex, VertexBuffers,
};
use zerocopy::AsBytes;

/// The max 3d coordinate can used in a frame
/// - every draw triangles call need one.
const MAX_3D_COORDINATE_PER_FRAME: usize = 256;

pub struct WgpuImpl {
  device: wgpu::Device,
  queue: wgpu::Queue,
  anti_aliasing: AntiAliasing,
  command_encoder: Option<wgpu::CommandEncoder>,
  command_buffers: Vec<wgpu::CommandBuffer>,

  /// The coordinate matrix used in current frame.
  coordinate_data: Vec<[[f32; 4]; 4]>,
  /// Buffer store the coordinate_data and use to update coordinate_uniform.
  coordinate_pool: wgpu::Buffer,
  coordinate_uniform: wgpu::Buffer,
  coordinate_layout: wgpu::BindGroupLayout,
  coordinate_bind: wgpu::BindGroup,

  alpha_vertices_buffer: wgpu::Buffer,
  alpha_indices_buffer: wgpu::Buffer,
  alpha_triangles_pipeline: wgpu::RenderPipeline,
  alpha_multisample: Option<AlphaMultiSample>,

  primitives_layout: wgpu::BindGroupLayout,

  color_primitives_buffer: wgpu::Buffer,
  color_primitives_bind: wgpu::BindGroup,
  color_triangles_pipeline: Option<wgpu::RenderPipeline>,

  tex_primitives_buffer: wgpu::Buffer,
  tex_primitives_bind: wgpu::BindGroup,
  tex_triangles_pipeline: Option<wgpu::RenderPipeline>,

  textures: Vec<wgpu::TextureView>,
  textures_bind: Option<wgpu::BindGroup>,
  textures_layout: Option<wgpu::BindGroupLayout>,
  sampler: wgpu::Sampler,

  vertices_buffer: wgpu::Buffer,
  indices_buffer: wgpu::Buffer,
  tex_view_desc: wgpu::TextureViewDescriptor<'static>,
}

pub struct WgpuTexture(wgpu::Texture);

struct AlphaMultiSample {
  texture: wgpu::Texture,
  view: wgpu::TextureView,
  anti_aliasing: AntiAliasing,
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

impl GPUBackendImpl for WgpuImpl {
  type Texture = WgpuTexture;
  fn set_anti_aliasing(&mut self, anti_aliasing: AntiAliasing) {
    if self.anti_aliasing != anti_aliasing {
      self.anti_aliasing = anti_aliasing;
      self.alpha_triangles_pipeline =
        alpha_triangles_pipeline(&self.device, &self.coordinate_layout, anti_aliasing as u32)
    }
  }

  fn begin_frame(&mut self) {
    assert!(self.command_encoder.is_none());
    let encoder = self
      .device
      .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Frame Encoder") });
    self.command_encoder = Some(encoder);
  }

  fn start_draw_phase(&mut self) {}

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
    let texture = self.device.create_texture(texture_descriptor);
    WgpuTexture(texture)
  }

  fn load_color_primitives(&mut self, primitives: &[ColorPrimitive]) {
    let buffer_len = self.color_primitives_buffer.size() as usize / size_of::<ColorPrimitive>();
    if buffer_len < primitives.len() {
      self.color_primitives_buffer =
        Self::new_storage::<ColorPrimitive>(&self.device, primitives.len());
      self.color_primitives_bind = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &self.primitives_layout,
        entries: &[wgpu::BindGroupEntry {
          binding: 0,
          resource: self.color_primitives_buffer.as_entire_binding(),
        }],
        label: Some("Color primitives storage bind group"),
      });
    }
    self
      .queue
      .write_buffer(&self.color_primitives_buffer, 0, primitives.as_bytes());
  }

  fn load_texture_primitives(&mut self, primitives: &[TexturePrimitive]) {
    let buffer_len = self.tex_primitives_buffer.size() as usize / size_of::<TexturePrimitive>();
    if buffer_len < primitives.len() {
      self.tex_primitives_buffer =
        Self::new_storage::<TexturePrimitive>(&self.device, primitives.len());
      self.tex_primitives_bind = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &self.primitives_layout,
        entries: &[wgpu::BindGroupEntry {
          binding: 0,
          resource: self.tex_primitives_buffer.as_entire_binding(),
        }],
        label: Some("Texture primitives storage bind group"),
      });
    }
    self
      .queue
      .write_buffer(&self.tex_primitives_buffer, 0, primitives.as_bytes());
  }

  fn load_textures<'a, Iter>(&mut self, textures: Iter)
  where
    Iter: Iterator<Item = &'a Self::Texture> + 'a,
    Self::Texture: 'a,
  {
    self.textures.clear();
    for t in textures {
      self.textures.push(t.0.create_view(&self.tex_view_desc))
    }

    let texture_size = NonZeroU32::new(self.textures.len() as u32);
    let textures_layout = self
      .device
      .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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

    let mut views = Vec::with_capacity(self.textures.len());
    self.textures.iter().for_each(|v| views.push(v));
    let samplers = vec![&self.sampler; self.textures.len()];

    let texture_bind = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
      layout: &textures_layout,
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
      label: Some("color triangles bind group"),
    });

    self.textures_layout = Some(textures_layout);
    self.textures_bind = Some(texture_bind);
    self.update_color_triangles_pipeline();
    self.update_texture_triangles_pipeline();
  }

  fn load_alpha_vertices(&mut self, buffers: &VertexBuffers<()>) {
    let VertexBuffers { vertices, indices } = buffers;
    let v_buffer_len = self.alpha_vertices_buffer.size() as usize / size_of::<Vertex<()>>();
    if v_buffer_len < vertices.len() {
      self.alpha_vertices_buffer = Self::new_vertices::<Vertex<()>>(&self.device, vertices.len());
    }
    self
      .queue
      .write_buffer(&self.alpha_vertices_buffer, 0, vertices.as_bytes());

    let i_buffer_len = self.alpha_indices_buffer.size() as usize / size_of::<Vertex<()>>();
    if i_buffer_len < indices.len() {
      self.alpha_indices_buffer = Self::new_indices(&self.device, indices.len());
    }
    self
      .queue
      .write_buffer(&self.alpha_indices_buffer, 0, indices.as_bytes());
  }

  fn draw_alpha_triangles(&mut self, indices: &Range<u32>, texture: &mut Self::Texture) {
    self.draw_alpha_triangles(indices, texture, None)
  }

  fn draw_alpha_triangles_with_scissor(
    &mut self,
    indices: &Range<u32>,
    texture: &mut Self::Texture,
    scissor: DeviceRect,
  ) {
    self.draw_alpha_triangles(indices, texture, Some(scissor))
  }

  fn load_triangles_vertices(&mut self, buffers: &VertexBuffers<u32>) {
    let VertexBuffers { vertices, indices } = buffers;
    let v_buffer_len = self.vertices_buffer.size() as usize / size_of::<Vertex<u32>>();
    if v_buffer_len < vertices.len() {
      self.vertices_buffer = Self::new_vertices::<Vertex<u32>>(&self.device, vertices.len());
    }
    self
      .queue
      .write_buffer(&self.vertices_buffer, 0, vertices.as_bytes());

    let i_buffer_len = self.indices_buffer.size() as usize / size_of::<u32>();
    if i_buffer_len < indices.len() {
      self.indices_buffer = Self::new_indices(&self.device, indices.len());
    }
    self
      .queue
      .write_buffer(&self.indices_buffer, 0, indices.as_bytes());
  }

  fn draw_triangles(
    &mut self,
    texture: &mut Self::Texture,
    scissor: DeviceRect,
    commands: &[DrawIndices],
  ) {
    self.draw_pre_update(&texture.0);
    let view = texture.0.create_view(&self.tex_view_desc);
    let encoder = command_encoder!(self);
    let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
      label: Some("Triangles render pass"),
      color_attachments: &[Some(wgpu::RenderPassColorAttachment {
        view: &view,
        resolve_target: None,
        ops: wgpu::Operations {
          load: wgpu::LoadOp::Load,
          store: true,
        },
      })],
      depth_stencil_attachment: None,
    });
    rpass.set_vertex_buffer(0, self.vertices_buffer.slice(..));
    rpass.set_index_buffer(self.indices_buffer.slice(..), wgpu::IndexFormat::Uint32);
    rpass.set_bind_group(0, &self.coordinate_bind, &[]);
    let texture_group = self
      .textures_bind
      .as_ref()
      .expect("Should load textures before draws!");
    rpass.set_bind_group(1, texture_group, &[]);

    rpass.set_scissor_rect(
      scissor.min_x() as u32,
      scissor.min_y() as u32,
      scissor.width() as u32,
      scissor.height() as u32,
    );

    commands.iter().for_each(|cmd| match cmd {
      DrawIndices::Color(rg) => {
        rpass.set_bind_group(2, &self.color_primitives_bind, &[]);
        let pipeline = self.color_triangles_pipeline.as_ref().unwrap();
        rpass.set_pipeline(pipeline);
        rpass.draw_indexed(rg.clone(), 0, 0..1);
      }
      DrawIndices::Texture(rg) => {
        rpass.set_bind_group(2, &self.tex_primitives_bind, &[]);
        let pipeline = self.tex_triangles_pipeline.as_ref().unwrap();
        rpass.set_pipeline(&pipeline);
        rpass.draw_indexed(rg.clone(), 0, 0..1);
      }
      DrawIndices::Gradient(_) => todo!(),
    })
  }

  fn end_draw_phase(&mut self) {}

  fn end_frame(&mut self) {
    self.submit();
    self.device.poll(wgpu::Maintain::Wait);
  }
}

impl Texture for WgpuTexture {
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
    let bytes_per_pixel = self.format().pixel_per_bytes();
    backend.queue.write_texture(
      wgpu::ImageCopyTexture {
        texture: &self.0,
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

  fn copy_from_texture(
    &mut self,
    copy_to: DevicePoint,
    from_texture: &Self,
    from_rect: DeviceRect,
    backend: &mut Self::Host,
  ) {
    let encoder = command_encoder!(backend);
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
        texture: &from_texture.0,
        mip_level: 0,
        origin: src_origin,
        aspect: wgpu::TextureAspect::All,
      },
      wgpu::ImageCopyTexture {
        texture: &self.0,
        mip_level: 0,
        origin: dst_origin,
        aspect: wgpu::TextureAspect::All,
      },
      copy_size,
    );
  }

  fn copy_as_image(
    &self,
    rect: &DeviceRect,
    backend: &mut Self::Host,
  ) -> Pin<Box<dyn Future<Output = Result<PixelImage, Box<dyn Error>>> + Send + Sync>> {
    let width = rect.width();
    let height = rect.height();
    let format = self.format();
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
        texture: &self.0,
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

      let unpadded_row_bytes = width as usize * pixel_bytes as usize;
      let mut data = vec![0; unpadded_row_bytes * height as usize];

      let slice = buffer.slice(..).get_mapped_range();
      (0..height as usize).for_each(|r| {
        let padded_start = r * padded_row_bytes as usize;
        let unppaded_start = r * unpadded_row_bytes;
        data[unppaded_start..unppaded_start + unpadded_row_bytes as usize]
          .copy_from_slice(&slice[padded_start..padded_start + unpadded_row_bytes]);
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

  fn format(&self) -> ColorFormat {
    match self.0.format() {
      wgpu::TextureFormat::R8Unorm => ColorFormat::Alpha8,
      wgpu::TextureFormat::Rgba8Unorm => ColorFormat::Rgba8,
      _ => unreachable!(),
    }
  }

  fn size(&self) -> DeviceSize {
    let wgpu::Extent3d { width, height, .. } = self.0.size();
    DeviceSize::new(width as i32, height as i32)
  }
}

impl WgpuImpl {
  pub async fn new(anti_aliasing: AntiAliasing) -> WgpuImpl {
    let instance = wgpu::Instance::new(<_>::default());

    let adapter = instance
      .request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::default(),
        compatible_surface: None,
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
    let coordinate_data = Vec::with_capacity(MAX_3D_COORDINATE_PER_FRAME);
    let coordinate_pool = device.create_buffer(&wgpu::BufferDescriptor {
      label: Some("Coordinate pool"),
      size: (size_of::<[[f32; 4]; 4]>() * MAX_3D_COORDINATE_PER_FRAME) as u64,
      usage: wgpu::BufferUsages::UNIFORM
        | wgpu::BufferUsages::COPY_SRC
        | wgpu::BufferUsages::COPY_DST,
      mapped_at_creation: false,
    });
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

    let color_primitives_buffer = Self::new_storage::<ColorPrimitive>(&device, 256);
    let primitives_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
      entries: &[wgpu::BindGroupLayoutEntry {
        binding: 0,
        visibility: wgpu::ShaderStages::VERTEX,
        ty: wgpu::BindingType::Buffer {
          ty: wgpu::BufferBindingType::Storage { read_only: true },
          has_dynamic_offset: false,
          min_binding_size: None,
        },
        count: None,
      }],
      label: Some("Color primitives storage layout"),
    });

    let color_primitives_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
      layout: &primitives_layout,
      entries: &[wgpu::BindGroupEntry {
        binding: 0,
        resource: color_primitives_buffer.as_entire_binding(),
      }],
      label: Some("Color primitives storage bind group"),
    });

    let tex_primitives_buffer = Self::new_storage::<TexturePrimitive>(&device, 256);
    let tex_primitives_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
      layout: &primitives_layout,
      entries: &[wgpu::BindGroupEntry {
        binding: 0,
        resource: tex_primitives_buffer.as_entire_binding(),
      }],
      label: Some("Texture primitives storage bind group"),
    });

    let alpha_vertices_buffer = Self::new_vertices::<Vertex<()>>(&device, 1024);
    let alpha_indices_buffer = Self::new_indices(&device, 1024);
    let alpha_triangles_pipeline =
      alpha_triangles_pipeline(&device, &coordinate_layout, anti_aliasing as u32);

    let vertices_buffer = Self::new_vertices::<Vertex<u32>>(&device, 512);
    let indices_buffer = Self::new_indices(&device, 1024);
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
    WgpuImpl {
      device,
      queue,
      command_encoder: None,
      command_buffers: vec![],
      anti_aliasing,

      coordinate_data,
      coordinate_pool,
      coordinate_uniform,
      coordinate_layout,
      coordinate_bind,

      alpha_vertices_buffer,
      alpha_indices_buffer,

      color_primitives_buffer,
      primitives_layout,
      color_primitives_bind,
      color_triangles_pipeline: None,

      tex_primitives_buffer,
      tex_primitives_bind,
      tex_triangles_pipeline: None,

      vertices_buffer,
      indices_buffer,
      textures: vec![],
      tex_view_desc: <_>::default(),
      alpha_triangles_pipeline,
      alpha_multisample: None,

      sampler,
      textures_bind: None,
      textures_layout: None,
    }
  }

  pub fn start_capture(&self) { self.device.start_capture(); }

  pub fn stop_capture(&self) { self.device.stop_capture(); }

  fn submit(&mut self) {
    if let Some(encoder) = self.command_encoder.take() {
      self.command_buffers.push(encoder.finish());
    }
    if !self.command_buffers.is_empty() {
      self
        .queue
        .write_buffer(&self.coordinate_pool, 0, &self.coordinate_data.as_bytes());
      self.queue.submit(self.command_buffers.drain(..));
    }
    self.coordinate_data.clear();
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

  fn draw_pre_update(&mut self, target: &wgpu::Texture) {
    self.switch_main_texture(target);
    self.update_alpha_multi_sample(target);
  }

  fn switch_main_texture(&mut self, main_texture: &wgpu::Texture) {
    if self.coordinate_data.len() == self.coordinate_data.capacity() {
      self.submit();
    }

    let coordinate_bytes = size_of::<[[f32; 4]; 4]>() as u64;
    let encoder = command_encoder!(self);

    // The `coordinate_data` will write to `coordinate_pool` before encoder submit.
    encoder.copy_buffer_to_buffer(
      &self.coordinate_pool,
      self.coordinate_data.len() as u64 * coordinate_bytes,
      &self.coordinate_uniform,
      0,
      coordinate_bytes,
    );
    self.coordinate_data.push([
      [2. / main_texture.width() as f32, 0., 0., 0.],
      [0., -2. / main_texture.height() as f32, 0., 0.],
      [0., 0., 1., 0.],
      [-1., 1., 0., 1.],
    ]);
  }

  fn update_alpha_multi_sample(&mut self, target: &wgpu::Texture) {
    if self.anti_aliasing == AntiAliasing::None {
      self.alpha_multisample.take();
      return;
    }

    if let Some(sample) = self.alpha_multisample.as_ref() {
      if sample.anti_aliasing == self.anti_aliasing && sample.texture.size() == target.size() {
        return;
      }
    }

    self.alpha_multisample.take();

    let sample_count = self.anti_aliasing as u32;
    let multisampled_texture_extent = wgpu::Extent3d {
      width: target.width(),
      height: target.height(),
      depth_or_array_layers: 1,
    };
    let multisampled_frame_descriptor = &wgpu::TextureDescriptor {
      size: multisampled_texture_extent,
      mip_level_count: 1,
      sample_count,
      dimension: wgpu::TextureDimension::D2,
      format: wgpu::TextureFormat::R8Unorm,
      usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
      label: None,
      view_formats: &[],
    };

    let texture = self.device.create_texture(multisampled_frame_descriptor);
    let view = texture.create_view(&self.tex_view_desc);
    self.alpha_multisample = Some(AlphaMultiSample {
      texture,
      view,
      anti_aliasing: self.anti_aliasing,
    })
  }

  fn draw_alpha_triangles(
    &mut self,
    indices: &Range<u32>,
    texture: &mut WgpuTexture,
    scissor: Option<DeviceRect>,
  ) {
    self.draw_pre_update(&texture.0);
    let view = texture.0.create_view(&self.tex_view_desc);
    let color_attachments = if let Some(multi_sample) = self.alpha_multisample.as_ref() {
      wgpu::RenderPassColorAttachment {
        view: &multi_sample.view,
        resolve_target: Some(&view),
        ops: wgpu::Operations {
          load: wgpu::LoadOp::Load,
          store: false,
        },
      }
    } else {
      wgpu::RenderPassColorAttachment {
        view: &view,
        resolve_target: None,
        ops: wgpu::Operations {
          load: wgpu::LoadOp::Load,
          store: true,
        },
      }
    };

    let encoder = command_encoder!(self);
    let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
      label: Some("Alpha triangles render pass"),
      color_attachments: &[Some(color_attachments)],
      depth_stencil_attachment: None,
    });
    rpass.set_vertex_buffer(0, self.alpha_vertices_buffer.slice(..));
    rpass.set_index_buffer(
      self.alpha_indices_buffer.slice(..),
      wgpu::IndexFormat::Uint32,
    );

    rpass.set_bind_group(0, &self.coordinate_bind, &[]);
    if let Some(scissor) = scissor {
      rpass.set_scissor_rect(
        scissor.min_x() as u32,
        scissor.min_y() as u32,
        scissor.width() as u32,
        scissor.height() as u32,
      );
    }
    rpass.set_pipeline(&self.alpha_triangles_pipeline);
    rpass.draw_indexed(indices.clone(), 0, 0..1)
  }

  fn update_color_triangles_pipeline(&mut self) {
    let pipeline = self.triangles_pipeline(
      include_str!("./wgpu_impl/shaders/color_triangles.wgsl"),
      "Color triangles",
    );
    self.color_triangles_pipeline = Some(pipeline);
  }

  fn update_texture_triangles_pipeline(&mut self) {
    let pipeline = self.triangles_pipeline(
      include_str!("./wgpu_impl/shaders/tex_triangles.wgsl"),
      "Texture triangles",
    );
    self.tex_triangles_pipeline = Some(pipeline);
  }

  fn triangles_pipeline(&mut self, shader: &str, label: &str) -> wgpu::RenderPipeline {
    let label = Some(label);
    let pipeline_layout = self
      .device
      .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label,
        bind_group_layouts: &[
          &self.coordinate_layout,
          &self.textures_layout.as_ref().unwrap(),
          &self.primitives_layout,
        ],
        push_constant_ranges: &[],
      });

    let module = self
      .device
      .create_shader_module(wgpu::ShaderModuleDescriptor {
        label,
        source: wgpu::ShaderSource::Wgsl(shader.into()),
      });
    self
      .device
      .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label,
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
          module: &module,
          entry_point: "vs_main",
          buffers: &[wgpu::VertexBufferLayout {
            array_stride: size_of::<Vertex<u32>>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
              wgpu::VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: wgpu::VertexFormat::Float32x2,
              },
              wgpu::VertexAttribute {
                offset: (size_of::<[f32; 2]>()) as wgpu::BufferAddress,
                shader_location: 1,
                format: wgpu::VertexFormat::Uint32,
              },
            ],
          }],
        },
        fragment: Some(wgpu::FragmentState {
          module: &module,
          entry_point: "fs_main",
          targets: &[Some(wgpu::ColorTargetState {
            format: wgpu::TextureFormat::Rgba8Unorm,
            blend: Some(wgpu::BlendState::ALPHA_BLENDING),
            write_mask: wgpu::ColorWrites::all(),
          })],
        }),
        primitive: wgpu::PrimitiveState {
          topology: wgpu::PrimitiveTopology::TriangleList,
          strip_index_format: None,
          front_face: wgpu::FrontFace::Ccw,
          cull_mode: Some(wgpu::Face::Back),
          unclipped_depth: false,
          polygon_mode: wgpu::PolygonMode::Fill,
          conservative: false,
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState {
          count: 1,
          mask: !0,
          alpha_to_coverage_enabled: false,
        },
        multiview: None,
      })
  }
}

fn into_wgpu_format(format: ColorFormat) -> wgpu::TextureFormat {
  match format {
    ColorFormat::Rgba8 => wgpu::TextureFormat::Rgba8Unorm,
    ColorFormat::Alpha8 => wgpu::TextureFormat::R8Unorm,
  }
}

fn alpha_triangles_pipeline(
  device: &wgpu::Device,
  coordinate_layout: &wgpu::BindGroupLayout,
  msaa_count: u32,
) -> wgpu::RenderPipeline {
  let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
    label: Some("Alpha triangles pipeline layout"),
    bind_group_layouts: &[coordinate_layout],
    push_constant_ranges: &[],
  });

  let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
    label: Some("Alpha triangles"),
    source: wgpu::ShaderSource::Wgsl(
      include_str!("./wgpu_impl/shaders/alpha_triangles.wgsl").into(),
    ),
  });

  device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
    label: Some("Alpha triangles pipeline"),
    layout: Some(&pipeline_layout),
    vertex: wgpu::VertexState {
      module: &module,
      entry_point: "vs_main",
      buffers: &[wgpu::VertexBufferLayout {
        array_stride: size_of::<Vertex<()>>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &[wgpu::VertexAttribute {
          offset: 0,
          shader_location: 0,
          format: wgpu::VertexFormat::Float32x2,
        }],
      }],
    },
    fragment: Some(wgpu::FragmentState {
      module: &module,
      entry_point: "fs_main",
      targets: &[Some(wgpu::ColorTargetState {
        format: wgpu::TextureFormat::R8Unorm,
        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
        write_mask: wgpu::ColorWrites::all(),
      })],
    }),
    primitive: wgpu::PrimitiveState {
      topology: wgpu::PrimitiveTopology::TriangleList,
      strip_index_format: None,
      front_face: wgpu::FrontFace::Ccw,
      cull_mode: Some(wgpu::Face::Back),
      unclipped_depth: false,
      polygon_mode: wgpu::PolygonMode::Fill,
      conservative: false,
    },
    depth_stencil: None,
    multisample: wgpu::MultisampleState {
      count: msaa_count,
      mask: !0,
      alpha_to_coverage_enabled: false,
    },
    multiview: None,
  })
}

fn align(width: u32, align: u32) -> u32 {
  match width % align {
    0 => width,
    other => width - other + align,
  }
}
