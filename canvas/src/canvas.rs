use super::{
  Color, ColorBufferAttr, DeviceUnit, LogicUnit, Point, RenderCommand,
  Rendering2DLayer, TextureBufferAttr, Transform,
};

use lyon::tessellation::VertexBuffers;

pub struct Canvas {
  surface: wgpu::Surface,
  device: wgpu::Device,
  queue: wgpu::Queue,
  swap_chain: wgpu::SwapChain,
  sc_desc: wgpu::SwapChainDescriptor,
  color_pipeline: wgpu::RenderPipeline,
  canvas_2d_coordinate_bind_group_layout: wgpu::BindGroupLayout,
}

/// Frame is created by Canvas, and provide a blank box to drawing. It's
/// guarantee auto commit all data to texture when is drop.
pub trait Frame {
  /// Create a 2d layer to drawing, and not effect current canvas before compose
  /// back to the canvas.
  fn new_2d_layer(&self) -> Rendering2DLayer;
  /// Compose a layer into the canvas.
  fn compose_2d_layer(&mut self, layer: Rendering2DLayer);
  /// Compose a layer buffer into current drawing. Layer buffer is the result
  /// of a layer drawing finished.
  fn compose_2d_layer_buffer(&mut self, commands: &[RenderCommand]);
}

/// A frame for screen, anything drawing on the frame will commit to screen
/// display.
pub struct ScreenFrame<'a>(FrameImpl<'a, wgpu::SwapChainOutput>);

/// A texture frame, don't like [`ScreenFrame`](ScreenFrame), `TextureFrame` not
/// directly present drawing on screen but drawing on a texture. Below example
/// show how to store frame as a png image.
///
/// # Example
///
/// This example draw a circle and write as a image.
/// ```
/// # use canvas::*;
/// fn generate_png(mut canvas: Canvas, file_path: &str) {
///   let mut frame = canvas.new_texture_frame();
///   let mut layer = frame.new_2d_layer();
///   let mut path = Path::builder();
///   layer.set_brush_style(FillStyle::Color(const_color::BLACK.into()));
///   path.add_circle(euclid::Point2D::new(200., 200.), 100., Winding::Positive);
///   let path = path.build();
///   layer.fill_path(path);
///   frame.compose_2d_layer(layer);
///   futures::executor::block_on(
///     frame
///     .png_encode(std::fs::File::create(file_path).unwrap()),
///   ).unwrap();
/// }
/// ```
pub struct TextureFrame<'a>(FrameImpl<'a, TextureTextureView>);

impl<'a> TextureFrame<'a> {
  /// PNG encoded the texture frame then write by `writer`.
  pub async fn png_encode<W: std::io::Write>(
    mut self,
    writer: W,
  ) -> Result<(), &'static str> {
    let device = &self.0.canvas.device;
    let sc_desc = &self.0.canvas.sc_desc;
    let width = sc_desc.width;
    let height = sc_desc.height;
    let size = width as u64 * height as u64 * std::mem::size_of::<u32>() as u64;

    // The output buffer lets us retrieve the data as an array
    let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
      size,
      usage: wgpu::BufferUsage::MAP_READ | wgpu::BufferUsage::COPY_DST,
      label: None,
    });

    self.0.commit_buffer();

    // Copy the data from the texture to the buffer
    encoder_mut!(self.0).copy_texture_to_buffer(
      wgpu::TextureCopyView {
        texture: &self.0.texture.texture,
        mip_level: 0,
        array_layer: 0,
        origin: wgpu::Origin3d::ZERO,
      },
      wgpu::BufferCopyView {
        buffer: &output_buffer,
        offset: 0,
        bytes_per_row: std::mem::size_of::<u32>() as u32 * width as u32,
        rows_per_image: 0,
      },
      wgpu::Extent3d {
        width,
        height,
        depth: 1,
      },
    );

    // Drop this frame and commit render data to gpu before encode to png.
    std::mem::drop(self);

    // Note that we're not calling `.await` here.
    let buffer_future = output_buffer.map_read(0, size);

    // Poll the device in a blocking manner so that our future resolves.
    device.poll(wgpu::Maintain::Wait);

    let mapping = buffer_future.await.map_err(|_| "Async buffer error")?;
    let mut png_encoder = png::Encoder::new(writer, width, height);
    png_encoder.set_depth(png::BitDepth::Eight);
    png_encoder.set_color(png::ColorType::RGBA);
    png_encoder
      .write_header()
      .unwrap()
      .write_image_data(mapping.as_slice())
      .unwrap();

    Ok(())
  }

  /// Save the texture frame as a PNG image, store at the `path` location.
  pub async fn save_as_png(self, path: &str) -> Result<(), &'static str> {
    self.png_encode(std::fs::File::create(path).unwrap()).await
  }
}

