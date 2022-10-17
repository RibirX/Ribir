use crate::Vertex;

pub struct StencilPass {
  pub uniform_layout: wgpu::BindGroupLayout,
  pub uniform: wgpu::BindGroup,
  pub push_stencil_pipeline: wgpu::RenderPipeline,
  pub pop_stencil_pipeline: wgpu::RenderPipeline,
}

impl StencilPass {
  pub fn new(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    coordinate_matrix: &wgpu::Buffer,
    primitive_layout: &wgpu::BindGroupLayout,
    msaa_count: u32,
  ) -> Self {
    let uniform_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
      label: Some("uniforms stable layout"),
    });

    let uniform = uniform_bind_group(device, &uniform_layout, coordinate_matrix);
    let push_stencil_pipeline = pipeline(
      device,
      format,
      &uniform_layout,
      primitive_layout,
      false,
      msaa_count,
    );
    let pop_stencil_pipeline = pipeline(
      device,
      format,
      &uniform_layout,
      primitive_layout,
      true,
      msaa_count,
    );

    Self {
      uniform_layout,
      uniform,
      push_stencil_pipeline,
      pop_stencil_pipeline,
    }
  }

  pub fn set_anti_aliasing(
    &mut self,
    msaa_count: u32,
    primitive_layout: &wgpu::BindGroupLayout,
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
  ) {
    self.push_stencil_pipeline = pipeline(
      device,
      format,
      &self.uniform_layout,
      primitive_layout,
      false,
      msaa_count,
    );
    self.pop_stencil_pipeline = pipeline(
      device,
      format,
      &self.uniform_layout,
      primitive_layout,
      true,
      msaa_count,
    );
  }

  pub fn resize(&mut self, coordinate_matrix: &wgpu::Buffer, device: &wgpu::Device) {
    self.uniform = uniform_bind_group(device, &self.uniform_layout, coordinate_matrix);
  }
}

fn uniform_bind_group(
  device: &wgpu::Device,
  layout: &wgpu::BindGroupLayout,
  coordinate_matrix: &wgpu::Buffer,
) -> wgpu::BindGroup {
  device.create_bind_group(&wgpu::BindGroupDescriptor {
    layout,
    entries: &[wgpu::BindGroupEntry {
      binding: 0,
      resource: coordinate_matrix.as_entire_binding(),
    }],
    label: Some("Stencil uniforms bind group"),
  })
}

fn pipeline(
  device: &wgpu::Device,
  format: wgpu::TextureFormat,
  uniform_layout: &wgpu::BindGroupLayout,
  primitive_layout: &wgpu::BindGroupLayout,
  is_clear: bool,
  msaa_count: u32,
) -> wgpu::RenderPipeline {
  let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
    label: Some("Stencil geometry pipeline layout"),
    bind_group_layouts: &[uniform_layout, primitive_layout],
    push_constant_ranges: &[],
  });

  let module = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
    label: Some("Stencil geometry shader"),
    source: wgpu::ShaderSource::Wgsl(include_str!("./shaders/stencil_geometry.wgsl").into()),
  });

  let stencil_state = if is_clear {
    wgpu::StencilFaceState {
      compare: wgpu::CompareFunction::Equal,
      fail_op: wgpu::StencilOperation::Keep,
      depth_fail_op: wgpu::StencilOperation::Keep,
      pass_op: wgpu::StencilOperation::DecrementClamp,
    }
  } else {
    wgpu::StencilFaceState {
      compare: wgpu::CompareFunction::Equal,
      fail_op: wgpu::StencilOperation::Keep,
      depth_fail_op: wgpu::StencilOperation::Keep,
      pass_op: wgpu::StencilOperation::IncrementClamp,
    }
  };

  device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
    label: Some("Stencil geometry pipeline"),
    layout: Some(&pipeline_layout),
    vertex: wgpu::VertexState {
      module: &module,
      entry_point: "vs_main",
      buffers: &[Vertex::desc()],
    },
    fragment: Some(wgpu::FragmentState {
      module: &module,
      entry_point: "fs_main",
      targets: &[wgpu::ColorTargetState {
        format,
        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
        write_mask: wgpu::ColorWrites::empty(),
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
    depth_stencil: Some(wgpu::DepthStencilState {
      format: wgpu::TextureFormat::Depth24PlusStencil8,
      depth_write_enabled: false,
      depth_compare: wgpu::CompareFunction::Always,
      stencil: wgpu::StencilState {
        front: stencil_state,
        back: stencil_state,
        read_mask: 0x0000_0000_0000_FFFF,
        write_mask: 0x0000_0000_0000_FFFF,
      },
      bias: wgpu::DepthBiasState {
        constant: 0,
        slope_scale: 0.,
        clamp: 0.,
      },
    }),
    multisample: wgpu::MultisampleState {
      count: msaa_count,
      mask: !0,
      alpha_to_coverage_enabled: false,
    },
    multiview: None,
  })
}
