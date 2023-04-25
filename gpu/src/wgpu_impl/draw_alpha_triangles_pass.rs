use crate::WgpuImpl;
use ribir_painter::{AntiAliasing, DeviceRect, Vertex, VertexBuffers};
use std::{mem::size_of, ops::Range};
use zerocopy::AsBytes;

struct AlphaMultiSample {
  texture: wgpu::Texture,
  view: wgpu::TextureView,
  anti_aliasing: AntiAliasing,
}

pub struct DrawAlphaTrianglesPass {
  anti_aliasing: AntiAliasing,
  alpha_vertices_buffer: wgpu::Buffer,
  alpha_indices_buffer: wgpu::Buffer,
  pipeline: wgpu::RenderPipeline,
  shader: wgpu::ShaderModule,
  layout: wgpu::PipelineLayout,
  alpha_multisample: Option<AlphaMultiSample>,
}

impl DrawAlphaTrianglesPass {
  pub fn new(
    anti_aliasing: AntiAliasing,
    device: &wgpu::Device,
    coordinate_layout: &wgpu::BindGroupLayout,
  ) -> Self {
    let alpha_vertices_buffer = WgpuImpl::new_vertices::<Vertex<()>>(device, 1024);
    let alpha_indices_buffer = WgpuImpl::new_indices(device, 1024);
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
      label: Some("Alpha triangles"),
      source: wgpu::ShaderSource::Wgsl(include_str!("./shaders/alpha_triangles.wgsl").into()),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
      label: Some("Alpha triangles pipeline layout"),
      bind_group_layouts: &[coordinate_layout],
      push_constant_ranges: &[],
    });

    let pipeline = Self::pipeline(device, anti_aliasing, &shader, &layout);
    Self {
      anti_aliasing,
      alpha_vertices_buffer,
      alpha_indices_buffer,
      alpha_multisample: None,
      pipeline,
      shader,
      layout,
    }
  }
  pub fn set_anti_aliasing(&mut self, anti_aliasing: AntiAliasing, device: &wgpu::Device) {
    if self.anti_aliasing != anti_aliasing {
      self.anti_aliasing = anti_aliasing;
      let Self { pipeline, shader, layout, .. } = self;
      *pipeline = Self::pipeline(device, anti_aliasing, shader, layout);
    }
  }

  pub fn load_alpha_vertices(
    &mut self,
    buffers: &VertexBuffers<()>,
    device: &wgpu::Device,
    queue: &mut wgpu::Queue,
  ) {
    let VertexBuffers { vertices, indices } = buffers;
    let v_buffer_len = self.alpha_vertices_buffer.size() as usize / size_of::<Vertex<()>>();
    if v_buffer_len < vertices.len() {
      self.alpha_vertices_buffer = WgpuImpl::new_vertices::<Vertex<()>>(device, vertices.len());
    }
    queue.write_buffer(&self.alpha_vertices_buffer, 0, vertices.as_bytes());

    let i_buffer_len = self.alpha_indices_buffer.size() as usize / size_of::<Vertex<()>>();
    if i_buffer_len < indices.len() {
      self.alpha_indices_buffer = WgpuImpl::new_indices(device, indices.len());
    }
    queue.write_buffer(&self.alpha_indices_buffer, 0, indices.as_bytes());
  }

  fn pipeline(
    device: &wgpu::Device,
    anti_aliasing: AntiAliasing,
    shader: &wgpu::ShaderModule,
    layout: &wgpu::PipelineLayout,
  ) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
      label: Some("Alpha triangles pipeline"),
      layout: Some(layout),
      vertex: wgpu::VertexState {
        module: shader,
        entry_point: "vs_main",
        buffers: &[wgpu::VertexBufferLayout {
          array_stride: size_of::<Vertex<()>>() as wgpu::BufferAddress,
          step_mode: wgpu::VertexStepMode::Vertex,
          attributes: &[wgpu::VertexAttribute {
            offset: 0,
            shader_location: 0,
            format: wgpu::VertexFormat::Float32x2,
          }],
        }],
      },
      fragment: Some(wgpu::FragmentState {
        module: shader,
        entry_point: "fs_main",
        targets: &[Some(wgpu::ColorTargetState {
          format: wgpu::TextureFormat::R8Unorm,
          blend: Some(wgpu::BlendState::ALPHA_BLENDING),
          write_mask: wgpu::ColorWrites::all(),
        })],
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
        count: anti_aliasing as u32,
        mask: !0,
        alpha_to_coverage_enabled: false,
      },
      multiview: None,
    })
  }

  pub fn draw_alpha_triangles(
    &mut self,
    indices: &Range<u32>,
    texture: &mut wgpu::Texture,
    scissor: Option<DeviceRect>,
    encoder: &mut wgpu::CommandEncoder,
    coordinate_bind: &wgpu::BindGroup,
    device: &wgpu::Device,
  ) {
    self.update_alpha_multi_sample(&texture, device);

    let view = texture.create_view(&<_>::default());
    let color_attachments = if let Some(multi_sample) = self.alpha_multisample.as_ref() {
      wgpu::RenderPassColorAttachment {
        view: &multi_sample.view,
        resolve_target: Some(&view),
        ops: wgpu::Operations {
          load: wgpu::LoadOp::Load,
          store: false,
        },
      }
    } else {
      wgpu::RenderPassColorAttachment {
        view: &view,
        resolve_target: None,
        ops: wgpu::Operations {
          load: wgpu::LoadOp::Load,
          store: true,
        },
      }
    };

    let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
      label: Some("Alpha triangles render pass"),
      color_attachments: &[Some(color_attachments)],
      depth_stencil_attachment: None,
    });
    rpass.set_vertex_buffer(0, self.alpha_vertices_buffer.slice(..));
    rpass.set_index_buffer(
      self.alpha_indices_buffer.slice(..),
      wgpu::IndexFormat::Uint32,
    );

    rpass.set_bind_group(0, coordinate_bind, &[]);
    if let Some(scissor) = scissor {
      rpass.set_scissor_rect(
        scissor.min_x() as u32,
        scissor.min_y() as u32,
        scissor.width() as u32,
        scissor.height() as u32,
      );
    }
    rpass.set_pipeline(&self.pipeline);
    rpass.draw_indexed(indices.clone(), 0, 0..1)
  }

  fn update_alpha_multi_sample(&mut self, target: &wgpu::Texture, device: &wgpu::Device) {
    if self.anti_aliasing == AntiAliasing::None {
      self.alpha_multisample.take();
      return;
    }

    if let Some(sample) = self.alpha_multisample.as_ref() {
      if sample.anti_aliasing == self.anti_aliasing && sample.texture.size() == target.size() {
        return;
      }
    }

    self.alpha_multisample.take();

    let sample_count = self.anti_aliasing as u32;
    let multisample_frame_descriptor = &wgpu::TextureDescriptor {
      size: wgpu::Extent3d {
        width: target.width(),
        height: target.height(),
        depth_or_array_layers: 1,
      },
      mip_level_count: 1,
      sample_count,
      dimension: wgpu::TextureDimension::D2,
      format: wgpu::TextureFormat::R8Unorm,
      usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
      label: None,
      view_formats: &[],
    };

    let texture = device.create_texture(multisample_frame_descriptor);
    let view = texture.create_view(&<_>::default());
    self.alpha_multisample = Some(AlphaMultiSample {
      texture,
      view,
      anti_aliasing: self.anti_aliasing,
    })
  }
}
