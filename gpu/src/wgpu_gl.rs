use crate::{
  tessellator::Tessellator, ColorRenderData, GlRender, GpuBackend, RenderData, TextureRenderData,
  Vertex,
};
use futures::executor::block_on;
use painter::DeviceSize;
use std::{iter, num::NonZeroU32};
use text::shaper::TextShaper;
mod color_pass;
pub mod surface;

use surface::{Surface, TextureSurface, WindowSurface};
use wgpu::util::DeviceExt;

use zerocopy::AsBytes;
mod img_pass;
use self::{color_pass::ColorPass, img_pass::ImagePass};

const TEXTURE_INIT_SIZE: DeviceSize = DeviceSize::new(1024, 1024);
const TEXTURE_MAX_SIZE: DeviceSize = DeviceSize::new(4096, 4096);

/// create wgpu backend with window
pub async fn wgpu_backend_with_wnd<W: raw_window_handle::HasRawWindowHandle>(
  window: &W,
  size: DeviceSize,
  tex_init_size: Option<DeviceSize>,
  tex_max_size: Option<DeviceSize>,
  tolerance: f32,
  shaper: TextShaper,
) -> GpuBackend<WgpuGl> {
  let init_size = tex_init_size.unwrap_or(TEXTURE_INIT_SIZE);
  let max_size = tex_max_size.unwrap_or(TEXTURE_MAX_SIZE);
  let tessellator = Tessellator::new(init_size, max_size, tolerance, shaper);
  let gl = WgpuGl::from_wnd(window, size, AntiAliasing::Msaa4X).await;

  GpuBackend { tessellator, gl }
}

/// create wgpu backend windowless
pub async fn wgpu_backend_headless(
  size: DeviceSize,
  tex_init_size: Option<DeviceSize>,
  tex_max_size: Option<DeviceSize>,
  tolerance: f32,
  shaper: TextShaper,
) -> GpuBackend<WgpuGl<surface::TextureSurface>> {
  let init_size = tex_init_size.unwrap_or(TEXTURE_INIT_SIZE);
  let max_size = tex_max_size.unwrap_or(TEXTURE_MAX_SIZE);
  let tessellator = Tessellator::new(init_size, max_size, tolerance, shaper);
  let gl = WgpuGl::headless(size).await;
  GpuBackend { tessellator, gl }
}

#[derive(Clone, Copy, PartialEq)]
pub enum AntiAliasing {
  None = 1,
  Msaa2X = 2,
  Msaa4X = 4,
  Msaa8X = 8,
  Msaa16X = 16,
}

pub struct WgpuGl<S: Surface = WindowSurface> {
  device: wgpu::Device,
  queue: wgpu::Queue,
  surface: S,
  color_pass: ColorPass,
  img_pass: ImagePass,
  coordinate_matrix: wgpu::Buffer,
  // todo: remove it, should get from the surface.
  s_config: wgpu::SurfaceConfiguration,
  primitives_layout: wgpu::BindGroupLayout,
  encoder: Option<wgpu::CommandEncoder>,
}

impl WgpuGl<WindowSurface> {
  /// Create a canvas and bind to a native window, its size is `width` and
  /// `height`. If you want to create a headless window, use
  /// [`headless_render`](WgpuRender::headless_render).
  pub async fn from_wnd<W: raw_window_handle::HasRawWindowHandle>(
    window: &W,
    size: DeviceSize,
    anti_aliasing: AntiAliasing,
  ) -> Self {
    let instance = wgpu::Instance::new(wgpu::Backends::PRIMARY);

    let w_surface = unsafe { instance.create_surface(window) };

    let adapter = instance
      .request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::default(),
        compatible_surface: Some(&w_surface),
        force_fallback_adapter: false,
      })
      .await
      .unwrap();

    Self::new(
      size,
      &adapter,
      |device| WindowSurface::new(w_surface, &adapter, device, size),
      anti_aliasing,
    )
    .await
  }
}

impl WgpuGl<TextureSurface> {
  /// Create a headless wgpu render, if you want to bind to a window, use
  /// [`wnd_render`](WgpuRender::wnd_render).
  pub async fn headless(size: DeviceSize) -> Self {
    let instance = wgpu::Instance::new(wgpu::Backends::PRIMARY);

    let adapter = instance
      .request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::default(),
        compatible_surface: None,
        force_fallback_adapter: false,
      })
      .await
      .unwrap();

    WgpuGl::new(
      size,
      &adapter,
      |device| {
        TextureSurface::new(
          device,
          size,
          wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        )
      },
      AntiAliasing::None,
    )
    .await
  }
}

