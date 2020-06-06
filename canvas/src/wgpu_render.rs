mod img_helper;
pub(crate) use img_helper::{bgra_texture_to_png, RgbaConvert};
pub mod surface;
use surface::{PhysicSurface, Surface, TextureSurface};

pub struct WgpuRender<S: Surface = PhysicSurface> {
  pub(crate) device: wgpu::Device,
  pub(crate) queue: wgpu::Queue,
  pub(crate) surface: S,
  pub(crate) pipeline: wgpu::RenderPipeline,
  pub(crate) primitives_layout: wgpu::BindGroupLayout,
  pub(crate) uniform_layout: wgpu::BindGroupLayout,
  pub(crate) uniforms: wgpu::BindGroup,
  pub(crate) encoder: Option<wgpu::CommandEncoder>,
  pub(crate) view: Option<S::V>,
  pub(crate) rgba_converter: Option<RgbaConvert>,
  pub(crate) sampler: wgpu::Sampler,
  glyph_texture: wgpu::Texture,
  texture_atlas: wgpu::Texture,
}





impl Canvas<PhysicSurface> {
  /// Create a canvas and bind to a native window, its size is `width` and
  /// `height`. If you want to create a headless window, use
  /// [`from_window`](Canvas::window).
  pub async fn from_window<W: raw_window_handle::HasRawWindowHandle>(
    window: &W,
    size: DeviceSize,
  ) -> Self {
    let instance = wgpu::Instance::new();

    let w_surface = unsafe { instance.create_surface(window) };

    let adapter = instance.request_adapter(
      &wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::Default,
        compatible_surface: Some(&w_surface),
      },
      wgpu::BackendBit::PRIMARY,
    );

    Self::create_canvas(size, adapter, move |device| {
      PhysicSurface::new(w_surface, &device, size)
    })
    .await
  }
}

impl Canvas<TextureSurface> {
  /// Create a canvas which its size is `width` and `size`, if you want to bind
  /// to a window, use [`from_window`](Canvas::from_window).
  pub async fn new(size: DeviceSize) -> Self {
    let instance = wgpu::Instance::new();

    let adapter = instance.request_adapter(
      &wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::Default,
        compatible_surface: None,
      },
      wgpu::BackendBit::PRIMARY,
    );

    Canvas::create_canvas(size, adapter, |device| {
      TextureSurface::new(
        &device,
        size,
        wgpu::TextureUsage::OUTPUT_ATTACHMENT | wgpu::TextureUsage::COPY_SRC,
      )
    })
    .await
  }

  /// PNG encoded the canvas then write by `writer`.
  pub async fn to_png<W: std::io::Write>(&mut self, writer: W) -> Result<(), &'static str> {
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

impl<S: Surface> Canvas<S> {
  /// Resize canvas
  pub fn resize(&mut self, width: u32, height: u32) {
    self
      .surface
      .resize(&self.device, &self.queue, width, height);
    self.update_uniforms();
  }

  #[cfg(debug_assertions)]
  pub fn log_texture_atlas(&mut self) {
    unimplemented!();
    // self.ensure_rgba_converter();

    // let size = self.surface.size();
    // let Canvas {
    //   tex_atlas,
    //   device,
    //   queue,
    //   rgba_converter,
    //   ..
    // } = self;

    // let pkg_root = env!("CARGO_MANIFEST_DIR");
    // let atlas_capture = format!("{}/.log/{}", pkg_root, "texture_atlas.png");

    // let atlas = bgra_texture_to_png(
    //   &tex_atlas.texture.raw_texture,
    //   DeviceRect::from_size(size),
    //   device,
    //   queue,
    //   rgba_converter.as_ref().unwrap(),
    //   std::fs::File::create(&atlas_capture).unwrap(),
    // );

    // let _r = futures::executor::block_on(atlas);

    // log::debug!("Write a image of canvas atlas at: {}", &atlas_capture);
  }
}