impl Canvas {
  const COORDINATE_2D_BINDING_INDEX: u32 = 0;

  /// Create a canvas by a native window.
  pub async fn new<W: raw_window_handle::HasRawWindowHandle>(
    window: &W,
    width: u32,
    height: u32,
  ) -> Self {
    let surface = wgpu::Surface::create(window);
    let adapter = wgpu::Adapter::request(
      &wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::Default,
        compatible_surface: Some(&surface),
      },
      wgpu::BackendBit::PRIMARY,
    )
    .await
    .unwrap();

    let (device, queue) = adapter
      .request_device(&wgpu::DeviceDescriptor {
        extensions: wgpu::Extensions {
          anisotropic_filtering: false,
        },
        limits: Default::default(),
      })
      .await;

    let sc_desc = wgpu::SwapChainDescriptor {
      usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
      format: wgpu::TextureFormat::Bgra8UnormSrgb,
      width,
      height,
      present_mode: wgpu::PresentMode::Fifo,
    };
    let swap_chain = device.create_swap_chain(&surface, &sc_desc);
    let canvas_2d_coordinate_bind_group_layout = device
      .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        bindings: &[wgpu::BindGroupLayoutEntry {
          binding: Self::COORDINATE_2D_BINDING_INDEX,
          visibility: wgpu::ShaderStage::VERTEX,
          ty: wgpu::BindingType::UniformBuffer { dynamic: false },
        }],
        label: Some("canvas_2d_coordinate_bind_group_layout"),
      });
    let color_pipeline = Self::create_color_render_pipeline(
      &device,
      &sc_desc,
      &[&canvas_2d_coordinate_bind_group_layout],
    );

    Canvas {
      device,
      surface,
      queue,
      swap_chain,
      sc_desc,
      color_pipeline,
      canvas_2d_coordinate_bind_group_layout,
    }
  }

  /// Create a new frame texture to draw, and commit to device when the `Frame`
  /// is dropped.
  pub fn new_screen_frame(&mut self) -> ScreenFrame {
    let chain_output = self
      .swap_chain
      .get_next_texture()
      .expect("Timeout getting texture");

    let encoder =
      self
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
          label: Some("Render Encoder"),
        });

    ScreenFrame(FrameImpl {
      texture: chain_output,
      canvas: self,
      encoder: Some(encoder),
      buffer: None,
    })
  }

  pub fn new_texture_frame(&mut self) -> TextureFrame {
    let wgpu::SwapChainDescriptor {
      width,
      height,
      format,
      ..
    } = self.sc_desc;
    // The render pipeline renders data into this texture
    let texture = self.device.create_texture(&wgpu::TextureDescriptor {
      size: wgpu::Extent3d {
        width,
        height,
        depth: 1,
      },
      mip_level_count: 1,
      sample_count: 1,
      array_layer_count: 1,
      dimension: wgpu::TextureDimension::D2,
      format,
      usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT
        | wgpu::TextureUsage::COPY_SRC,
      label: None,
    });

    let encoder =
      self
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
          label: Some("Render Encoder"),
        });

    TextureFrame(FrameImpl {
      texture: TextureTextureView {
        view: texture.create_default_view(),
        texture,
      },

      canvas: self,
      encoder: Some(encoder),
      buffer: None,
    })
  }

  pub fn resize(&mut self, width: u32, height: u32) {
    self.sc_desc.width = width;
    self.sc_desc.height = height;
    self.swap_chain =
      self.device.create_swap_chain(&self.surface, &self.sc_desc);
  }

  /// Convert coordinate system from canvas 2d into wgpu.
  pub fn coordinate_2d_to_device_matrix(
    &self,
  ) -> euclid::Transform2D<f32, LogicUnit, DeviceUnit> {
    euclid::Transform2D::row_major(
      2. / self.sc_desc.width as f32,
      0.,
      0.,
      -2. / self.sc_desc.height as f32,
      -1.,
      1.,
    )
  }
}

