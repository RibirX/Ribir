use std::{mem::size_of, ops::Range};

use ribir_painter::{Color, Vertex, VertexBuffers};

use super::{shaders::color_triangles_shader, uniform::Uniform, vertex_buffer::VerticesBuffer};
use crate::{ColorAttr, MaskLayer, WgpuTexture};

pub struct DrawColorTrianglesPass {
  vertices_buffer: VerticesBuffer<ColorAttr>,
  layout: wgpu::PipelineLayout,
  pipeline: Option<wgpu::RenderPipeline>,
  shader: wgpu::ShaderModule,
  format: Option<wgpu::TextureFormat>,
  current_range: (Range<wgpu::BufferAddress>, Range<wgpu::BufferAddress>),
}

impl DrawColorTrianglesPass {
  pub fn new(
    device: &wgpu::Device, mask_layout: &wgpu::BindGroupLayout,
    texs_layout: &wgpu::BindGroupLayout, slot0_layout: &wgpu::BindGroupLayout,
    slot1_layout: &wgpu::BindGroupLayout, max_mask_layers: usize,
  ) -> Self {
    let vertices_buffer = VerticesBuffer::new(512, 1024, device);

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
      label: Some("Color triangles shader"),
      source: wgpu::ShaderSource::Wgsl(color_triangles_shader(max_mask_layers).into()),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
      label: Some("Color triangles pipeline layout"),
      bind_group_layouts: &[mask_layout, texs_layout, slot0_layout, slot1_layout],
      immediate_size: 0,
    });

    Self {
      layout,
      vertices_buffer,
      pipeline: None,
      shader,
      format: None,
      current_range: (0..0, 0..0),
    }
  }

  pub fn reset(&mut self) { self.vertices_buffer.reset(); }

  pub fn load_triangles_vertices(
    &mut self, buffers: &VertexBuffers<ColorAttr>, device: &wgpu::Device, queue: &wgpu::Queue,
  ) -> Option<()> {
    self.current_range = self
      .vertices_buffer
      .write_buffer(buffers, device, queue)?;
    Some(())
  }

  #[allow(clippy::too_many_arguments)]
  pub fn draw_triangles(
    &mut self, texture: &WgpuTexture, indices: Range<u32>, clear: Option<Color>,
    device: &wgpu::Device, encoder: &mut wgpu::CommandEncoder, textures_bind: &wgpu::BindGroup,
    mask_layer_storage: &Uniform<MaskLayer>, slot0_bind: &wgpu::BindGroup,
    slot1_bind: &wgpu::BindGroup, mask_layer_offset: u32,
  ) {
    self.update(texture.format(), device);
    let pipeline = self.pipeline.as_ref().unwrap();

    let color_attachments = texture.color_attachments(clear);
    let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
      label: Some("Color triangles pass"),
      color_attachments: &[Some(color_attachments)],
      depth_stencil_attachment: None,
      timestamp_writes: None,
      occlusion_query_set: None,
      multiview_mask: None,
    });

    rpass.set_pipeline(pipeline);
    if !indices.is_empty() {
      rpass.set_vertex_buffer(
        0,
        self
          .vertices_buffer
          .vertices()
          .slice(self.current_range.0.clone()),
      );
      rpass.set_index_buffer(
        self
          .vertices_buffer
          .indices()
          .slice(self.current_range.1.clone()),
        wgpu::IndexFormat::Uint32,
      );
      rpass.set_bind_group(0, mask_layer_storage.bind_group(), &[mask_layer_offset]);
      rpass.set_bind_group(1, textures_bind, &[]);
      rpass.set_bind_group(2, slot0_bind, &[0]);
      rpass.set_bind_group(3, slot1_bind, &[0]);
      rpass.draw_indexed(indices, 0, 0..1);
    }
  }

  fn update(&mut self, format: wgpu::TextureFormat, device: &wgpu::Device) {
    if self.format != Some(format) {
      self.pipeline.take();
      self.format = Some(format);
    }

    if self.pipeline.is_none() {
      let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Color triangles pipeline"),
        layout: Some(&self.layout),
        vertex: wgpu::VertexState {
          module: &self.shader,
          entry_point: Some("vs_main"),
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
          compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
          module: &self.shader,
          entry_point: Some("fs_main"),
          targets: &[Some(wgpu::ColorTargetState {
            format,
            blend: Some(wgpu::BlendState::ALPHA_BLENDING),
            write_mask: wgpu::ColorWrites::all(),
          })],
          compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
          topology: wgpu::PrimitiveTopology::TriangleList,
          strip_index_format: None,
          front_face: wgpu::FrontFace::Ccw,
          // Always draw rect with transform, there is no distinction between front and back,
          // everything needs to be drawn.
          cull_mode: None,
          unclipped_depth: false,
          polygon_mode: wgpu::PolygonMode::Fill,
          conservative: false,
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState {
          count: 1,
          mask: !0,
          alpha_to_coverage_enabled: false,
        },
        multiview_mask: None,
        cache: None,
      });
      self.pipeline = Some(pipeline);
    }
  }
}
