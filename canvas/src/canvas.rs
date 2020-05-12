use super::{
  FillStyle, LogicUnit, PhysicPoint, PhysicSize, PhysicUnit, Point, RenderAttr,
  RenderCommand, Rendering2DLayer,
};
use zerocopy::AsBytes;

use super::atlas::{AtlasStoreErr, TextureAtlas};

enum CanvasBindings {
  GlobalUniform = 0,
  Primitives = 1,
  TextureAtlas = 2,
  TextureAtlasSampler = 3,
}

pub struct Canvas {
  surface: wgpu::Surface,
  device: wgpu::Device,
  queue: wgpu::Queue,
  swap_chain: wgpu::SwapChain,
  sc_desc: wgpu::SwapChainDescriptor,
  pipeline: wgpu::RenderPipeline,
  tex_infos_layout: wgpu::BindGroupLayout,
  uniform_layout: wgpu::BindGroupLayout,
  uniforms: wgpu::BindGroup,

  // texture atlas for pure color and image to draw.
  tex_atlas: TextureAtlas,
  tex_atlas_sampler: wgpu::Sampler,

  render_data: RenderData,
}

/// Frame is created by Canvas, and provide a blank box to drawing. It's
/// guarantee auto commit all data to texture when is drop.
pub trait Frame {
  /// Create a 2d layer to drawing, and not effect current canvas before compose
  /// back to the canvas.
  #[inline]
  fn new_2d_layer(&self) -> Rendering2DLayer { Rendering2DLayer::new() }

  /// Compose a layer into the canvas.
  #[inline]
  fn compose_2d_layer(&mut self, layer: Rendering2DLayer) {
    self.upload_render_command(&layer.finish())
  }

  /// Upload a RenderCommand into current frame. RenderCommand is the result
  /// of a layer drawing finished.
  fn upload_render_command(&mut self, command: &RenderCommand);

  /// Commit all uploaded render command, but will not present in your texture
  /// before [submit](Frame::submit) called.
  fn draw(&mut self);

  /// Submits a series of finished command buffers for execution. You needn't
  /// call this method manually, only if you want flush drawing things into gpu
  /// immediately.
  fn submit(&mut self);

  /// Return the host canvas.
  fn canvas(&mut self) -> &mut Canvas;
}

/// A frame for screen, anything drawing on the frame will commit to screen
/// display.
pub struct ScreenFrame<'a> {
  texture: wgpu::SwapChainOutput,
  canvas: &'a mut Canvas,
  encoder: Option<wgpu::CommandEncoder>,
}

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
pub struct TextureFrame<'a> {
  texture: wgpu::Texture,
  view: wgpu::TextureView,
  canvas: &'a mut Canvas,
  encoder: Option<wgpu::CommandEncoder>,
}

impl<'a> TextureFrame<'a> {
  /// PNG encoded the texture frame then write by `writer`.
  pub async fn png_encode<W: std::io::Write>(
    mut self,
    writer: W,
  ) -> Result<(), &'static str> {
    let device = &self.canvas.device;
    let sc_desc = &self.canvas.sc_desc;
    let width = sc_desc.width;
    let height = sc_desc.height;
    let size = width as u64 * height as u64 * std::mem::size_of::<u32>() as u64;