impl Canvas {
  fn create_color_render_pipeline(
    device: &wgpu::Device,
    sc_desc: &wgpu::SwapChainDescriptor,
    bind_group_layouts: &[&wgpu::BindGroupLayout],
  ) -> wgpu::RenderPipeline {
    let render_pipeline_layout =
      device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        bind_group_layouts,
      });

    let (vs_module, fs_module) = Self::color_shaders(device);

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
        color_blend: wgpu::BlendDescriptor::REPLACE,
        alpha_blend: wgpu::BlendDescriptor::REPLACE,
        write_mask: wgpu::ColorWrite::ALL,
      }],
      primitive_topology: wgpu::PrimitiveTopology::TriangleList,
      depth_stencil_state: None,
      vertex_state: wgpu::VertexStateDescriptor {
        index_format: wgpu::IndexFormat::Uint16,
        vertex_buffers: &[Vertex::desc()],
      },
      sample_count: 1,
      sample_mask: !0,
      alpha_to_coverage_enabled: false,
    })
  }

  fn color_shaders(
    device: &wgpu::Device,
  ) -> (wgpu::ShaderModule, wgpu::ShaderModule) {
    let vs_bytes = include_bytes!("./shaders/geometry.vert.spv");
    let fs_bytes = include_bytes!("./shaders/geometry.frag.spv");
    let vs_spv = wgpu::read_spirv(std::io::Cursor::new(&vs_bytes[..])).unwrap();
    let fs_spv = wgpu::read_spirv(std::io::Cursor::new(&fs_bytes[..])).unwrap();
    let vs_module = device.create_shader_module(&vs_spv);
    let fs_module = device.create_shader_module(&fs_spv);

    (vs_module, fs_module)
  }
}

macro frame_impl_delegate($ty: ident) {
  impl<'a> Frame for $ty<'a> {
    #[inline]
    fn new_2d_layer(&self) -> Rendering2DLayer { self.0.new_2d_layer() }

    #[inline]
    fn compose_2d_layer(&mut self, other_layer: Rendering2DLayer) {
      self.0.compose_2d_layer(other_layer)
    }

    #[inline]
    fn compose_2d_layer_buffer(&mut self, commands: &[RenderCommand]) {
      self.0.compose_2d_layer_buffer(commands);
    }
  }
}

frame_impl_delegate!(ScreenFrame);
frame_impl_delegate!(TextureFrame);

trait FrameTextureView {
  fn texture_view(&self) -> &wgpu::TextureView;
}

impl FrameTextureView for wgpu::SwapChainOutput {
  #[inline]
  fn texture_view(&self) -> &wgpu::TextureView { &self.view }
}

impl FrameTextureView for TextureTextureView {
  #[inline]
  fn texture_view(&self) -> &wgpu::TextureView { &self.view }
}
struct TextureTextureView {
  texture: wgpu::Texture,
  view: wgpu::TextureView,
}

struct FrameImpl<'a, T: FrameTextureView> {
  texture: T,
  encoder: Option<wgpu::CommandEncoder>,
  buffer: Option<RenderCommand>,
  canvas: &'a Canvas,
}

macro encoder_mut($frame: expr) {
  $frame
    .encoder
    .as_mut()
    .expect("Encoder should always exist before drop!")
}

impl<'a, T: FrameTextureView> Frame for FrameImpl<'a, T> {
  #[inline]
  fn new_2d_layer(&self) -> Rendering2DLayer { Rendering2DLayer::new() }

  #[inline]
  fn compose_2d_layer(&mut self, other_layer: Rendering2DLayer) {
    self.compose_2d_layer_buffer(&other_layer.finish())
  }

  #[inline]
  fn compose_2d_layer_buffer(&mut self, commands: &[RenderCommand]) {
    // if the first render command is same type with last layer's last render
    // command, will merge them into one to commit.
    let mut merged = false;
    if let Some(last) = &mut self.buffer {
      if let Some(first) = commands.first() {
        merged = last.merge(first);
      }
    }

    // Skip the first command if it merged into buffer
    let start = merged as usize;
    if commands.len() > start {
      self.commit_buffer();
      let end = commands.len() - 1;
      if end > start {
        commands[start..end]
          .iter()
          .for_each(|cmd| self.commit_command(cmd));
      }

      // Retain the last command as new buffer to merge new layer.
      self.buffer = commands.last().cloned();
    }
  }
}

impl<'a, T: FrameTextureView> FrameImpl<'a, T> {
  fn commit_command(&mut self, command: &RenderCommand) {
    match command {
      RenderCommand::PureColor { geometry, attrs } => {
        self.commit_pure_color_command(geometry, attrs);
      }
      RenderCommand::Texture { geometry, attrs } => {
        self.commit_texture_command(geometry, attrs)
      }
    }
  }

