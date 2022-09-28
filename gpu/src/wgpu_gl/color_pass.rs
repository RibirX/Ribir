use crate::Vertex;

pub struct ColorPass {
  pub uniform_layout: wgpu::BindGroupLayout,
  pub uniform: wgpu::BindGroup,
  pub pipeline: wgpu::RenderPipeline,
}

impl ColorPass {
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
    let pipeline = pipeline(
      device,
      format,
      &uniform_layout,
      primitive_layout,
      msaa_count,
    );

    Self { uniform_layout, uniform, pipeline }
  }

  pub fn set_anti_aliasing(
    &mut self,
    msaa_count: u32,
    primitive_layout: &wgpu::BindGroupLayout,
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
  ) {
    self.pipeline = pipeline(
      device,
      format,
      &self.uniform_layout,
      primitive_layout,
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
    label: Some("Color uniforms bind group"),
  })
}

fn pipeline(
  device: &wgpu::Device,
  format: wgpu::TextureFormat,
  uniform_layout: &wgpu::BindGroupLayout,
  primitive_layout: &wgpu::BindGroupLayout,
  msaa_count: u32,
) -> wgpu::RenderPipeline {
  let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
    label: Some("Color geometry pipeline layout"),
    bind_group_layouts: &[uniform_layout, primitive_layout],
    push_constant_ranges: &[],
  });

  let module = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
    label: Some("Color geometry shader"),
    source: wgpu::ShaderSource::Wgsl(include_str!("./shaders/color_geometry.wgsl").into()),
  });

  device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
    label: Some("Color geometry pipeline"),
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
      count: msaa_count,
      mask: !0,
      alpha_to_coverage_enabled: false,
    },
    multiview: None,
  })
}
