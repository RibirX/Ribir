use std::{mem::size_of, ops::Range};

use ribir_painter::{Color, Vertex, VertexBuffers};

use super::{storage::Storage, vertex_buffer::VerticesBuffer};
use crate::{
  GradientStopPrimitive, MaskLayer, RadialGradientPrimIndex, RadialGradientPrimitive, WgpuTexture,
};

pub struct DrawRadialGradientTrianglesPass {
  vertices_buffer: VerticesBuffer<RadialGradientPrimIndex>,
  pipeline: Option<wgpu::RenderPipeline>,
  shader: wgpu::ShaderModule,
  format: Option<wgpu::TextureFormat>,
  prims_storage: Storage<RadialGradientPrimitive>,
  stops_storage: Storage<GradientStopPrimitive>,
  layout: wgpu::PipelineLayout,
}

impl DrawRadialGradientTrianglesPass {
  pub fn new(
    device: &wgpu::Device, mask_layout: &wgpu::BindGroupLayout, texs_layout: &wgpu::BindGroupLayout,
  ) -> Self {
    let vertices_buffer = VerticesBuffer::new(512, 1024, device);
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
      label: Some("Radial triangles pass"),
      source: wgpu::ShaderSource::Wgsl(
        include_str!("./shaders/radial_gradient_triangles.wgsl").into(),
      ),
    });
    let prims_storage = Storage::new(device, wgpu::ShaderStages::FRAGMENT, 64);
    let stops_storage = Storage::new(device, wgpu::ShaderStages::FRAGMENT, 64);
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
      label: Some("update triangles pipeline layout"),
      bind_group_layouts: &[
        mask_layout,
        stops_storage.layout(),
        prims_storage.layout(),
        texs_layout,
      ],
      push_constant_ranges: &[],
    });

    Self {
      vertices_buffer,
      pipeline: None,
      shader,
      format: None,
      prims_storage,
      stops_storage,
      layout,
    }
  }

  pub fn load_triangles_vertices(
    &mut self, buffers: &VertexBuffers<RadialGradientPrimIndex>, device: &wgpu::Device,
    queue: &wgpu::Queue,
  ) {
    self
      .vertices_buffer
      .write_buffer(buffers, device, queue);
  }

  pub fn load_radial_gradient_primitives(
    &mut self, device: &wgpu::Device, queue: &wgpu::Queue, primitives: &[RadialGradientPrimitive],
  ) {
    self
      .prims_storage
      .write_buffer(device, queue, primitives);
  }

  pub fn load_gradient_stops(
    &mut self, device: &wgpu::Device, queue: &wgpu::Queue, stops: &[GradientStopPrimitive],
  ) {
    self
      .stops_storage
      .write_buffer(device, queue, stops);
  }

  #[allow(clippy::too_many_arguments)]
  pub fn draw_triangles(
    &mut self, texture: &WgpuTexture, indices: Range<u32>, clear: Option<Color>,
    device: &wgpu::Device, encoder: &mut wgpu::CommandEncoder, textures_bind: &wgpu::BindGroup,
    mask_layer_storage: &Storage<MaskLayer>,
  ) {
    self.update(texture.format(), device);
    let pipeline = self.pipeline.as_ref().unwrap();

    let color_attachments = texture.color_attachments(clear);
    let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
      label: Some("Radial triangles render pass"),
      color_attachments: &[Some(color_attachments)],
      depth_stencil_attachment: None,
      timestamp_writes: None,
      occlusion_query_set: None,
    });

    rpass.set_vertex_buffer(0, self.vertices_buffer.vertices().slice(..));
    rpass.set_index_buffer(self.vertices_buffer.indices().slice(..), wgpu::IndexFormat::Uint32);
    rpass.set_bind_group(0, mask_layer_storage.bind_group(), &[]);
    rpass.set_bind_group(1, self.stops_storage.bind_group(), &[]);
    rpass.set_bind_group(2, self.prims_storage.bind_group(), &[]);
    rpass.set_bind_group(3, textures_bind, &[]);

    rpass.set_pipeline(pipeline);
    rpass.draw_indexed(indices, 0, 0..1);
  }

  fn update(&mut self, format: wgpu::TextureFormat, device: &wgpu::Device) {
    if self.format != Some(format) {
      self.pipeline.take();
      self.format = Some(format);
    }

    if self.pipeline.is_none() {
      let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Radial triangles pipeline"),
        layout: Some(&self.layout),
        vertex: wgpu::VertexState {
          module: &self.shader,
          entry_point: "vs_main",
          buffers: &[wgpu::VertexBufferLayout {
            array_stride: size_of::<Vertex<RadialGradientPrimIndex>>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
              // position
              wgpu::VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: wgpu::VertexFormat::Float32x2,
              },
              // prim_idx
              wgpu::VertexAttribute {
                offset: 8,
                shader_location: 1,
                format: wgpu::VertexFormat::Uint32,
              },
            ],
          }],
          compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
          module: &self.shader,
          entry_point: "fs_main",
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
        multiview: None,
      });
      self.pipeline = Some(pipeline);
    }
  }
}
