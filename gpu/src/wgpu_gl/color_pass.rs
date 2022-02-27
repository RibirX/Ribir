use super::AntiAliasing;
use crate::Vertex;
use painter::DeviceSize;

pub struct ColorPass {
  pub uniform_layout: wgpu::BindGroupLayout,
  pub uniform: wgpu::BindGroup,
  pub multisample_framebuffer: wgpu::TextureView,
  pub pipeline: wgpu::RenderPipeline,
  pub anti_aliasing: AntiAliasing,
  format: wgpu::TextureFormat,
}

impl ColorPass {
  pub fn new(
    device: &wgpu::Device,
    size: DeviceSize,
    format: wgpu::TextureFormat,
    coordinate_matrix: &wgpu::Buffer,
    primitive_layout: &wgpu::BindGroupLayout,
    anti_aliasing: AntiAliasing,
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

    let msaa_count = anti_aliasing as u32;
    let uniform = uniform_bind_group(device, &uniform_layout, coordinate_matrix);
    let multisample_framebuffer = multisample_framebuffer(device, size, format, msaa_count);
    let pipeline = pipeline(
      device,
      format,
      &uniform_layout,
      primitive_layout,
      msaa_count,
    );

    Self {
      uniform_layout,
      uniform,
      multisample_framebuffer,
      pipeline,
      format,
      anti_aliasing,
    }
  }

  pub fn set_anti_aliasing(
    &mut self,
    anti_aliasing: AntiAliasing,
    size: DeviceSize,
    primitive_layout: &wgpu::BindGroupLayout,
    device: &wgpu::Device,
  ) {
    if self.anti_aliasing != anti_aliasing {
      self.anti_aliasing = anti_aliasing;
      let msaa_count = self.anti_aliasing as u32;
      self.multisample_framebuffer = multisample_framebuffer(device, size, self.format, msaa_count);
      self.pipeline = pipeline(
        device,
        self.format,
        &self.uniform_layout,
        primitive_layout,
        msaa_count,
      );
    }
  }

  pub fn resize(
    &mut self,
    size: DeviceSize,
    coordinate_matrix: &wgpu::Buffer,
    device: &wgpu::Device,
  ) {
    self.uniform = uniform_bind_group(device, &self.uniform_layout, coordinate_matrix);
    self.multisample_framebuffer =
      multisample_framebuffer(device, size, self.format, self.anti_aliasing as u32);
  }

  pub fn color_attachments<'a>(
    &'a self,
    view: &'a wgpu::TextureView,
  ) -> wgpu::RenderPassColorAttachment<'a> {
    let (view, resolve_target, store) = match self.anti_aliasing {
      AntiAliasing::None => (view, None, true),
      _ => (&self.multisample_framebuffer, Some(view), false),
    };
    let ops = wgpu::Operations {
      load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
      store,
    };
    wgpu::RenderPassColorAttachment { view, resolve_target, ops }
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

fn multisample_framebuffer(
  device: &wgpu::Device,
  size: DeviceSize,
  format: wgpu::TextureFormat,
  sample_count: u32,
) -> wgpu::TextureView {
  let multisampled_texture_extent = wgpu::Extent3d {
    width: size.width,
    height: size.height,
    depth_or_array_layers: 1,
  };

  let multisampled_frame_descriptor = &wgpu::TextureDescriptor {
    size: multisampled_texture_extent,
    mip_level_count: 1,
    sample_count,
    dimension: wgpu::TextureDimension::D2,
    format,
    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
    label: None,
  };

  device
    .create_texture(multisampled_frame_descriptor)
    .create_view(&wgpu::TextureViewDescriptor::default())
}

fn pipeline(
  device: &wgpu::Device,
  format: wgpu::TextureFormat,
  uniform_layout: &wgpu::BindGroupLayout,
  primitive_layout: &wgpu::BindGroupLayout,
  count: u32,
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
