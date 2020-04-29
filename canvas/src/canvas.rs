use super::{LayerBuffer2D, Rendering2DLayer};

pub struct Canvas {
  buffers: Vec<LayerBuffer2D>,
  device: wgpu::Device,
  queue: wgpu::Queue,
  swap_chain: wgpu::SwapChain,
  render_pipeline: wgpu::RenderPipeline,
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
    let render_pipeline = Self::create_render_pipeline(&device, &sc_desc);

    Canvas {
      device,
      queue,
      buffers: vec![],
      swap_chain,
      render_pipeline,
    }
  }

  /// Create a new 2d layer to drawing, and not effect current canvas before
  /// compose back to the canvas.
  #[inline]
  pub fn new_2d_layer(&self) -> Rendering2DLayer { Rendering2DLayer::new() }

  /// Compose a layer into the canvas.
  #[inline]
  pub fn compose_2d_layer(
    &mut self,
    other_layer: Rendering2DLayer,
  ) -> &mut Self {
    self.compose_layer_buffer(&other_layer.finish())
  }

  /// Compose a layer buffer into current drawing. Layer buffer is the result
  /// of a layer drawing finished.
  #[inline]
  pub fn compose_layer_buffer(&mut self, buffer: &LayerBuffer2D) -> &mut Self {
    if let Some(last) = self.buffers.last_mut() {
      if last.mergeable(buffer) {
        last.merge(buffer);
        return self;
      }
    }

    self.buffers.push(buffer.clone());
    self
  }

  /// Commit all composed layer to gpu for painting on screen.
  pub fn render(&mut self) {
    let frame = self
      .swap_chain
      .get_next_texture()
      .expect("Timeout getting texture");

    let mut encoder =
      self
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
          label: Some("Render Encoder"),
        });
    {
      let mut render_pass =
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
          color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
            attachment: &frame.view,
            resolve_target: None,
            load_op: wgpu::LoadOp::Clear,
            store_op: wgpu::StoreOp::Store,
            clear_color: wgpu::Color::WHITE,
          }],
          depth_stencil_attachment: None,
        });
      render_pass.set_pipeline(&mut self.render_pipeline);
      render_pass.draw(0..3, 0..1);
    }

    self.queue.submit(&[encoder.finish()]);
  }

  fn create_render_pipeline(
    device: &wgpu::Device,
    sc_desc: &wgpu::SwapChainDescriptor,
  ) -> wgpu::RenderPipeline {
    let render_pipeline_layout =
      device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        bind_group_layouts: &[],
      });

    let (vs_module, fs_module) = Self::shaders(device);

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
        cull_mode: wgpu::CullMode::Back,
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
        vertex_buffers: &[],
      },
      sample_count: 1,
      sample_mask: !0,
      alpha_to_coverage_enabled: false,
    })
  }

  fn flush_buffers(&mut self) {
    self.buffers.iter().for_each(|buffer| {
      // unimplemented
    });
  }

  fn shaders(
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
