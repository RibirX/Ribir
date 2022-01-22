use crate::{
  tessellator::Tessellator, ColorPrimitive, ColorRenderData, GlRender, GpuBackend, RenderData,
  Texture, TexturePrimitive, TextureRenderData, Vertex,
};
use painter::{DeviceRect, DeviceSize, LogicUnit, PhysicUnit};
use std::num::NonZeroU32;
use text::shaper::TextShaper;
mod img_helper;
pub(crate) use img_helper::{bgra_texture_to_png, RgbaConvert};
pub mod surface;
use std::borrow::Borrow;
use surface::{PhysicSurface, Surface, TextureSurface};

use text::shaper::TextShaper;
use wgpu::{util::DeviceExt};
use zerocopy::AsBytes;

type Transform2D = painter::Transform2D<f32, LogicUnit, PhysicUnit>;
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
  let gl = WgpuGl::from_wnd(
    window,
    size,
    tessellator.atlas.texture().size(),
    AntiAliasing::Msaa4X,
  )
  .await;

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
  let gl = WgpuGl::headless(size, tessellator.atlas.texture().size()).await;
  GpuBackend { tessellator, gl }
}

enum PrimaryBindings {
  GlobalUniform = 0,
  TextureAtlas = 1,
  Sampler = 3,
}

enum SecondBindings {
  Primitive = 0,
}

#[derive(Clone, Copy, PartialEq)]
pub enum AntiAliasing {
  None = 1,
  Msaa2X = 2,
  Msaa4X = 4,
  Msaa8X = 8,
  Msaa16X = 16,
}

pub struct WgpuGl<S: Surface = PhysicSurface> {
  device: wgpu::Device,
  queue: wgpu::Queue,
  surface: S,
  s_config: wgpu::SurfaceConfiguration,
  pipeline: wgpu::RenderPipeline,
  primitives_layout: wgpu::BindGroupLayout,
  uniform_layout: wgpu::BindGroupLayout,
  uniforms: wgpu::BindGroup,
  rgba_converter: Option<RgbaConvert>,
  sampler: wgpu::Sampler,
  atlas: wgpu::Texture,
  rebuild_pipeline: bool,
  anti_aliasing: AntiAliasing,
  multisampled_framebuffer: wgpu::TextureView,
}

impl WgpuGl<PhysicSurface> {
  /// Create a canvas and bind to a native window, its size is `width` and
  /// `height`. If you want to create a headless window, use
  /// [`headless_render`](WgpuRender::headless_render).
  pub async fn from_wnd<W: raw_window_handle::HasRawWindowHandle>(
    window: &W,
    size: DeviceSize,
    atlas_texture_size: DeviceSize,
    anti_aliasing: AntiAliasing,
  ) -> Self {
    let instance = wgpu::Instance::new(wgpu::Backends::PRIMARY);

    let w_surface = unsafe { instance.create_surface(window) };

    let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
      power_preference: wgpu::PowerPreference::default(),
      compatible_surface: Some(&w_surface),
      force_fallback_adapter: false,
    });

    Self::new(
      size,
      atlas_texture_size,
      adapter,
      move |device| PhysicSurface::new(w_surface, device, size),
      anti_aliasing,
    )
    .await
  }
}

impl WgpuGl<TextureSurface> {
  /// Create a headless wgpu render, if you want to bind to a window, use
  /// [`wnd_render`](WgpuRender::wnd_render).
  pub async fn headless(size: DeviceSize, atlas_texture_size: DeviceSize) -> Self {
    let instance = wgpu::Instance::new(wgpu::Backends::PRIMARY);

    let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
      power_preference: wgpu::PowerPreference::default(),
      compatible_surface: None,
      force_fallback_adapter: false,
    });

    WgpuGl::new(
      size,
      atlas_texture_size,
      adapter,
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

  // todo: remove png dependency.
  /// PNG encoded the canvas then write by `writer`.
  pub async fn write_png<W: std::io::Write>(&mut self, writer: W) -> Result<(), &'static str> {
    self.ensure_rgba_converter();
    let rect = DeviceRect::from_size(self.surface.size());

    let Self {
      surface,
      device,
      queue,
      rgba_converter,
      ..
    } = self;
    bgra_texture_to_png(
      &surface.raw_texture,
      rect,
      device,
      queue,
      rgba_converter.as_ref().unwrap(),
      writer,
    )
    .await
  }
}