impl<S: Surface> Canvas<S> {
  async fn create_canvas<C>(
    size: DeviceSize,
    adapter: impl std::future::Future<Output = Option<wgpu::Adapter>> + Send,
    surface_ctor: C,
  ) -> Canvas<S>
  where
    C: FnOnce(&wgpu::Device) -> S,
  {
    let (device, queue) = adapter
      .await
      .unwrap()
      .request_device(
        &wgpu::DeviceDescriptor {
          extensions: wgpu::Extensions::empty(),
          limits: Default::default(),
        },
        None,
      )
      .await
      .unwrap();

    let surface = surface_ctor(&device);

    let sc_desc = wgpu::SwapChainDescriptor {
      usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
      format: wgpu::TextureFormat::Bgra8UnormSrgb,
      width: size.width,
      height: size.height,
      present_mode: wgpu::PresentMode::Fifo,
    };

    let [uniform_layout, primitives_layout] = create_uniform_layout(&device);
    let pipeline =
      create_render_pipeline(&device, &sc_desc, &[&uniform_layout, &primitives_layout]);

    let tex_atlas = TextureAtlas::new();
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
      address_mode_u: wgpu::AddressMode::ClampToEdge,
      address_mode_v: wgpu::AddressMode::ClampToEdge,
      address_mode_w: wgpu::AddressMode::ClampToEdge,
      mag_filter: wgpu::FilterMode::Nearest,
      min_filter: wgpu::FilterMode::Nearest,
      mipmap_filter: wgpu::FilterMode::Nearest,
      lod_min_clamp: 0.0,
      lod_max_clamp: 0.0,
      compare: None,
      label: Some("Texture atlas sampler"),
      anisotropy_clamp: None,
    });

    let glyph_brush = TextBrush::new();
    let glyph_texture = Self::glyph_texture(&device, &glyph_brush);
    let texture_atlas = Self::atlas_texture(&device, &tex_atlas);
    let uniforms = create_uniforms(
      &device,
      &uniform_layout,
      *tex_atlas.texture().size(),
      &coordinate_2d_to_device_matrix(size.width, size.height),
      &sampler,
      &texture_atlas.create_default_view(),
      &glyph_texture.create_default_view(),
    );

    Canvas {
      glyph_brush,
      atlas: tex_atlas,
      sampler,
      device,
      surface,
      queue,
      pipeline,
      uniform_layout,
      primitives_layout,
      uniforms,
      render_data: RenderData::default(),
      glyph_texture,
      texture_atlas,
      rgba_converter: None,
      encoder: None,
      view: None,
    }
  }

  pub(crate) fn ensure_encoder_exist(&mut self) {
    if self.encoder.is_none() {
      self.encoder = Some(
        self
          .device
          .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
          }),
      )
    }
  }

  fn ensure_view_exist(&mut self) {
    if self.view.is_none() {
      self.view = Some(self.surface.get_next_view());
    }
  }

  pub(crate) fn ensure_rgba_converter(&mut self) {
    if self.rgba_converter.is_none() {
      self.rgba_converter = Some(RgbaConvert::new(&self.device));
    }
  }


  /// Submits a series of finished command buffers for execution. You needn't
  /// call this method manually, only if you want flush drawing things into gpu
  /// immediately.
  pub fn submit(&mut self) {
    if !self.render_data.has_data() {
      return;
    }

    self.ensure_encoder_exist();
    self.ensure_view_exist();

    if self.atlas.texture().is_updated() {
      self.update_uniforms();
    }

    let tex_infos_bind_group = self.create_primitives_bind_group();

    let Self {
      device,
      encoder,
      view,
      glyph_texture,
      texture_atlas,
      ..
    } = self;
    let encoder = encoder.as_mut().unwrap();
    let view = view.as_ref().unwrap().borrow();

    self.atlas.flush_cache(device, encoder, texture_atlas);
    self.glyph_brush.flush_cache(device, encoder, glyph_texture);

    let vertices_buffer = device.create_buffer_with_data(
      self.render_data.vertices.as_bytes(),
      wgpu::BufferUsage::VERTEX,
    );

    let indices_buffer = device.create_buffer_with_data(
      self.render_data.indices.as_bytes(),
      wgpu::BufferUsage::INDEX,
    );

    {
      let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
          attachment: view,
          resolve_target: None,
          load_op: wgpu::LoadOp::Clear,
          store_op: wgpu::StoreOp::Store,
          clear_color: wgpu::Color::WHITE,
        }],
        depth_stencil_attachment: None,
      });
      render_pass.set_pipeline(&self.pipeline);
      render_pass.set_vertex_buffer(0, vertices_buffer.slice(..));
      render_pass.set_index_buffer(indices_buffer.slice(..));
      render_pass.set_bind_group(0, &self.uniforms, &[]);
      render_pass.set_bind_group(1, &tex_infos_bind_group, &[]);

      render_pass.draw_indexed(0..self.render_data.indices.len() as u32, 0, 0..1);
    }

    self.render_data.clear();

    if let Some(encoder) = self.encoder.take() {
      self.queue.submit(Some(encoder.finish()));
    }
    self.view.take();
  }

  #[inline]
  pub(crate) fn update_uniforms(&mut self) {
    let size = self.surface.size();
    self.uniforms = create_uniforms(
      &self.device,
      &self.uniform_layout,
      *self.atlas.texture().size(),
      &coordinate_2d_to_device_matrix(size.width, size.height),
      &self.sampler,
      &self.texture_atlas.create_default_view(),
      &self.glyph_texture.create_default_view(),
    )
  }

  fn create_primitives_bind_group(&mut self) -> wgpu::BindGroup {
    let primitives = &self.render_data.primitives;
    let primitives_buffer = self
      .device
      .create_buffer_with_data(primitives.as_bytes(), wgpu::BufferUsage::STORAGE);
    self.device.create_bind_group(&wgpu::BindGroupDescriptor {
      layout: &self.primitives_layout,
      bindings: &[wgpu::Binding {
        binding: SecondBindings::Primitive as u32,
        resource: wgpu::BindingResource::Buffer(primitives_buffer.slice(..)),
      }],
      label: Some("texture infos bind group"),
    })
  }

  pub(crate) fn resize_glyph_texture(&mut self) {
    self.glyph_texture = Self::glyph_texture(&self.device, &self.glyph_brush);
  }

  fn glyph_texture(device: &wgpu::Device, brush: &TextBrush) -> wgpu::Texture {
    let size = brush.texture().size();
    device.create_texture(&wgpu::TextureDescriptor {
      label: Some("new glyph texture"),
      size: wgpu::Extent3d {
        width: size.width,
        height: size.height,
        depth: 1,
      },
      dimension: wgpu::TextureDimension::D2,
      format: wgpu::TextureFormat::R8Unorm,
      usage: wgpu::TextureUsage::COPY_DST
        | wgpu::TextureUsage::SAMPLED
        | wgpu::TextureUsage::COPY_SRC,
      mip_level_count: 1,
      sample_count: 1,
    })
  }

  fn atlas_texture(device: &wgpu::Device, atlas: &TextureAtlas) -> wgpu::Texture {
    let size = atlas.texture().size();
    device.create_texture(&wgpu::TextureDescriptor {
      label: Some("new glyph texture"),
      size: wgpu::Extent3d {
        width: size.width,
        height: size.height,
        depth: 1,
      },
      dimension: wgpu::TextureDimension::D2,
      format: wgpu::TextureFormat::Bgra8UnormSrgb,
      usage: wgpu::TextureUsage::COPY_DST
        | wgpu::TextureUsage::SAMPLED
        | wgpu::TextureUsage::COPY_SRC,
      mip_level_count: 1,
      sample_count: 1,
    })
  }
}