  fn commit_pure_color_command(
    &mut self,
    geometry: &VertexBuffers<Point, u16>,
    attrs: &[ColorBufferAttr],
  ) {
    let mut vertices = Vec::with_capacity(geometry.vertices.len());
    geometry.vertices.iter().for_each(|pos| {
      vertices.push(Vertex {
        pos: *pos,
        prim_id: 0,
      })
    });

    let mut primitives = Vec::with_capacity(attrs.len());
    attrs.iter().for_each(|attr| {
      let rg = &attr.rg_attr.rg;
      geometry.indices[rg.start..rg.end].iter().for_each(|idx| {
        vertices[*idx as usize].prim_id = primitives.len() as u32
      });
      primitives.push(ColorPrimitive {
        color: attr.color,
        transform: attr.rg_attr.transform,
      });
    });

    let coordinate_map =
      CoordinateMapMatrix(self.canvas.coordinate_2d_to_device_matrix());

    let device = &self.canvas.device;

    let vertices_buffer = device.create_buffer_with_data(
      bytemuck::cast_slice(vertices.as_slice()),
      wgpu::BufferUsage::VERTEX,
    );

    let indices_buffer = device.create_buffer_with_data(
      bytemuck::cast_slice(geometry.indices.as_slice()),
      wgpu::BufferUsage::INDEX,
    );

    let uniform_buffer = device.create_buffer_with_data(
      bytemuck::cast_slice(&[coordinate_map]),
      wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
    );

    let uniform_bind_group =
      device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &self.canvas.canvas_2d_coordinate_bind_group_layout,
        bindings: &[wgpu::Binding {
          binding: 0,
          resource: wgpu::BindingResource::Buffer {
            buffer: &uniform_buffer,
            range: 0..std::mem::size_of_val(&coordinate_map)
              as wgpu::BufferAddress,
          },
        }],
        label: Some("uniform_bind_group"),
      });

    let mut render_pass =
      encoder_mut!(self).begin_render_pass(&wgpu::RenderPassDescriptor {
        color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
          attachment: self.texture.texture_view(),
          resolve_target: None,
          load_op: wgpu::LoadOp::Clear,
          store_op: wgpu::StoreOp::Store,
          clear_color: wgpu::Color::WHITE,
        }],
        depth_stencil_attachment: None,
      });
    render_pass.set_pipeline(&self.canvas.color_pipeline);
    render_pass.set_vertex_buffer(0, &vertices_buffer, 0, 0);
    render_pass.set_index_buffer(&indices_buffer, 0, 0);
    render_pass.set_bind_group(
      Canvas::COORDINATE_2D_BINDING_INDEX,
      &uniform_bind_group,
      &[],
    );
    render_pass.draw_indexed(0..geometry.indices.len() as u32, 0, 0..1);
  }

  fn commit_texture_command(
    &mut self,
    _geometry: &VertexBuffers<Point, u16>,
    _attrs: &[TextureBufferAttr],
  ) {
    unimplemented!();
  }

  fn commit_buffer(&mut self) {
    if let Some(cmd) = self.buffer.take() {
      self.commit_command(&cmd);
    }
  }
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct Vertex {
  pos: Point,
  prim_id: u32,
}

unsafe impl bytemuck::Pod for Vertex {}
unsafe impl bytemuck::Zeroable for Vertex {}

#[repr(C)]
struct ColorPrimitive {
  color: Color,
  transform: Transform,
}

impl Vertex {
  fn desc<'a>() -> wgpu::VertexBufferDescriptor<'a> {
    use std::mem;
    wgpu::VertexBufferDescriptor {
      stride: mem::size_of::<Vertex>() as wgpu::BufferAddress,
      step_mode: wgpu::InputStepMode::Vertex,
      attributes: &[
        wgpu::VertexAttributeDescriptor {
          offset: 0,
          shader_location: 0,
          format: wgpu::VertexFormat::Float2,
        },
        wgpu::VertexAttributeDescriptor {
          offset: mem::size_of::<Point>() as wgpu::BufferAddress,
          shader_location: 1,
          format: wgpu::VertexFormat::Uint,
        },
      ],
    }
  }
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct CoordinateMapMatrix(euclid::Transform2D<f32, LogicUnit, DeviceUnit>);

unsafe impl bytemuck::Pod for CoordinateMapMatrix {}
unsafe impl bytemuck::Zeroable for CoordinateMapMatrix {}

fn finish_frame<T: FrameTextureView>(frame: &mut FrameImpl<'_, T>) {
  frame.commit_buffer();

  frame.canvas.queue.submit(&[frame
    .encoder
    .take()
    .expect("Encoder should always exist before drop!")
    .finish()]);
}

impl<'a> Drop for ScreenFrame<'a> {
  #[inline]
  fn drop(&mut self) { finish_frame(&mut self.0); }
}

impl<'a> Drop for TextureFrame<'a> {
  #[inline]
  fn drop(&mut self) { finish_frame(&mut self.0); }
}