impl<S: Surface> GlRender for WgpuGl<S> {
  fn submit_render_data(&mut self, data: RenderData) {
    let encoder = self
      .device
      .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Render Encoder") });

    match data {
      RenderData::Color(data) => self.render_color(encoder, data),
      RenderData::Image(data) => self.render_image(encoder, data),
    }
  }

  fn resize(&mut self, size: DeviceSize) {
    self.s_config.width = size.width;
    self.s_config.height = size.height;
    self
      .surface
      .update(&self.device, &self.queue, &self.s_config);
    self.rebuild_pipeline = true;
  }

  fn finish(&mut self) {
    // todo: immediate draw now when render data submitted.
  }
}

impl<S: Surface> WgpuGl<S> {
  async fn new<C>(
    size: DeviceSize,
    atlas_texture_size: DeviceSize,
    adapter: impl std::future::Future<Output = Option<wgpu::Adapter>> + Send,
    surface_ctor: C,
    anti_aliasing: AntiAliasing,
  ) -> WgpuGl<S>
  where
    C: FnOnce(&wgpu::Device) -> S,
  {
    let (device, queue) = adapter
      .await
      .unwrap()
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
      format: wgpu::TextureFormat::Bgra8UnormSrgb,
      width: size.width,
      height: size.height,
      present_mode: wgpu::PresentMode::Fifo,
    };