    // The output buffer lets us retrieve the data as an array
    let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
      size,
      usage: wgpu::BufferUsage::MAP_READ | wgpu::BufferUsage::COPY_DST,
      label: None,
    });

    // Copy the data from the texture to the buffer
    if self.encoder.is_none() {
      self.encoder = Some(self.canvas.new_encoder())
    }
    let encoder = self.encoder.as_mut().unwrap();
    self.canvas.draw(&self.view, encoder);
    encoder.copy_texture_to_buffer(
      wgpu::TextureCopyView {
        texture: &self.texture,
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

    // Submit what are drawing before capture
    self.submit();

    // Note that we're not calling `.await` here.
    let buffer_future = output_buffer.map_read(0, size);

    // Poll the device in a blocking manner so that our future resolves.
    self.canvas.device.poll(wgpu::Maintain::Wait);

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
    let [uniform_layout, tex_infos_layout] = create_uniform_layout(&device);
    let pipeline = create_render_pipeline(
      &device,
      &sc_desc,
      &[&uniform_layout, &tex_infos_layout],
    );

    let tex_atlas = TextureAtlas::new(&device);
    let tex_atlas_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
      address_mode_u: wgpu::AddressMode::ClampToEdge,
      address_mode_v: wgpu::AddressMode::ClampToEdge,
      address_mode_w: wgpu::AddressMode::ClampToEdge,
      mag_filter: wgpu::FilterMode::Linear,
      min_filter: wgpu::FilterMode::Linear,
      mipmap_filter: wgpu::FilterMode::Linear,
      lod_min_clamp: 0.0,
      lod_max_clamp: 0.0,
      compare: wgpu::CompareFunction::Always,
    });

    let uniforms = create_uniforms(
      &device,
      &uniform_layout,
      tex_atlas.size(),
      &coordinate_2d_to_device_matrix(width, height),
      &tex_atlas_sampler,
      &tex_atlas.view,
    );

    Canvas {
      tex_atlas,
      tex_atlas_sampler,
      device,
      surface,
      queue,
      swap_chain,
      sc_desc,
      pipeline: pipeline,
      uniform_layout,
      tex_infos_layout,
      uniforms,
      render_data: RenderData::default(),
    }
  }

  /// Create a new frame texture to draw, and commit to device when the `Frame`
  /// is dropped.
  pub fn new_screen_frame(&mut self) -> ScreenFrame {
    let chain_output = self
      .swap_chain
      .get_next_texture()
      .expect("Timeout getting texture");

    ScreenFrame {
      encoder: None,
      texture: chain_output,
      canvas: self,
    }
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

    TextureFrame {
      view: texture.create_default_view(),
      texture,
      canvas: self,
      encoder: None,
    }
  }

  /// Resize canvas
  pub fn resize(&mut self, width: u32, height: u32) {
    self.sc_desc.width = width;
    self.sc_desc.height = height;
    self.swap_chain =
      self.device.create_swap_chain(&self.surface, &self.sc_desc);
    self.update_uniforms();
  }
}

impl Canvas {
  fn draw(
    &mut self,
    view: &wgpu::TextureView,
    encoder: &mut wgpu::CommandEncoder,
  ) {
    let device = &self.device;

    self.tex_atlas.flush(device, encoder);
    let vertices_buffer = device.create_buffer_with_data(
      self.render_data.vertices.as_bytes(),
      wgpu::BufferUsage::VERTEX,
    );

    let indices_buffer = device.create_buffer_with_data(
      self.render_data.indices.as_bytes(),
      wgpu::BufferUsage::INDEX,
    );

    let tex_infos_bind_group = self.create_primitives_bind_group();
    {
      let mut render_pass =
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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
      render_pass.set_vertex_buffer(0, &vertices_buffer, 0, 0);
      render_pass.set_index_buffer(&indices_buffer, 0, 0);
      render_pass.set_bind_group(0, &self.uniforms, &[]);
      render_pass.set_bind_group(1, &tex_infos_bind_group, &[]);

      render_pass.draw_indexed(
        0..self.render_data.indices.len() as u32,
        0,
        0..1,
      );
    }

    self.render_data.clear();
  }

  pub(crate) fn new_encoder(&mut self) -> wgpu::CommandEncoder {
    self
      .device
      .create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Render Encoder"),
      })
  }

  fn store_style_in_atlas(
    &mut self,
    style: &FillStyle,
    encoder: &mut wgpu::CommandEncoder,
  ) -> Result<(PhysicPoint, PhysicSize), AtlasStoreErr> {
    let (pos, size, grown) = match style {
      FillStyle::Color(c) => {
        let (pos, grown) =
          self
            .tex_atlas
            .store_color_in_palette(*c, &self.device, encoder)?;

        (pos, PhysicSize::new(1, 1), grown)
      }
      _ => todo!("not support in early develop"),
    };

    if grown {
      self.update_uniforms();
    }
    Ok((pos, size))
  }

  #[inline]
  fn update_uniforms(&mut self) {
    self.uniforms = create_uniforms(
      &self.device,
      &self.uniform_layout,
      self.tex_atlas.size(),
      &coordinate_2d_to_device_matrix(self.sc_desc.width, self.sc_desc.height),
      &self.tex_atlas_sampler,
      &self.tex_atlas.view,
    )
  }

  fn create_primitives_bind_group(&mut self) -> wgpu::BindGroup {
    let primitives = &self.render_data.primitives;
    let primitives_buffer = self.device.create_buffer_with_data(
      primitives.as_bytes(),
      wgpu::BufferUsage::STORAGE_READ,
    );
    let size = primitives.len() * std::mem::size_of::<Primitive>();
    self.device.create_bind_group(&wgpu::BindGroupDescriptor {
      layout: &self.tex_infos_layout,
      bindings: &[wgpu::Binding {
        binding: CanvasBindings::Primitives as u32,
        resource: wgpu::BindingResource::Buffer {
          buffer: &primitives_buffer,
          range: 0..size as wgpu::BufferAddress,
        },
      }],
      label: Some("texture infos bind group"),
    })
  }
}

