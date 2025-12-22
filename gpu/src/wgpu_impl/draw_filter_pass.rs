use std::{mem::size_of, ops::Range};

use ribir_painter::{Color, Vertex, VertexBuffers};
use zerocopy::AsBytes;

use super::{uniform::Uniform, vertex_buffer::VerticesBuffer};
use crate::{
  DrawPhaseLimits, FilterPrimitive, MaskLayer, WgpuTexture,
  wgpu_impl::{shaders::filter_triangles_shader, uniform::UniformVar},
};

pub struct DrawFilterPass {
  vertices_buffer: VerticesBuffer<()>,
  layout: wgpu::PipelineLayout,
  pipeline: Option<wgpu::RenderPipeline>,
  shader: wgpu::ShaderModule,
  prims_uniform: UniformVar,
  format: Option<wgpu::TextureFormat>,
  origin_tex_layout: wgpu::BindGroupLayout,
}

impl DrawFilterPass {
  pub fn new(
    device: &wgpu::Device, mask_layout: &wgpu::BindGroupLayout,
    texs_layout: &wgpu::BindGroupLayout, limits: &DrawPhaseLimits,
  ) -> Self {
    let prims_uniform = UniformVar::new(
      device,
      wgpu::ShaderStages::FRAGMENT,
      size_of::<FilterPrimitive>() + limits.max_filter_matrix_len * size_of::<f32>(),
    );

    let origin_tex_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
      entries: &[wgpu::BindGroupLayoutEntry {
        binding: 0,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
          sample_type: wgpu::TextureSampleType::Float { filterable: true },
          view_dimension: wgpu::TextureViewDimension::D2,
          multisampled: false,
        },
        count: None,
      }],
      label: Some("Textures layout"),
    });

    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
      label: Some("Filter pipeline layout"),
      bind_group_layouts: &[mask_layout, texs_layout, prims_uniform.layout(), &origin_tex_layout],
      immediate_size: 0,
    });

    let vertices_buffer = VerticesBuffer::new(128, 512, device);
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
      label: Some("filter triangles shader"),
      source: wgpu::ShaderSource::Wgsl(filter_triangles_shader(limits).into()),
    });

    Self {
      vertices_buffer,
      layout,
      pipeline: None,
      shader,
      prims_uniform,
      format: None,
      origin_tex_layout,
    }
  }

  pub fn load_triangles_vertices(
    &mut self, buffers: &VertexBuffers<()>, device: &wgpu::Device, queue: &wgpu::Queue,
  ) {
    self
      .vertices_buffer
      .write_buffer(buffers, device, queue);
  }

  pub fn load_filter_primitive(
    &mut self, queue: &wgpu::Queue, primitive: &FilterPrimitive, matrix: &[f32],
  ) {
    self
      .prims_uniform
      .write_buffer(queue, 0, primitive.as_bytes());
    self
      .prims_uniform
      .write_buffer(queue, size_of::<FilterPrimitive>(), matrix.as_bytes());
  }

  #[allow(clippy::too_many_arguments)]
  pub fn draw_triangles(
    &mut self, output: &WgpuTexture, origin: &WgpuTexture, indices: Range<u32>,
    clear: Option<Color>, device: &wgpu::Device, encoder: &mut wgpu::CommandEncoder,
    textures_bind: &wgpu::BindGroup, mask_layer_storage: &Uniform<MaskLayer>,
  ) {
    self.update(output.format(), device);
    let pipeline = self.pipeline.as_ref().unwrap();
    let color_attachments = output.color_attachments(clear);
    let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
      label: Some("filter render pass"),
      color_attachments: &[Some(color_attachments)],
      depth_stencil_attachment: None,
      timestamp_writes: None,
      occlusion_query_set: None,
      multiview_mask: None,
    });

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
      layout: &self.origin_tex_layout,
      entries: &[wgpu::BindGroupEntry {
        binding: 0,
        resource: wgpu::BindingResource::TextureView(origin.view()),
      }],
      label: Some("filter origin texture bind group"),
    });

    rpass.set_vertex_buffer(0, self.vertices_buffer.vertices().slice(..));
    rpass.set_index_buffer(self.vertices_buffer.indices().slice(..), wgpu::IndexFormat::Uint32);
    rpass.set_bind_group(0, mask_layer_storage.bind_group(), &[]);
    rpass.set_bind_group(1, textures_bind, &[]);
    rpass.set_bind_group(2, self.prims_uniform.bind_group(), &[]);
    rpass.set_bind_group(3, &bind_group, &[]);

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
        label: Some("Filter triangles pipeline"),
        layout: Some(&self.layout),
        vertex: wgpu::VertexState {
          module: &self.shader,
          entry_point: Some("vs_main"),
          buffers: &[wgpu::VertexBufferLayout {
            array_stride: size_of::<Vertex<()>>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
              // position
              wgpu::VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: wgpu::VertexFormat::Float32x2,
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
            blend: Some(wgpu::BlendState::REPLACE),
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