fn create_render_pipeline(
  device: &wgpu::Device,
  sc_desc: &wgpu::SwapChainDescriptor,
  bind_group_layouts: &[&wgpu::BindGroupLayout; 2],
) -> wgpu::RenderPipeline {
  let render_pipeline_layout =
    device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { bind_group_layouts });

  let vs_module = spv_2_shader_module!(device, "./shaders/geometry.vert.spv");
  let fs_module = spv_2_shader_module!(device, "./shaders/geometry.frag.spv");

  device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
    layout: &render_pipeline_layout,
    vertex_stage: wgpu::ProgrammableStageDescriptor {
      module: &vs_module,
      entry_point: "main",
    },
    fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
      module: &fs_module,
      entry_point: "main",
    }),
    rasterization_state: Some(wgpu::RasterizationStateDescriptor {
      front_face: wgpu::FrontFace::Ccw,
      cull_mode: wgpu::CullMode::None,
      depth_bias: 0,
      depth_bias_slope_scale: 0.0,
      depth_bias_clamp: 0.0,
    }),
    color_states: &[wgpu::ColorStateDescriptor {
      format: sc_desc.format,
      color_blend: wgpu::BlendDescriptor {
        src_factor: wgpu::BlendFactor::SrcAlpha,
        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
        operation: wgpu::BlendOperation::Add,
      },
      alpha_blend: wgpu::BlendDescriptor {
        src_factor: wgpu::BlendFactor::One,
        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
        operation: wgpu::BlendOperation::Add,
      },
      write_mask: wgpu::ColorWrite::ALL,
    }],
    primitive_topology: wgpu::PrimitiveTopology::TriangleList,
    depth_stencil_state: None,
    vertex_state: wgpu::VertexStateDescriptor {
      index_format: wgpu::IndexFormat::Uint32,
      vertex_buffers: &[Vertex::desc()],
    },
    sample_count: 1,
    sample_mask: !0,
    alpha_to_coverage_enabled: false,
  })
}

