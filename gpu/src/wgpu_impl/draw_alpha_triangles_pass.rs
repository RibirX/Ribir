use std::{mem::size_of, ops::Range};

use ribir_geom::DeviceRect;
use ribir_painter::{Vertex, VertexBuffers};
use wgpu::include_wgsl;

use super::{uniform::Uniform, vertex_buffer::VerticesBuffer};
use crate::WgpuTexture;

pub const SAMPLE_COUNT: u32 = 6;

pub struct DrawAlphaTrianglesPass {
  vertices_buffer: VerticesBuffer<()>,
  pipeline: wgpu::RenderPipeline,
  size_uniform: Uniform<u32>,
}

impl DrawAlphaTrianglesPass {
  pub fn new(device: &wgpu::Device) -> Self {
    let vertices_buffer = VerticesBuffer::new(2048, 4096, device);
    let shader = device.create_shader_module(include_wgsl!("./shaders/alpha_triangles.wgsl"));
    // Although we only need 2 x u32, we use 4 x f32 to align with the 16-byte
    // uniform buffer. This is because WebGL requires the buffer to be 16-byte
    // aligned.
    let size_uniform = Uniform::new(device, wgpu::ShaderStages::VERTEX, 4);
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
      label: Some("Alpha triangles pipeline layout"),
      bind_group_layouts: &[size_uniform.layout()],
      push_constant_ranges: &[],
    });

    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
      label: Some("Alpha triangles pipeline"),
      layout: Some(&layout),
      vertex: wgpu::VertexState {
        module: &shader,
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
        compilation_options: Default::default(),
      },
      fragment: Some(wgpu::FragmentState {
        module: &shader,
        entry_point: "fs_main",
        targets: &[Some(wgpu::ColorTargetState {
          format: wgpu::TextureFormat::R8Unorm,
          blend: Some(wgpu::BlendState {
            color: wgpu::BlendComponent {
              src_factor: wgpu::BlendFactor::One,
              dst_factor: wgpu::BlendFactor::One,
              operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent::REPLACE,
          }),
          write_mask: wgpu::ColorWrites::RED,
        })],
        compilation_options: Default::default(),
      }),
      primitive: wgpu::PrimitiveState {
        topology: wgpu::PrimitiveTopology::TriangleList,
        strip_index_format: None,
        front_face: wgpu::FrontFace::Ccw,
        cull_mode: None,
        unclipped_depth: false,
        polygon_mode: wgpu::PolygonMode::Fill,
        conservative: false,
      },
      depth_stencil: None,
      multisample: wgpu::MultisampleState { count: 1, mask: !0, alpha_to_coverage_enabled: false },
      multiview: None,
    });

    Self { vertices_buffer, pipeline, size_uniform }
  }

  pub fn load_alpha_vertices(
    &mut self, buffers: &VertexBuffers<()>, device: &wgpu::Device, queue: &wgpu::Queue,
  ) {
    self
      .vertices_buffer
      .write_buffer(buffers, device, queue);
  }

  pub fn draw_alpha_triangles(
    &mut self, indices: &Range<u32>, texture: &WgpuTexture, scissor: Option<DeviceRect>,
    queue: &wgpu::Queue, encoder: &mut wgpu::CommandEncoder,
  ) {
    let color_attachments = texture.color_attachments(None);
    self
      .size_uniform
      .write_buffer(queue, &texture.size().to_u32().to_array());

    let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
      label: Some("Alpha triangles render pass"),
      color_attachments: &[Some(color_attachments)],
      depth_stencil_attachment: None,
      timestamp_writes: None,
      occlusion_query_set: None,
    });
    rpass.set_bind_group(0, self.size_uniform.bind_group(), &[]);
    rpass.set_vertex_buffer(0, self.vertices_buffer.vertices().slice(..));
    rpass.set_index_buffer(self.vertices_buffer.indices().slice(..), wgpu::IndexFormat::Uint32);

    if let Some(scissor) = scissor {
      rpass.set_scissor_rect(
        scissor.min_x() as u32,
        scissor.min_y() as u32,
        scissor.width() as u32,
        scissor.height() as u32,
      );
    }
    rpass.set_pipeline(&self.pipeline);
    rpass.draw_indexed(indices.clone(), 0, 0..SAMPLE_COUNT)
  }
}