impl<S: Surface> GlRender for WgpuGl<S> {
  fn submit_render_data(&mut self, data: RenderData) {
    if self.encoder.is_none() {
      let encoder = self
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Create Encoder") });
      self.encoder = Some(encoder);
    }

    match data {
      RenderData::Color(data) => self.render_color(data),
      RenderData::Image(data) => self.render_image(data),
    }
  }

  fn resize(&mut self, size: DeviceSize) {
    self.s_config.width = size.width;
    self.s_config.height = size.height;
    self
      .surface
      .update(&self.device, &self.queue, &self.s_config);
    self.coordinate_matrix = coordinate_matrix_buffer_2d(&self.device, size.width, size.height);
    self
      .color_pass
      .resize(&self.s_config, &self.coordinate_matrix, &self.device)
  }

  fn finish(&mut self) {
    if let Some(encoder) = self.encoder.take() {
      self.queue.submit(iter::once(encoder.finish()));
      self.surface.present();
      self.img_pass.end_frame();
    }
  }

  fn capture<'a>(
    &self,
    f: Box<dyn for<'r> FnOnce(DeviceSize, Box<dyn Iterator<Item = &[u8]> + 'r>) + 'a>,
  ) -> Result<(), &str> {
    let Self { surface, device, queue, .. } = self;
    let t = surface.current_texture();

    let size = surface.view_size();
    let DeviceSize { width, height, .. } = size;
    // fixme: crash if the surface is screen and it's maybe not rgba format
    // todo: pixel bytes from format
    const PX_BYTES: u32 = 4;
    // align to 256 bytes by WebGPU require.
    const WGPU_ALIGN: u32 = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    const ALIGN_BYTES: u32 = WGPU_ALIGN / PX_BYTES;
    let align_width = {
      match width % ALIGN_BYTES {
        0 => width,
        other => width - other + ALIGN_BYTES,
      }
    };

    let data_size = align_width as u64 * height as u64 * PX_BYTES as u64;

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
      label: Some("Encoder for encoding texture as png"),
    });

    // The output buffer lets us retrieve the data as an array
    let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
      size: data_size,
      usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
      mapped_at_creation: false,
      label: None,
    });

    let buffer_bytes_per_row = PX_BYTES * align_width;
    encoder.copy_texture_to_buffer(
      t.as_image_copy(),
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

    queue.submit(Some(encoder.finish()));

    let buffer_slice = output_buffer.slice(..);
    let buffer_future = buffer_slice.map_async(wgpu::MapMode::Read);

    // Poll the device in a blocking manner so that our future resolves.
    device.poll(wgpu::Maintain::Wait);

    block_on(buffer_future).map_err(|_| "Async buffer error")?;

    let slice = buffer_slice.get_mapped_range();
    let img_bytes_pre_row = (size.width * PX_BYTES) as usize;
    let rows = (0..size.height).map(|i| {
      let offset = (i * buffer_bytes_per_row) as usize;
      &slice.as_ref()[offset..offset + img_bytes_pre_row]
    });

    f(size, Box::new(rows));
    Ok(())
  }
}

impl<S: Surface> WgpuGl<S> {
  async fn new<C>(
    size: DeviceSize,
    adapter: &wgpu::Adapter,
    surface_ctor: C,
    anti_aliasing: AntiAliasing,
  ) -> WgpuGl<S>
  where
    C: FnOnce(&wgpu::Device) -> S,
  {
    let (device, queue) = adapter
      .request_device(
        &wgpu::DeviceDescriptor {
          label: Some("Request device"),
          features: wgpu::Features::empty(),
          limits: Default::default(),
        },
        None,
      )
      .await
      .unwrap();

    let surface = surface_ctor(&device);

    let s_config = wgpu::SurfaceConfiguration {
      usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
      format: wgpu::TextureFormat::Rgba8UnormSrgb,
      width: size.width,
      height: size.height,
      present_mode: wgpu::PresentMode::Fifo,
    };

    let primitive_layout = primitives_layout(&device);
    let coordinate_matrix = coordinate_matrix_buffer_2d(&device, size.width, size.height);

    let color_pass = ColorPass::new(
      &device,
      &s_config,
      &coordinate_matrix,
      &primitive_layout,
      anti_aliasing,
    );
    let texture_pass = ImagePass::new(&device, s_config.format);

    WgpuGl {
      device,
      surface,
      queue,
      color_pass,
      img_pass: texture_pass,
      coordinate_matrix,
      primitives_layout: primitive_layout,
      encoder: None,

      s_config,
    }
  }

  #[inline]
  pub fn set_anti_aliasing(&mut self, anti_aliasing: AntiAliasing) {
    self.color_pass.set_anti_aliasing(
      anti_aliasing,
      &self.s_config,
      &self.primitives_layout,
      &self.device,
    );
  }

