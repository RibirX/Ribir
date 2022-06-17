use std::{
  cell::{Ref, RefCell},
  num::{NonZeroU32, NonZeroU64},
};

use super::DeviceSize;
/// `Surface` is a thing presentable canvas visual display.
pub trait Surface {
  fn resize(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, size: DeviceSize);

  fn view_size(&self) -> DeviceSize;

  fn format(&self) -> wgpu::TextureFormat;

  fn current_texture(&self) -> SurfaceTexture;

  fn copy_as_rgba_buffer(
    &self,
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
  ) -> wgpu::Buffer;

  fn present(&mut self);
}

/// A `Surface` represents a platform-specific surface (e.g. a window).
pub struct WindowSurface {
  surface: wgpu::Surface,
  s_config: wgpu::SurfaceConfiguration,
  current_texture: RefCell<Option<wgpu::SurfaceTexture>>,
}

impl Surface for WindowSurface {
  fn resize(&mut self, device: &wgpu::Device, _queue: &wgpu::Queue, size: DeviceSize) {
    self.s_config.width = size.width;
    self.s_config.height = size.height;
    self.surface.configure(device, &self.s_config);
  }

  fn view_size(&self) -> DeviceSize { DeviceSize::new(self.s_config.width, self.s_config.height) }

  fn format(&self) -> wgpu::TextureFormat { self.s_config.format }

  fn current_texture(&self) -> SurfaceTexture {
    self.current_texture.borrow_mut().get_or_insert_with(|| {
      self
        .surface
        .get_current_texture()
        .expect("Timeout getting texture")
    });
    SurfaceTexture::RefCell(Ref::map(self.current_texture.borrow(), |t| {
      &t.as_ref().unwrap().texture
    }))
  }

  fn copy_as_rgba_buffer(
    &self,
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
  ) -> wgpu::Buffer {
    let texture = self.current_texture.borrow();
    let texture = &texture.as_ref().expect("should always have").texture;
    let wgpu::SurfaceConfiguration { width, height, .. } = self.s_config;
    let buffer = texture_to_buffer_4_bytes_per_pixel(device, encoder, texture, width, height);

    let group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
      entries: &[wgpu::BindGroupLayoutEntry {
        binding: 0,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
          ty: wgpu::BufferBindingType::Storage { read_only: false },
          has_dynamic_offset: false,
          min_binding_size: None,
        },
        count: None,
      }],
      label: None,
    });

    let cs_module = device.create_shader_module(&wgpu::include_wgsl!("./shaders/bgra_2_rgba.wgsl"));

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
      bind_group_layouts: &[&group_layout],
      push_constant_ranges: &[],
      label: Some("RGBA convert render pipeline layout"),
    });

    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
      label: Some("image convert pipeline"),
      layout: Some(&pipeline_layout),
      module: &cs_module,
      entry_point: "main",
    });

    let limits = device.limits();
    let max_group = align(
      limits.max_compute_workgroups_per_dimension,
      limits.min_storage_buffer_offset_alignment,
      false,
    );

    const UNIT: u32 = 4;
    {
      let sum = align(width, wgpu::COPY_BYTES_PER_ROW_ALIGNMENT / 4, true) * height;
      let mut offset = 0;

      while offset < sum {
        let size = max_group.min(sum - offset);
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
          layout: &group_layout,
          entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
              buffer: &buffer,
              offset: (offset * UNIT) as u64,
              size: NonZeroU64::new((size * UNIT) as u64),
            }),
          }],
          label: None,
        });
        let mut c_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
          label: Some("Window surface convert to rgba"),
        });
        c_pass.set_pipeline(&pipeline);
        c_pass.set_bind_group(0, &bind_group, &[]);
        c_pass.dispatch(size, 1, 1);

        offset += max_group;
      }
    }

    buffer
  }

  fn present(&mut self) {
    if let Some(texture) = self.current_texture.take() {
      texture.present()
    }
  }
}