    let [uniform_layout, primitives_layout] = create_uniform_layout(&device);
    let pipeline = create_render_pipeline(
      &device,
      &s_config,
      &[&uniform_layout, &primitives_layout],
      anti_aliasing as u32,
    );

    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
      address_mode_u: wgpu::AddressMode::ClampToEdge,
      address_mode_v: wgpu::AddressMode::ClampToEdge,
      address_mode_w: wgpu::AddressMode::ClampToEdge,
      mag_filter: wgpu::FilterMode::Nearest,
      min_filter: wgpu::FilterMode::Nearest,
      mipmap_filter: wgpu::FilterMode::Nearest,
      lod_min_clamp: 0.0,
      lod_max_clamp: 0.0,
      label: Some("Texture atlas sampler"),
      ..Default::default()
    });

    let texture_atlas = Self::create_wgpu_texture(
      &device,
      atlas_texture_size,
      wgpu::TextureFormat::Bgra8UnormSrgb,
    );
    let uniforms = create_uniforms(
      &device,
      &uniform_layout,
      atlas_texture_size,
      &coordinate_2d_to_device_matrix(size.width, size.height),
      &sampler,
      &texture_atlas.create_view(&wgpu::TextureViewDescriptor::default()),
    );

    let multisampled_framebuffer =
      Self::create_multisampled_framebuffer(&device, &s_config, anti_aliasing as u32);

    WgpuGl {
      sampler,
      device,
      surface,
      queue,
      pipeline,
      uniform_layout,
      primitives_layout,
      uniforms,
      atlas: texture_atlas,
      rgba_converter: None,
      rebuild_pipeline: false,
      anti_aliasing,
      multisampled_framebuffer,
      s_config,
    }
  }

  pub fn set_anti_aliasing(&mut self, anti_aliasing: AntiAliasing) {
    self.anti_aliasing = anti_aliasing;
    self.rebuild_pipeline = true;
  }

  pub(crate) fn ensure_rgba_converter(&mut self) {
    if self.rgba_converter.is_none() {
      self.rgba_converter = Some(RgbaConvert::new(&self.device));
    }
  }

  pub fn render_color(&mut self, mut encoder: wgpu::CommandEncoder, data: ColorRenderData) {
    let tex_infos_bind_group = self.create_primitives_bind_group(&data.primitives);
    let Self { device, surface, queue, .. } = self;

    let view = surface.get_current_view();

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

    {
      let rpass_color_attachment = if self.anti_aliasing == AntiAliasing::None {
        wgpu::RenderPassColorAttachment {
          view: &view.borrow(),
          resolve_target: None,
          ops: wgpu::Operations {
            load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
            store: true,
          },
        }
      } else {
        wgpu::RenderPassColorAttachment {
          view: &self.multisampled_framebuffer,
          resolve_target: Some(&view.borrow()),
          ops: wgpu::Operations {
            load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
            store: true,
          },
        }
      };

      let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("Render Pass"),
        color_attachments: &[rpass_color_attachment],
        depth_stencil_attachment: None,
      });
      render_pass.set_pipeline(&self.pipeline);
      render_pass.set_vertex_buffer(0, vertices_buffer.slice(..));
      render_pass.set_index_buffer(indices_buffer.slice(..), wgpu::IndexFormat::Uint32);
      render_pass.set_bind_group(0, &self.uniforms, &[]);
      render_pass.set_bind_group(1, &tex_infos_bind_group, &[]);

      render_pass.draw_indexed(0..data.indices.len() as u32, 0, 0..1);
    }

    queue.submit(iter::once(encoder.finish()));
  }

  pub fn render_image(&mut self, mut encoder: wgpu::CommandEncoder, data: TextureRenderData) {
    let tex_infos_bind_group = self.create_primitives_bind_group(&data.primitives);
    let Self {
      device,
      atlas,
      surface,
      queue,
      uniform_layout,
      s_config,
      ..
    } = self;

    type Tf = wgpu::TextureFormat;
    Self::sync_texture(
      device,
      atlas,
      &data.texture,
      Tf::Bgra8UnormSrgb,
      &mut encoder,
    );

    if self.rebuild_pipeline {
      self.uniforms = create_uniforms(
        device,
        uniform_layout,
        DeviceSize::new(data.texture.size.width, data.texture.size.height),
        &coordinate_2d_to_device_matrix(s_config.width, s_config.height),
        &self.sampler,
        &atlas.create_view(&wgpu::TextureViewDescriptor::default()),
      )
    }
    if self.rebuild_pipeline {
      let sample_count = self.anti_aliasing as u32;
      self.multisampled_framebuffer =
        Self::create_multisampled_framebuffer(device, s_config, sample_count);
      let [uniform_layout, primitives_layout] = create_uniform_layout(device);
      self.pipeline = create_render_pipeline(
        device,
        s_config,
        &[&uniform_layout, &primitives_layout],
        sample_count,
      );
      self.rebuild_pipeline = false;
    }

    let view = surface.get_current_view();

    let vertices_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
      label: Some("Vertices buffer"),
      contents: &data.vertices.as_bytes(),
      usage: wgpu::BufferUsages::VERTEX,
    });
    let indices_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
      contents: &data.indices.as_bytes(),
      usage: wgpu::BufferUsages::INDEX,
      label: Some("Indices buffer"),
    });

    {
      let rpass_color_attachment = if self.anti_aliasing == AntiAliasing::None {
        wgpu::RenderPassColorAttachment {
          view: &view.borrow(),
          resolve_target: None,
          ops: wgpu::Operations {
            load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
            store: true,
          },
        }
      } else {
        wgpu::RenderPassColorAttachment {
          view: &self.multisampled_framebuffer,
          resolve_target: Some(&view.borrow()),
          ops: wgpu::Operations {
            load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
            store: true,
          },
        }
      };

      let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("Render Pass"),
        color_attachments: &[rpass_color_attachment],
        depth_stencil_attachment: None,
      });
      render_pass.set_pipeline(&self.pipeline);
      render_pass.set_vertex_buffer(0, vertices_buffer.slice(..));
      render_pass.set_index_buffer(indices_buffer.slice(..), wgpu::IndexFormat::Uint32);
      render_pass.set_bind_group(0, &self.uniforms, &[]);
      render_pass.set_bind_group(1, &tex_infos_bind_group, &[]);

      render_pass.draw_indexed(0..data.indices.len() as u32, 0, 0..1);
    }

    queue.submit(iter::once(encoder.finish()));
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
        binding: SecondBindings::Primitive as u32,
        resource: wgpu::BindingResource::Buffer(primitives_buffer.as_entire_buffer_binding()),
      }],
      label: Some("texture infos bind group"),
    })
  }

  fn sync_texture(
    device: &wgpu::Device,
    wgpu_tex: &mut wgpu::Texture,
    texture: &Texture,
    format: wgpu::TextureFormat,
    encoder: &mut wgpu::CommandEncoder,
  ) {
    if let Some(data) = texture.data {
      *wgpu_tex = Self::create_wgpu_texture(device, texture.size, format);
      let DeviceSize { width, height, .. } = texture.size;
      let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Memory texture submit to wgpu render."),
        contents: data,
        usage: wgpu::BufferUsages::COPY_SRC,
      });

      encoder.copy_buffer_to_texture(
        wgpu::ImageCopyBuffer {
          buffer: &buffer,
          layout: wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: NonZeroU32::new(width * 4),
            rows_per_image: NonZeroU32::new(height),
          },
        },
        wgpu::ImageCopyTexture {
          texture: wgpu_tex,
          mip_level: 0,
          origin: wgpu::Origin3d::ZERO,
          aspect: wgpu::TextureAspect::All,
        },
        wgpu::Extent3d {
          width,
          height,
          depth_or_array_layers: 1,
        },
      )
    }
  }

  fn create_wgpu_texture(
    device: &wgpu::Device,
    size: DeviceSize,
    format: wgpu::TextureFormat,
  ) -> wgpu::Texture {
    let texture_descriptor = &wgpu::TextureDescriptor {
      label: Some("Create texture for memory texture"),
      size: wgpu::Extent3d {
        width: size.width,
        height: size.height,
        depth_or_array_layers: 1,
      },
      dimension: wgpu::TextureDimension::D2,
      format,
      usage: wgpu::TextureUsages::COPY_DST
        | wgpu::TextureUsages::TEXTURE_BINDING
        | wgpu::TextureUsages::COPY_SRC,
      mip_level_count: 1,
      sample_count: 1,
    };
    device.create_texture(texture_descriptor)
  }

  fn create_multisampled_framebuffer(
    device: &wgpu::Device,
    s_config: &wgpu::SurfaceConfiguration,
    sample_count: u32,
  ) -> wgpu::TextureView {
    let multisampled_texture_extent = wgpu::Extent3d {
      width: s_config.width,
      height: s_config.height,
      depth_or_array_layers: 1,
    };
    let multisampled_frame_descriptor = &wgpu::TextureDescriptor {
      size: multisampled_texture_extent,
      mip_level_count: 1,
      sample_count,
      dimension: wgpu::TextureDimension::D2,
      format: s_config.format,
      usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
      label: None,
    };

    device
      .create_texture(multisampled_frame_descriptor)
      .create_view(&wgpu::TextureViewDescriptor::default())
  }
}