pub(crate) macro spv_2_shader_module($device: expr, $path: literal) {{
  let bytes = include_bytes!($path);
  let spv = wgpu::read_spirv(std::io::Cursor::new(&bytes[..])).unwrap();
  $device.create_shader_module(&spv)
}}

fn create_uniform_layout(device: &wgpu::Device) -> [wgpu::BindGroupLayout; 2] {
  let stable = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
    bindings: &[
      wgpu::BindGroupLayoutEntry {
        binding: PrimaryBindings::GlobalUniform as u32,
        visibility: wgpu::ShaderStage::VERTEX,
        ty: wgpu::BindingType::UniformBuffer { dynamic: false },
      },
      wgpu::BindGroupLayoutEntry {
        binding: PrimaryBindings::TextureAtlas as u32,
        visibility: wgpu::ShaderStage::FRAGMENT,
        ty: wgpu::BindingType::SampledTexture {
          dimension: wgpu::TextureViewDimension::D2,
          component_type: wgpu::TextureComponentType::Float,
          multisampled: false,
        },
      },
      wgpu::BindGroupLayoutEntry {
        binding: PrimaryBindings::Sampler as u32,
        visibility: wgpu::ShaderStage::FRAGMENT,
        ty: wgpu::BindingType::Sampler { comparison: false },
      },
      wgpu::BindGroupLayoutEntry {
        binding: PrimaryBindings::GlyphTexture as u32,
        visibility: wgpu::ShaderStage::FRAGMENT,
        ty: wgpu::BindingType::SampledTexture {
          dimension: wgpu::TextureViewDimension::D2,
          component_type: wgpu::TextureComponentType::Float,
          multisampled: false,
        },
      },
    ],
    label: Some("uniforms stable layout"),
  });

  let dynamic = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
    bindings: &[wgpu::BindGroupLayoutEntry {
      binding: SecondBindings::Primitive as u32,
      visibility: wgpu::ShaderStage::VERTEX,
      ty: wgpu::BindingType::StorageBuffer {
        dynamic: false,
        readonly: true,
      },
    }],
    label: Some("uniform layout for texture infos (changed every draw)"),
  });
  [stable, dynamic]
}

