use super::{storage::Storage, vertex_buffer::VerticesBuffer};
use crate::{ColorAttr, MaskLayer, TexturesBind, WgpuTexture};
use ribir_painter::{AntiAliasing, Color, Vertex, VertexBuffers};
use std::{mem::size_of, ops::Range};

pub struct DrawColorTrianglesPass {
  label: &'static str,
  vertices_buffer: VerticesBuffer<ColorAttr>,
  pipeline: Option<wgpu::RenderPipeline>,
  shader: wgpu::ShaderModule,
  format: Option<wgpu::TextureFormat>,
  textures_count: usize,
  anti_aliasing: AntiAliasing,
}

impl DrawColorTrianglesPass {
  pub fn new(device: &wgpu::Device) -> Self {
    let vertices_buffer = VerticesBuffer::new(512, 1024, device);
    let label = "Color triangles pass";
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
      label: Some(label),
      source: wgpu::ShaderSource::Wgsl(include_str!("./shaders/color_triangles.wgsl").into()),
    });

    Self {
      label,
      vertices_buffer,
      pipeline: None,
      shader,
      format: None,
      textures_count: 0,
      anti_aliasing: AntiAliasing::None,
    }
  }

  pub fn load_triangles_vertices(
    &mut self,
    buffers: &VertexBuffers<ColorAttr>,
    device: &wgpu::Device,
    queue: &mut wgpu::Queue,
  ) {
    self.vertices_buffer.write_buffer(buffers, device, queue);
  }

  #[allow(clippy::too_many_arguments)]
  pub fn draw_triangles(
    &mut self,
    texture: &mut WgpuTexture,
    indices: Range<u32>,
    clear: Option<Color>,
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    textures_bind: &TexturesBind,
    mask_layer_storage: &Storage<MaskLayer>,
  ) {
    self.update(
      texture.format(),
      texture.anti_aliasing,
      device,
      textures_bind,
      mask_layer_storage.layout(),
    );
    let pipeline = self.pipeline.as_ref().unwrap();

    let color_attachments = texture.color_attachments(clear);
    let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
      label: Some(self.label),
      color_attachments: &[Some(color_attachments)],
      depth_stencil_attachment: None,
    });

    rpass.set_vertex_buffer(0, self.vertices_buffer.vertices().slice(..));
    rpass.set_index_buffer(
      self.vertices_buffer.indices().slice(..),
      wgpu::IndexFormat::Uint32,
    );
    rpass.set_bind_group(0, mask_layer_storage.bind_group(), &[]);
    rpass.set_bind_group(1, textures_bind.assert_bind(), &[]);

    rpass.set_pipeline(pipeline);
    rpass.draw_indexed(indices, 0, 0..1);
  }

  fn update(
    &mut self,
    format: wgpu::TextureFormat,
    anti_aliasing: AntiAliasing,
    device: &wgpu::Device,
    textures_bind: &TexturesBind,
    mask_bind_layout: &wgpu::BindGroupLayout,
  ) {
    if self.format != Some(format)
      || textures_bind.textures_count() != self.textures_count
      || anti_aliasing != self.anti_aliasing
    {
      self.pipeline.take();
      self.format = Some(format);
      self.textures_count = textures_bind.textures_count();
      self.anti_aliasing = anti_aliasing;
    }

    if self.pipeline.is_none() {
      let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("update triangles pipeline layout"),
        bind_group_layouts: &[mask_bind_layout, textures_bind.assert_layout()],
        push_constant_ranges: &[],
      });
      let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(self.label),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
          module: &self.shader,
          entry_point: "vs_main",
          buffers: &[wgpu::VertexBufferLayout {
            array_stride: size_of::<Vertex<ColorAttr>>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
              // position
              wgpu::VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: wgpu::VertexFormat::Float32x2,
              },
              // color
              wgpu::VertexAttribute {
                offset: 8,
                shader_location: 1,
                format: wgpu::VertexFormat::Unorm8x4,
              },
              // mask_head
              wgpu::VertexAttribute {
                offset: 12,
                shader_location: 2,
                format: wgpu::VertexFormat::Sint32,
              },
            ],
          }],
        },
        fragment: Some(wgpu::FragmentState {
          module: &self.shader,
          entry_point: "fs_main",
          targets: &[Some(wgpu::ColorTargetState {
            format,
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
      });
      self.pipeline = Some(pipeline);
    }
  }
}