pub enum SurfaceTexture<'a> {
  RefCell(Ref<'a, wgpu::Texture>),
  Ref(&'a wgpu::Texture),
}

impl<'a> std::ops::Deref for SurfaceTexture<'a> {
  type Target = wgpu::Texture;

  fn deref(&self) -> &Self::Target {
    match self {
      SurfaceTexture::RefCell(t) => t,
      SurfaceTexture::Ref(t) => t,
    }
  }
}

impl Surface for TextureSurface {
  fn resize(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, size: DeviceSize) {
    let new_texture = Self::new_texture(device, size, self.usage);

    let size = size.min(self.size);
    let mut encoder = device
      .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Render Encoder") });
    encoder.copy_texture_to_texture(
      wgpu::ImageCopyTexture {
        texture: &self.raw_texture,
        mip_level: 0,
        origin: wgpu::Origin3d::ZERO,
        aspect: wgpu::TextureAspect::All,
      },
      wgpu::ImageCopyTexture {
        texture: &new_texture,
        mip_level: 0,
        origin: wgpu::Origin3d::ZERO,
        aspect: wgpu::TextureAspect::All,
      },
      wgpu::Extent3d {
        width: size.width,
        height: size.height,
        depth_or_array_layers: 1,
      },
    );

    queue.submit(Some(encoder.finish()));

    self.size = size;
    self.raw_texture = new_texture;
  }

  fn view_size(&self) -> DeviceSize { self.size }

  fn format(&self) -> wgpu::TextureFormat { TextureSurface::FORMAT }

  fn current_texture(&self) -> SurfaceTexture { SurfaceTexture::Ref(&self.raw_texture) }

  fn copy_as_rgba_buffer(
    &self,
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
  ) -> wgpu::Buffer {
    let DeviceSize { width, height, .. } = self.size;
    texture_to_buffer_4_bytes_per_pixel(device, encoder, &self.raw_texture, width, height)
  }

  fn present(&mut self) {}
}

fn align(width: u32, align: u32, include: bool) -> u32 {
  match width % align {
    0 => width,
    other => {
      let mut aligned = width - other;
      if include {
        aligned += align
      }
      aligned
    }
  }
}

fn texture_to_buffer_4_bytes_per_pixel(
  device: &wgpu::Device,
  encoder: &mut wgpu::CommandEncoder,
  texture: &wgpu::Texture,
  width: u32,
  height: u32,
) -> wgpu::Buffer {
  let align_width = align(width, wgpu::COPY_BYTES_PER_ROW_ALIGNMENT / 4, true);
  let data_size = align_width as u64 * height as u64 * 4u64;

  // The output buffer lets us retrieve the data as an array
  let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
    size: data_size,
    usage: wgpu::BufferUsages::COPY_DST
      | wgpu::BufferUsages::MAP_READ
      | wgpu::BufferUsages::STORAGE,
    mapped_at_creation: false,
    label: None,
  });

  let buffer_bytes_per_row = 4 * align_width;
  encoder.copy_texture_to_buffer(
    texture.as_image_copy(),
    wgpu::ImageCopyBuffer {
      buffer: &output_buffer,
      layout: wgpu::ImageDataLayout {
        offset: 0,
        bytes_per_row: NonZeroU32::new(buffer_bytes_per_row),
        rows_per_image: NonZeroU32::new(0),
      },
    },
    wgpu::Extent3d {
      width,
      height,
      depth_or_array_layers: 1,
    },
  );

  output_buffer
}

impl WindowSurface {
  pub(crate) fn new(surface: wgpu::Surface, device: &wgpu::Device, size: DeviceSize) -> Self {
    let s_config = wgpu::SurfaceConfiguration {
      usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
      format: wgpu::TextureFormat::Bgra8UnormSrgb,
      width: size.width,
      height: size.height,
      present_mode: wgpu::PresentMode::Fifo,
    };

    surface.configure(device, &s_config);

    Self {
      surface,
      s_config,
      current_texture: RefCell::new(None),
    }
  }
}

/// A `Surface` present in a texture. Usually `PhysicSurface` display things to
/// screen(window eg.), But `TextureSurface` is soft, may not display in any
/// device, bug only in memory.
pub struct TextureSurface {
  pub(crate) raw_texture: wgpu::Texture,
  size: DeviceSize,
  usage: wgpu::TextureUsages,
}

impl TextureSurface {
  const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
  pub(crate) fn new(device: &wgpu::Device, size: DeviceSize, usage: wgpu::TextureUsages) -> Self {
    let raw_texture = Self::new_texture(device, size, usage);
    TextureSurface { raw_texture, size, usage }
  }

  fn new_texture(
    device: &wgpu::Device,
    size: DeviceSize,
    usage: wgpu::TextureUsages,
  ) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
      label: Some("new texture"),
      size: wgpu::Extent3d {
        width: size.width,
        height: size.height,
        depth_or_array_layers: 1,
      },
      dimension: wgpu::TextureDimension::D2,
      format: TextureSurface::FORMAT,
      usage,
      mip_level_count: 1,
      sample_count: 1,
    })
  }
}
