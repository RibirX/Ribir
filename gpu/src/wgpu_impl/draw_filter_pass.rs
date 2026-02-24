use std::{mem::size_of, ops::Range};

use ribir_painter::{Color, Vertex, VertexBuffers};

use super::{primitive_pool::PrimitivePoolMode, uniform::Uniform, vertex_buffer::VerticesBuffer};
use crate::{DrawPhaseLimits, MaskLayer, WgpuTexture, wgpu_impl::shaders::filter_triangles_shader};

pub struct DrawFilterPass {
  vertices_buffer: VerticesBuffer<()>,
  layout: wgpu::PipelineLayout,
  pipeline: Option<wgpu::RenderPipeline>,
  shader: wgpu::ShaderModule,
  format: Option<wgpu::TextureFormat>,
  origin_tex_layout: wgpu::BindGroupLayout,
  current_range: (Range<wgpu::BufferAddress>, Range<wgpu::BufferAddress>),
}

impl DrawFilterPass {
  pub fn new(
    device: &wgpu::Device, mask_layout: &wgpu::BindGroupLayout,
    texs_layout: &wgpu::BindGroupLayout, slot0_layout: &wgpu::BindGroupLayout,
    pool_mode: PrimitivePoolMode, limits: &DrawPhaseLimits,
  ) -> Self {
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
      bind_group_layouts: &[mask_layout, texs_layout, slot0_layout, &origin_tex_layout],
      immediate_size: 0,
    });

    let vertices_buffer = VerticesBuffer::new(128, 512, device);
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
      label: Some("filter triangles shader"),
      source: wgpu::ShaderSource::Wgsl(filter_triangles_shader(limits, pool_mode).into()),
    });

    Self {
      vertices_buffer,
      layout,
      pipeline: None,
      shader,
      format: None,
      origin_tex_layout,
      current_range: (0..0, 0..0),
    }
  }

  pub fn reset(&mut self) { self.vertices_buffer.reset(); }

  pub fn load_triangles_vertices(
    &mut self, buffers: &VertexBuffers<()>, device: &wgpu::Device, queue: &wgpu::Queue,
  ) -> Option<()> {
    self.current_range = self
      .vertices_buffer
      .write_buffer(buffers, device, queue)?;
    Some(())
  }

  #[allow(clippy::too_many_arguments)]
  pub fn draw_triangles(
    &mut self, output: &WgpuTexture, origin: &WgpuTexture, indices: Range<u32>,
    clear: Option<Color>, device: &wgpu::Device, encoder: &mut wgpu::CommandEncoder,
    textures_bind: &wgpu::BindGroup, mask_layer_storage: &Uniform<MaskLayer>,
    slot0_bind: &wgpu::BindGroup, mask_layer_offset: u32, prims_offset: u32,
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
      rpass.set_bind_group(2, slot0_bind, &[prims_offset]);
      rpass.set_bind_group(3, &bind_group, &[]);
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