fn create_render_pipeline(
  device: &wgpu::Device,
  sc_desc: &wgpu::SwapChainDescriptor,
  bind_group_layouts: &[&wgpu::BindGroupLayout; 2],
) -> wgpu::RenderPipeline {
  let render_pipeline_layout =
    device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
      bind_group_layouts,
    });

  let (vs_module, fs_module) = create_shaders(device);

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
      index_format: wgpu::IndexFormat::Uint32,
      vertex_buffers: &[Vertex::desc()],
    },
    sample_count: 1,
    sample_mask: !0,
    alpha_to_coverage_enabled: false,
  })
}

fn create_shaders(
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

fn create_uniform_layout(device: &wgpu::Device) -> [wgpu::BindGroupLayout; 2] {
  let stable =
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
      bindings: &[
        wgpu::BindGroupLayoutEntry {
          binding: CanvasBindings::GlobalUniform as u32,
          visibility: wgpu::ShaderStage::VERTEX,
          ty: wgpu::BindingType::UniformBuffer { dynamic: false },
        },
        wgpu::BindGroupLayoutEntry {
          binding: CanvasBindings::TextureAtlas as u32,
          visibility: wgpu::ShaderStage::FRAGMENT,
          ty: wgpu::BindingType::SampledTexture {
            dimension: wgpu::TextureViewDimension::D2,
            component_type: wgpu::TextureComponentType::Float,
            multisampled: false,
          },
        },
        wgpu::BindGroupLayoutEntry {
          binding: CanvasBindings::TextureAtlasSampler as u32,
          visibility: wgpu::ShaderStage::FRAGMENT,
          ty: wgpu::BindingType::Sampler { comparison: false },
        },
      ],
      label: Some("uniforms stable layout"),
    });

  let dynamic =
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
      bindings: &[wgpu::BindGroupLayoutEntry {
        binding: CanvasBindings::Primitives as u32,
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
  euclid::Transform2D::row_major(
    2. / width as f32,
    0.,
    0.,
    -2. / height as f32,
    -1.,
    1.,
  )
}

fn create_uniforms(
  device: &wgpu::Device,
  layout: &wgpu::BindGroupLayout,
  atlas_size: PhysicSize,
  canvas_2d_to_device_matrix: &euclid::Transform2D<f32, LogicUnit, PhysicUnit>,
  tex_atlas_sampler: &wgpu::Sampler,
  tex_atlas: &wgpu::TextureView,
) -> wgpu::BindGroup {
  let uniform = GlobalUniform {
    texture_atlas_size: [atlas_size.width, atlas_size.height],
    canvas_coordinate_map: canvas_2d_to_device_matrix.to_row_major_array(),
  };
  let uniform_buffer = device.create_buffer_with_data(
    &uniform.as_bytes(),
    wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
  );
  device.create_bind_group(&wgpu::BindGroupDescriptor {
    layout: layout,
    bindings: &[
      wgpu::Binding {
        binding: CanvasBindings::GlobalUniform as u32,
        resource: wgpu::BindingResource::Buffer {
          buffer: &uniform_buffer,
          range: 0..std::mem::size_of::<GlobalUniform>() as wgpu::BufferAddress,
        },
      },
      wgpu::Binding {
        binding: CanvasBindings::TextureAtlas as u32,
        resource: wgpu::BindingResource::TextureView(tex_atlas),
      },
      wgpu::Binding {
        binding: CanvasBindings::TextureAtlasSampler as u32,
        resource: wgpu::BindingResource::Sampler(tex_atlas_sampler),
      },
    ],
    label: Some("uniform_bind_group"),
  })
}

fn mut_encoder<'a>(
  canvas: &mut Canvas,
  encoder_store: &'a mut Option<wgpu::CommandEncoder>,
) -> &'a mut wgpu::CommandEncoder {
  if encoder_store.is_none() {
    *encoder_store = Some(canvas.new_encoder())
  }
  encoder_store.as_mut().unwrap()
}