/// Convert coordinate system from canvas 2d into wgpu.
pub fn coordinate_2d_to_device_matrix(
  width: u32,
  height: u32,
) -> euclid::Transform2D<f32, LogicUnit, PhysicUnit> {
  euclid::Transform2D::row_major(2. / width as f32, 0., 0., -2. / height as f32, -1., 1.)
}

fn create_uniforms(
  device: &wgpu::Device,
  layout: &wgpu::BindGroupLayout,
  atlas_size: DeviceSize,
  canvas_2d_to_device_matrix: &euclid::Transform2D<f32, LogicUnit, PhysicUnit>,
  sampler: &wgpu::Sampler,
  tex_atlas: &wgpu::TextureView,
  glyph_texture: &wgpu::TextureView,
) -> wgpu::BindGroup {
  let uniform = GlobalUniform {
    texture_atlas_size: [atlas_size.width, atlas_size.height],
    canvas_coordinate_map: canvas_2d_to_device_matrix.to_row_arrays(),
  };
  let uniform_buffer = device.create_buffer_with_data(
    &uniform.as_bytes(),
    wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
  );
  device.create_bind_group(&wgpu::BindGroupDescriptor {
    layout,
    bindings: &[
      wgpu::Binding {
        binding: PrimaryBindings::GlobalUniform as u32,
        resource: wgpu::BindingResource::Buffer(uniform_buffer.slice(..)),
      },
      wgpu::Binding {
        binding: PrimaryBindings::TextureAtlas as u32,
        resource: wgpu::BindingResource::TextureView(tex_atlas),
      },
      wgpu::Binding {
        binding: PrimaryBindings::Sampler as u32,
        resource: wgpu::BindingResource::Sampler(sampler),
      },
      wgpu::Binding {
        binding: PrimaryBindings::GlyphTexture as u32,
        resource: wgpu::BindingResource::TextureView(glyph_texture),
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
  fn desc<'a>() -> wgpu::VertexBufferDescriptor<'a> {
    use std::mem::size_of;
    wgpu::VertexBufferDescriptor {
      stride: size_of::<Vertex>() as wgpu::BufferAddress,
      step_mode: wgpu::InputStepMode::Vertex,
      attributes: &[
        wgpu::VertexAttributeDescriptor {
          offset: 0,
          shader_location: 0,
          format: wgpu::VertexFormat::Float2,
        },
        wgpu::VertexAttributeDescriptor {
          offset: size_of::<[f32; 2]>() as wgpu::BufferAddress,
          shader_location: 1,
          format: wgpu::VertexFormat::Float2,
        },
        wgpu::VertexAttributeDescriptor {
          offset: (size_of::<[f32; 2]>() * 2) as wgpu::BufferAddress,
          shader_location: 2,
          format: wgpu::VertexFormat::Uint,
        },
      ],
    }
  }
}

/// Flush all data to the texture and ready to commit to gpu.
  /// Call this function before commit drawing to gpu.
  pub fn flush_atlas_cache(
    &mut self,
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    texture: &wgpu::Texture,
  ) {
    if self.texture.is_updated() {
      let DeviceSize { width, height, .. } = *self.texture.size();
      let buffer =
        device.create_buffer_with_data(self.texture.as_bytes(), wgpu::BufferUsage::COPY_SRC);

      encoder.copy_buffer_to_texture(
        wgpu::BufferCopyView {
          buffer: &buffer,
          layout: wgpu::TextureDataLayout {
            offset: 0,
            bytes_per_row: width * std::mem::size_of::<u32>() as u32,
            rows_per_image: height,
          },
        },
        wgpu::TextureCopyView {
          texture,
          mip_level: 0,
          origin: wgpu::Origin3d::ZERO,
        },
        wgpu::Extent3d {
          width,
          height,
          depth: 1,
        },
      )
    }
  }

pub fn flush_glyph_cache(
  &mut self,
  device: &wgpu::Device,
  encoder: &mut wgpu::CommandEncoder,
  texture: &wgpu::Texture,
) {
  if self.texture.is_updated() {
    let buffer =
      device.create_buffer_with_data(self.texture.as_bytes(), wgpu::BufferUsage::COPY_SRC);

    let DeviceSize { width, height, .. } = *self.texture.size();
    encoder.copy_buffer_to_texture(
      wgpu::BufferCopyView {
        buffer: &buffer,
        layout: wgpu::TextureDataLayout {
          offset: 0,
          bytes_per_row: width,
          rows_per_image: height,
        },
      },
      wgpu::TextureCopyView {
        texture,
        mip_level: 0,
        origin: wgpu::Origin3d::ZERO,
      },
      wgpu::Extent3d {
        width,
        height,
        depth: 1,
      },
    );
    self.texture.data_synced();
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::*;
  use futures::executor::block_on;

  fn circle_50() -> Path {
    let mut path = Path::builder();
    path.add_circle(euclid::Point2D::new(0., 0.), 50., Winding::Positive);
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

  #[test]
  #[ignore = "gpu need"]
  fn render_command_upload_indices_check() {
    use lyon::tessellation::VertexBuffers;

    let mut canvas = block_on(Canvas::new(DeviceSize::new(100, 100)));

    let v = layer_2d::Vertex {
      pixel_coords: Point::new(0., 0.),
      texture_coords: Point::new(-1., -1.),
    };
    let r_cmd = RenderCommand {
      geometry: VertexBuffers {
        vertices: vec![v.clone(), v.clone(), v],
        indices: vec![0, 1, 2],
      },
      attrs: vec![super::RenderAttr {
        count: lyon::tessellation::Count {
          indices: 3,
          vertices: 3,
        },
        align_bounds: Rect::default(),
        style: FillStyle::Color(Color::WHITE),
        transform: Transform::default(),
      }],
    };

    canvas.upload_render_command(&r_cmd);
    canvas.upload_render_command(&r_cmd);

    let data = &canvas.render_data;
    debug_assert_eq!(data.vertices.len(), 6);
    debug_assert_eq!(&data.indices, &[0, 1, 2, 3, 4, 5]);
  }

  #[test]
  #[ignore = "gpu need"]
  fn smoke_draw_circle() {
    let mut canvas = block_on(Canvas::new(DeviceSize::new(400, 400)));
    let path = circle_50();

    let mut layer = canvas.new_2d_layer();
    layer.set_style(FillStyle::Color(Color::BLACK));
    layer.translate(50., 50.);
    layer.fill_path(path);
    canvas.compose_2d_layer(layer);
    canvas.submit();

    unit_test::assert_canvas_eq!(canvas, "./test_imgs/smoke_draw_circle.png");
  }

  #[test]
  #[ignore = "gpu need"]
  fn color_palette_texture() {
    let mut canvas = block_on(Canvas::new(DeviceSize::new(400, 400)));
    let path = circle_50();
    {
      let mut layer = canvas.new_2d_layer();

      let mut fill_color_circle = |color: Color, offset_x: f32, offset_y: f32| {
        layer
          .set_style(FillStyle::Color(color))
          .translate(offset_x, offset_y)
          .fill_path(path.clone());
      };

      fill_color_circle(Color::YELLOW, 50., 50.);
      fill_color_circle(Color::RED, 100., 0.);
      fill_color_circle(Color::PINK, 100., 0.);
      fill_color_circle(Color::GREEN, 100., 0.);
      fill_color_circle(Color::BLUE, -0., 100.);

      canvas.compose_2d_layer(layer);
      canvas.submit();

      unit_test::assert_canvas_eq!(canvas, "./test_imgs/color_palette_texture.png");
    }
  }
}