fn create_render_pipeline(
  device: &wgpu::Device,
  s_config: &wgpu::SurfaceConfiguration,
  bind_group_layouts: &[&wgpu::BindGroupLayout; 2],
  count: u32,
) -> wgpu::RenderPipeline {
  let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
    label: Some("Create render pipeline"),
    bind_group_layouts,
    push_constant_ranges: &[],
  });

  let module = device.create_shader_module(&wgpu::include_wgsl!("./wgpu_gl/shaders/geometry.wgsl"));
  device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
    label: Some("Render Pipeline"),
    layout: Some(&render_pipeline_layout),
    vertex: wgpu::VertexState {
      module: &module,
      entry_point: "vs_main",
      buffers: &[Vertex::desc()],
    },
    fragment: Some(wgpu::FragmentState {
      module: &module,
      entry_point: "fs_main",
      targets: &[wgpu::ColorTargetState {
        format: s_config.format,
        blend: Some(wgpu::BlendState::REPLACE),
        write_mask: wgpu::ColorWrites::all(),
      }],
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
      count,
      mask: !0,
      alpha_to_coverage_enabled: false,
    },
    multiview: None,
  })
}

fn create_uniform_layout(device: &wgpu::Device) -> [wgpu::BindGroupLayout; 2] {
  let stable = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
    entries: &[
      wgpu::BindGroupLayoutEntry {
        binding: PrimaryBindings::GlobalUniform as u32,
        visibility: wgpu::ShaderStages::VERTEX,
        ty: wgpu::BindingType::Buffer {
          ty: wgpu::BufferBindingType::Uniform,
          has_dynamic_offset: false,
          min_binding_size: None,
        },
        count: None,
      },
      wgpu::BindGroupLayoutEntry {
        binding: PrimaryBindings::TextureAtlas as u32,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
          sample_type: wgpu::TextureSampleType::Float { filterable: true },
          view_dimension: wgpu::TextureViewDimension::D2,
          multisampled: false,
        },
        count: None,
      },
      wgpu::BindGroupLayoutEntry {
        binding: PrimaryBindings::Sampler as u32,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        count: None,
      },
    ],
    label: Some("uniforms stable layout"),
  });

  let dynamic = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
    entries: &[wgpu::BindGroupLayoutEntry {
      binding: SecondBindings::Primitive as u32,
      visibility: wgpu::ShaderStages::VERTEX,
      ty: wgpu::BindingType::Buffer {
        ty: wgpu::BufferBindingType::Storage { read_only: true },
        has_dynamic_offset: false,
        min_binding_size: None,
      },
      count: None,
    }],
    label: Some("uniform layout for texture infos (changed every draw)"),
  });
  [stable, dynamic]
}