  pub fn render_color(&mut self, data: ColorRenderData) {
    let prim_bind_group = self.create_primitives_bind_group(&data.primitives);
    let Self { device, surface, .. } = self;

    // todo: we can reuse the buffer
    let vertices_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
      label: Some("Vertices buffer"),
      contents: data.vertices.as_bytes(),
      usage: wgpu::BufferUsages::VERTEX,
    });
    let indices_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
      contents: data.indices.as_bytes(),
      usage: wgpu::BufferUsages::INDEX,
      label: Some("Indices buffer"),
    });

    let view = surface
      .current_texture()
      .create_view(&wgpu::TextureViewDescriptor::default());
    {
      let color_pass = &self.color_pass;
      let rpass_color_attachment = color_pass.color_attachments(&view);

      let mut render_pass =
        self
          .encoder
          .as_mut()
          .unwrap()
          .begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Color geometry render pass"),
            color_attachments: &[rpass_color_attachment],
            depth_stencil_attachment: None,
          });
      render_pass.set_pipeline(&color_pass.pipeline);
      render_pass.set_vertex_buffer(0, vertices_buffer.slice(..));
      render_pass.set_index_buffer(indices_buffer.slice(..), wgpu::IndexFormat::Uint32);
      render_pass.set_bind_group(0, &color_pass.uniform, &[]);
      render_pass.set_bind_group(1, &prim_bind_group, &[]);

      render_pass.draw_indexed(0..data.indices.len() as u32, 0, 0..1);
    }
  }

  pub fn render_image(&mut self, data: TextureRenderData) {
    let prim_bind_group = self.create_primitives_bind_group(&data.primitives);
    let Self { device, surface, queue, .. } = self;

    let vertices_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
      label: Some("Vertices buffer"),
      contents: data.vertices.as_bytes(),
      usage: wgpu::BufferUsages::VERTEX,
    });
    let indices_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
      contents: data.indices.as_bytes(),
      usage: wgpu::BufferUsages::INDEX,
      label: Some("Indices buffer"),
    });

    let view = surface
      .current_texture()
      .create_view(&wgpu::TextureViewDescriptor::default());
    {
      let uniform =
        self
          .img_pass
          .create_texture_uniform(device, data.texture, &self.coordinate_matrix, queue);

      let mut render_pass =
        self
          .encoder
          .as_mut()
          .unwrap()
          .begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Image geometry render pass"),
            color_attachments: &[wgpu::RenderPassColorAttachment {
              view: &view,
              resolve_target: None,
              ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                store: true,
              },
            }],
            depth_stencil_attachment: None,
          });
      render_pass.set_pipeline(&self.img_pass.pipeline);
      render_pass.set_vertex_buffer(0, vertices_buffer.slice(..));
      render_pass.set_index_buffer(indices_buffer.slice(..), wgpu::IndexFormat::Uint32);

      render_pass.set_bind_group(0, &uniform, &[]);
      render_pass.set_bind_group(1, &prim_bind_group, &[]);

      render_pass.draw_indexed(0..data.indices.len() as u32, 0, 0..1);
    }
  }

  fn create_primitives_bind_group<T: AsBytes>(&mut self, primitives: &[T]) -> wgpu::BindGroup {
    let primitives_buffer = self
      .device
      .create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Primitive Buffer"),
        contents: primitives.as_bytes(),
        usage: wgpu::BufferUsages::STORAGE,
      });
    self.device.create_bind_group(&wgpu::BindGroupDescriptor {
      layout: &self.primitives_layout,
      entries: &[wgpu::BindGroupEntry {
        binding: 0,
        resource: wgpu::BindingResource::Buffer(primitives_buffer.as_entire_buffer_binding()),
      }],
      label: Some("Primitive buffer bind group"),
    })
  }
}

fn primitives_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
  device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
    label: Some("Primitive layout (maybe changed every draw)"),
  })
}

fn coordinate_matrix_buffer_2d(device: &wgpu::Device, width: u32, height: u32) -> wgpu::Buffer {
  device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
    contents: [
      [2. / width as f32, 0., 0., 0.],
      [0., -2. / height as f32, 0., 0.],
      [0., 0., 1., 0.],
      [-1., 1., 0., 1.],
    ]
    .as_bytes(),
    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    label: Some("2d coordinate transform buffer."),
  })
}

impl Vertex {
  fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
    use std::mem::size_of;
    wgpu::VertexBufferLayout {
      array_stride: size_of::<Vertex>() as wgpu::BufferAddress,
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
    }
  }
}
