use std::{mem::size_of, ops::Range};

use ribir_geom::DeviceRect;
use ribir_painter::{AntiAliasing, Vertex, VertexBuffers};
use wgpu::include_wgsl;

use super::vertex_buffer::VerticesBuffer;
use crate::WgpuTexture;

pub struct DrawAlphaTrianglesPass {
  anti_aliasing: AntiAliasing,
  vertices_buffer: VerticesBuffer<f32>,
  pipeline: Option<wgpu::RenderPipeline>,
  shader: wgpu::ShaderModule,
}

impl DrawAlphaTrianglesPass {
  pub fn new(device: &wgpu::Device) -> Self {
    let vertices_buffer = VerticesBuffer::new(2048, 4096, device);
    let shader = device.create_shader_module(include_wgsl!("./shaders/alpha_triangles.wgsl"));

    Self { anti_aliasing: AntiAliasing::None, vertices_buffer, pipeline: None, shader }
  }

  pub fn load_alpha_vertices(
    &mut self, buffers: &VertexBuffers<f32>, device: &wgpu::Device, queue: &wgpu::Queue,
  ) {
    self
      .vertices_buffer
      .write_buffer(buffers, device, queue);
  }

  pub fn draw_alpha_triangles(
    &mut self, indices: &Range<u32>, texture: &WgpuTexture, scissor: Option<DeviceRect>,
    encoder: &mut wgpu::CommandEncoder, device: &wgpu::Device,
  ) {
    self.update_pipeline(texture.anti_aliasing, device);

    let color_attachments = texture.color_attachments(None);

    let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
      label: Some("Alpha triangles render pass"),
      color_attachments: &[Some(color_attachments)],
      depth_stencil_attachment: None,
      timestamp_writes: None,
      occlusion_query_set: None,
    });
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
    rpass.set_pipeline(self.pipeline.as_ref().unwrap());
    rpass.draw_indexed(indices.clone(), 0, 0..1)
  }

  fn update_pipeline(&mut self, anti_aliasing: AntiAliasing, device: &wgpu::Device) {
    if self.anti_aliasing != anti_aliasing {
      self.pipeline.take();
    }

    self.anti_aliasing = anti_aliasing;

    if self.pipeline.is_none() {
      let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Alpha triangles pipeline"),
        layout: None,
        vertex: wgpu::VertexState {
          module: &self.shader,
          entry_point: "vs_main",
          buffers: &[wgpu::VertexBufferLayout {
            array_stride: size_of::<Vertex<f32>>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
              wgpu::VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: wgpu::VertexFormat::Float32x2,
              },
              wgpu::VertexAttribute {
                offset: 8,
                shader_location: 1,
                format: wgpu::VertexFormat::Float32,
              },
            ],
          }],
          compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
          module: &self.shader,
          entry_point: "fs_main",
          targets: &[Some(wgpu::ColorTargetState {
            format: wgpu::TextureFormat::R8Unorm,
            blend: Some(wgpu::BlendState::REPLACE),
            write_mask: wgpu::ColorWrites::RED,
          })],
          compilation_options: Default::default(),
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
      });
      self.pipeline = Some(pipeline)
    }
  }
}