/// Convert coordinate system from canvas 2d into wgpu.
pub fn coordinate_2d_to_device_matrix(width: u32, height: u32) -> Transform2D {
  Transform2D::new(2. / width as f32, 0., 0., -2. / height as f32, -1., 1.)
}

fn create_uniforms(
  device: &wgpu::Device,
  layout: &wgpu::BindGroupLayout,
  atlas_size: DeviceSize,
  canvas_2d_to_device_matrix: &Transform2D,
  sampler: &wgpu::Sampler,
  tex_atlas: &wgpu::TextureView,
) -> wgpu::BindGroup {
  let uniform = GlobalUniform {
    texture_atlas_size: atlas_size.to_array(),
    canvas_coordinate_map: canvas_2d_to_device_matrix.to_arrays(),
  };
  let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
    contents: uniform.as_bytes(),
    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    label: Some("uniform buffer"),
  });
  device.create_bind_group(&wgpu::BindGroupDescriptor {
    layout,
    entries: &[
      wgpu::BindGroupEntry {
        binding: PrimaryBindings::GlobalUniform as u32,
        resource: wgpu::BindingResource::Buffer(uniform_buffer.as_entire_buffer_binding()),
      },
      wgpu::BindGroupEntry {
        binding: PrimaryBindings::TextureAtlas as u32,
        resource: wgpu::BindingResource::TextureView(tex_atlas),
      },
      wgpu::BindGroupEntry {
        binding: PrimaryBindings::Sampler as u32,
        resource: wgpu::BindingResource::Sampler(sampler),
      },
    ],
    label: Some("uniform_bind_group"),
  })
}

#[repr(C)]
#[derive(Copy, Clone, AsBytes)]
struct GlobalUniform {
  canvas_coordinate_map: [[f32; 2]; 3],
  texture_atlas_size: [u32; 2],
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
          offset: (size_of::<[f32; 2]>() * 2) as wgpu::BufferAddress,
          shader_location: 1,
          format: wgpu::VertexFormat::Uint32,
        },
      ],
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::*;
  use futures::executor::block_on;
  use painter::Color;

  fn circle_50() -> Path {
    let mut path = PathBuilder::new();
    path.circle(euclid::Point2D::new(0., 0.), 50.);
    path.build()
  }

  #[test]
  fn coordinate_2d_start() {
    let matrix = coordinate_2d_to_device_matrix(400, 400);

    let lt = matrix.transform_point(Point::new(0., 0.));
    assert_eq!((lt.x, lt.y), (-1., 1.));

    let rt = matrix.transform_point(Point::new(400., 0.));
    assert_eq!((rt.x, rt.y), (1., 1.));

    let lb = matrix.transform_point(Point::new(0., 400.));
    assert_eq!((lb.x, lb.y), (-1., -1.));

    let rb = matrix.transform_point(Point::new(400., 400.));
    assert_eq!((rb.x, rb.y), (1., -1.0));
  }
}