fn upload_render_command(
  command: &RenderCommand,
  canvas: &mut Canvas,
  encoder: &mut wgpu::CommandEncoder,
  view: &wgpu::TextureView,
) {
  let RenderCommand { attrs, geometry } = command;

  let mut v_start = 0;
  let mut i_start = 0;
  attrs.iter().for_each(
    |RenderAttr {
       transform,
       count,
       style,
     }| {
      let res = canvas.store_style_in_atlas(style, encoder).or_else(|err| {
        canvas.draw(view, encoder);

        // Todo: we should not directly clear the texture atlas,
        // but deallocate all not used texture.
        canvas.tex_atlas.clear(&canvas.device, &canvas.queue);

        match err {
          AtlasStoreErr::SpaceNotEnough => {
            let res = canvas.store_style_in_atlas(style, encoder);
            debug_assert!(res.is_ok());
            res
          }
          AtlasStoreErr::OverTheMaxLimit => {
            unimplemented!("draw current attr individual");
            Err(err)
          }
        }
      });

      let v_end = v_start + count.vertices as usize;
      let i_end = i_start + count.indices as usize;

      // Error already processed before, needn't care about it.
      if let Ok((tex_offset, tex_size)) = res {
        let tex_info = Primitive {
          tex_offset: [tex_offset.x, tex_offset.y],
          tex_size: [tex_size.width, tex_size.height],
          transform: transform.to_row_major_array(),
        };

        canvas.render_data.append(
          &geometry.vertices[v_start..v_end],
          &geometry.indices[i_start..i_end],
          tex_info,
        )
      }

      v_start = v_end;
      i_start = i_end;
    },
  );
}

macro frame_delegate_impl($($path: ident).*) {
  #[inline]
  fn canvas(&mut self) -> &mut Canvas { &mut self.canvas }

  fn draw(&mut self) {
    if self.canvas.render_data.has_data() {
      let encoder = mut_encoder(&mut self.canvas, &mut self.encoder);

      self
        .canvas
        .draw(&self$(.$path)*, encoder);
    }
  }

  fn submit(&mut self) {
    self.draw();

    if let Some(encoder) = self.encoder.take() {
      self.canvas().queue.submit(&[encoder.finish()]);
    }
  }

  fn upload_render_command(&mut self, command: &RenderCommand) {
    let Self {canvas, encoder, ..} = self;
    let encoder = mut_encoder(canvas, encoder);
    let view = &self$(.$path)*;
    upload_render_command(command, canvas, encoder, view);
  }
}

impl<'a> Frame for ScreenFrame<'a> {
  frame_delegate_impl!(texture.view);
}

impl<'a> Frame for TextureFrame<'a> {
  frame_delegate_impl!(view);
}

impl<'a> Drop for ScreenFrame<'a> {
  #[inline]
  fn drop(&mut self) { self.submit(); }
}

impl<'a> Drop for TextureFrame<'a> {
  #[inline]
  fn drop(&mut self) { self.submit(); }
}

/// We use a texture atlas to shader vertices, even if a pure color path.
#[repr(C)]
#[derive(Copy, Clone, Debug, AsBytes)]
struct Vertex {
  pos: [f32; 2],
  tex_id: u32,
}

#[repr(C)]
#[derive(AsBytes)]
struct GlobalUniform {
  canvas_coordinate_map: [f32; 6],
  texture_atlas_size: [u32; 2],
}

#[repr(C)]
#[derive(AsBytes)]
struct Primitive {
  // Texture offset in texture atlas.
  tex_offset: [u32; 2],
  // Texture size in texture atlas.
  tex_size: [u32; 2],
  transform: [f32; 6],
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

#[derive(Default)]
struct RenderData {
  vertices: Vec<Vertex>,
  indices: Vec<u32>,
  primitives: Vec<Primitive>,
}

impl RenderData {
  #[inline]
  fn has_data(&mut self) -> bool {
    debug_assert_eq!(self.vertices.is_empty(), self.indices.is_empty());
    debug_assert_eq!(self.vertices.is_empty(), self.primitives.is_empty());

    !self.vertices.is_empty()
  }

  fn clear(&mut self) {
    self.vertices.clear();
    self.indices.clear();
    self.primitives.clear();
  }

  fn append(
    &mut self,
    vertices: &[Point],
    indices: &[u32],
    tex_info: Primitive,
  ) {
    let tex_id = self.primitives.len() as u32;
    self.primitives.push(tex_info);

    let vertex_offset = self.vertices.len() as u32;
    let mapped_indices = indices.iter().map(|index| index + vertex_offset);
    self.indices.extend(mapped_indices);

    let mapped_vertices = vertices.iter().map(|pos| Vertex {
      pos: [pos.x, pos.y],
      tex_id,
    });
    self.vertices.extend(mapped_vertices);
  }
}

#[cfg(test)]
mod tests {
  use super::*;

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
