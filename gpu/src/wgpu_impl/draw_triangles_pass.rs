use crate::{ColorPrimitive, IndicesRange, TexturePrimitive, WgpuImpl};
use ribir_painter::{DeviceRect, Vertex, VertexBuffers};
use std::{mem::size_of, num::NonZeroU32};
use zerocopy::AsBytes;

pub struct TrianglesPipeline {
  label: &'static str,
  pipeline: Option<wgpu::RenderPipeline>,
  shader: wgpu::ShaderModule,
  format: Option<wgpu::TextureFormat>,
  layout_addr: *const wgpu::PipelineLayout,
}

pub struct DrawTrianglesPass {
  primitives_layout: wgpu::BindGroupLayout,
  pipeline_layout: wgpu::PipelineLayout,

  color_primitives_buffer: wgpu::Buffer,
  color_primitives_bind: wgpu::BindGroup,
  color_triangles_pipeline: TrianglesPipeline,

  tex_primitives_buffer: wgpu::Buffer,
  tex_primitives_bind: wgpu::BindGroup,
  tex_triangles_pipeline: TrianglesPipeline,

  textures: Vec<wgpu::TextureView>,
  textures_bind: Option<wgpu::BindGroup>,
  textures_layout: Option<wgpu::BindGroupLayout>,
  vertices_buffer: wgpu::Buffer,
  indices_buffer: wgpu::Buffer,
}

impl DrawTrianglesPass {
  pub fn new(device: &wgpu::Device) -> Self {
    let color_primitives_buffer = WgpuImpl::new_storage::<ColorPrimitive>(&device, 256);
    let primitives_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
      entries: &[wgpu::BindGroupLayoutEntry {
        binding: 0,
        visibility: wgpu::ShaderStages::VERTEX,
        ty: wgpu::BindingType::Buffer {
          ty: wgpu::BufferBindingType::Storage { read_only: true },
          has_dynamic_offset: false,
          min_binding_size: None,
        },
        count: None,
      }],
      label: Some("Color primitives storage layout"),
    });

    let color_primitives_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
      layout: &primitives_layout,
      entries: &[wgpu::BindGroupEntry {
        binding: 0,
        resource: color_primitives_buffer.as_entire_binding(),
      }],
      label: Some("Color primitives storage bind group"),
    });

    let tex_primitives_buffer = WgpuImpl::new_storage::<TexturePrimitive>(&device, 256);
    let tex_primitives_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
      layout: &primitives_layout,
      entries: &[wgpu::BindGroupEntry {
        binding: 0,
        resource: tex_primitives_buffer.as_entire_binding(),
      }],
      label: Some("Texture primitives storage bind group"),
    });

    let vertices_buffer = WgpuImpl::new_vertices::<Vertex<u32>>(&device, 512);
    let indices_buffer = WgpuImpl::new_indices(&device, 1024);

    let shader_src = include_str!("./shaders/color_triangles.wgsl");
    let color_triangles_pipeline = TrianglesPipeline::new("Color Triangles", shader_src, &device);
    let shader_src = include_str!("./shaders/tex_triangles.wgsl");
    let tex_triangles_pipeline = TrianglesPipeline::new("Texture Triangles", shader_src, &device);
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
      label: Some("triangles pipeline layout"),
      bind_group_layouts: &[],
      push_constant_ranges: &[],
    });

    Self {
      primitives_layout,
      pipeline_layout,
      color_primitives_buffer,
      color_primitives_bind,
      color_triangles_pipeline,
      tex_primitives_buffer,
      tex_primitives_bind,
      tex_triangles_pipeline,
      textures: vec![],
      textures_bind: None,
      textures_layout: None,
      vertices_buffer,
      indices_buffer,
    }
  }

  pub fn load_color_primitives(
    &mut self,
    primitives: &[ColorPrimitive],
    device: &wgpu::Device,
    queue: &mut wgpu::Queue,
  ) {
    let buffer_len = self.color_primitives_buffer.size() as usize / size_of::<ColorPrimitive>();
    if buffer_len < primitives.len() {
      self.color_primitives_buffer =
        WgpuImpl::new_storage::<ColorPrimitive>(&device, primitives.len());
      self.color_primitives_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &self.primitives_layout,
        entries: &[wgpu::BindGroupEntry {
          binding: 0,
          resource: self.color_primitives_buffer.as_entire_binding(),
        }],
        label: Some("Color primitives storage bind group"),
      });
    }
    queue.write_buffer(&self.color_primitives_buffer, 0, primitives.as_bytes());
  }

  pub fn load_triangles_vertices(
    &mut self,
    buffers: &VertexBuffers<u32>,
    device: &wgpu::Device,
    queue: &mut wgpu::Queue,
  ) {
    let VertexBuffers { vertices, indices } = buffers;
    let v_buffer_len = self.vertices_buffer.size() as usize / size_of::<Vertex<u32>>();
    if v_buffer_len < vertices.len() {
      self.vertices_buffer = WgpuImpl::new_vertices::<Vertex<u32>>(device, vertices.len());
    }
    queue.write_buffer(&self.vertices_buffer, 0, vertices.as_bytes());

    let i_buffer_len = self.indices_buffer.size() as usize / size_of::<u32>();
    if i_buffer_len < indices.len() {
      self.indices_buffer = WgpuImpl::new_indices(device, indices.len());
    }

    queue.write_buffer(&self.indices_buffer, 0, indices.as_bytes());
  }

  pub fn load_texture_primitives(
    &mut self,
    primitives: &[TexturePrimitive],
    device: &wgpu::Device,
    queue: &mut wgpu::Queue,
  ) {
    let buffer_len = self.tex_primitives_buffer.size() as usize / size_of::<TexturePrimitive>();
    if buffer_len < primitives.len() {
      self.tex_primitives_buffer =
        WgpuImpl::new_storage::<TexturePrimitive>(device, primitives.len());
      self.tex_primitives_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &self.primitives_layout,
        entries: &[wgpu::BindGroupEntry {
          binding: 0,
          resource: self.tex_primitives_buffer.as_entire_binding(),
        }],
        label: Some("Texture primitives storage bind group"),
      });
    }

    queue.write_buffer(&self.tex_primitives_buffer, 0, primitives.as_bytes());
  }

  pub fn draw_triangles(
    &mut self,
    texture: &mut wgpu::Texture,
    scissor: &DeviceRect,
    range: IndicesRange,
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    coordinate_bind: &wgpu::BindGroup,
  ) {
    let (prim_bind, pipeline, rg) = match range {
      IndicesRange::Color(rg) => (
        &self.color_primitives_bind,
        &mut self.color_triangles_pipeline,
        rg,
      ),
      IndicesRange::Texture(rg) => (
        &self.tex_primitives_bind,
        &mut self.tex_triangles_pipeline,
        rg,
      ),
      IndicesRange::Gradient(_) => todo!(),
    };

    pipeline.update(texture.format(), device, &self.pipeline_layout);
    let pipeline = pipeline.get();

    let view = texture.create_view(&<_>::default());

    let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
      label: Some("Triangles render pass"),
      color_attachments: &[Some(wgpu::RenderPassColorAttachment {
        view: &view,
        resolve_target: None,
        ops: wgpu::Operations {
          load: wgpu::LoadOp::Load,
          store: true,
        },
      })],
      depth_stencil_attachment: None,
    });
    rpass.set_vertex_buffer(0, self.vertices_buffer.slice(..));
    rpass.set_index_buffer(self.indices_buffer.slice(..), wgpu::IndexFormat::Uint32);
    rpass.set_bind_group(0, coordinate_bind, &[]);
    let texture_group = self
      .textures_bind
      .as_ref()
      .expect("Should load textures before draws!");
    rpass.set_bind_group(1, texture_group, &[]);

    rpass.set_scissor_rect(
      scissor.min_x() as u32,
      scissor.min_y() as u32,
      scissor.width() as u32,
      scissor.height() as u32,
    );

    rpass.set_bind_group(2, prim_bind, &[]);
    rpass.set_pipeline(pipeline);
    rpass.draw_indexed(rg.clone(), 0, 0..1);
  }

  pub(crate) fn load_textures<'a, Iter>(
    &mut self,
    textures: Iter,
    device: &wgpu::Device,
    coordinate_layout: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
  ) where
    Iter: Iterator<Item = &'a wgpu::Texture> + 'a,
  {
    let old_size = self.textures.len();
    self.textures.clear();
    let view_desc = <_>::default();
    for t in textures {
      self.textures.push(t.create_view(&view_desc))
    }

    if self.textures.len() != old_size {
      let texture_size = NonZeroU32::new(self.textures.len() as u32);
      let textures_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        entries: &[
          wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
              sample_type: wgpu::TextureSampleType::Float { filterable: true },
              view_dimension: wgpu::TextureViewDimension::D2,
              multisampled: false,
            },
            count: texture_size,
          },
          wgpu::BindGroupLayoutEntry {
            binding: 1,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            count: texture_size,
          },
        ],
        label: Some("Textures layout"),
      });

      self.pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("update triangles pipeline layout"),
        bind_group_layouts: &[coordinate_layout, &textures_layout, &self.primitives_layout],
        push_constant_ranges: &[],
      });
      self.textures_layout = Some(textures_layout);
    }

    let mut views = Vec::with_capacity(self.textures.len());
    self.textures.iter().for_each(|v| views.push(v));
    let samplers = vec![&*sampler; self.textures.len()];

    let texture_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
      layout: self.textures_layout.as_ref().unwrap(),
      entries: &[
        wgpu::BindGroupEntry {
          binding: 0,
          resource: wgpu::BindingResource::TextureViewArray(&views),
        },
        wgpu::BindGroupEntry {
          binding: 1,
          resource: wgpu::BindingResource::SamplerArray(&samplers),
        },
      ],
      label: Some("color triangles bind group"),
    });

    self.textures_bind = Some(texture_bind);
  }
}

impl TrianglesPipeline {
  fn new(label: &'static str, shader_src: &str, device: &wgpu::Device) -> Self {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
      label: Some(label),
      source: wgpu::ShaderSource::Wgsl(shader_src.into()),
    });

    Self {
      label,
      pipeline: None,
      shader,
      format: Some(wgpu::TextureFormat::Rgba8Unorm),
      layout_addr: std::ptr::null(),
    }
  }

  pub fn get(&self) -> &wgpu::RenderPipeline { &self.pipeline.as_ref().unwrap() }

  pub(super) fn update(
    &mut self,
    format: wgpu::TextureFormat,
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
  ) {
    if self.format != Some(format) || layout as *const _ != self.layout_addr {
      self.pipeline.take();
      self.format = Some(format);
      self.layout_addr = layout as *const _;
    }

    if self.pipeline.is_none() {
      self.pipeline = Some(Self::pipeline(
        self.label,
        &self.shader,
        device,
        format,
        layout,
      ));
    }
  }

  fn pipeline(
    label: &str,
    shader: &wgpu::ShaderModule,
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    layout: &wgpu::PipelineLayout,
  ) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
      label: Some(label),
      layout: Some(&layout),
      vertex: wgpu::VertexState {
        module: shader,
        entry_point: "vs_main",
        buffers: &[wgpu::VertexBufferLayout {
          array_stride: size_of::<Vertex<u32>>() as wgpu::BufferAddress,
          step_mode: wgpu::VertexStepMode::Vertex,
          attributes: &[
            wgpu::VertexAttribute {
              offset: 0,
              shader_location: 0,
              format: wgpu::VertexFormat::Float32x2,
            },
            wgpu::VertexAttribute {
              offset: (size_of::<[f32; 2]>()) as wgpu::BufferAddress,
              shader_location: 1,
              format: wgpu::VertexFormat::Uint32,
            },
          ],
        }],
      },
      fragment: Some(wgpu::FragmentState {
        module: shader,
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
        count: 1,
        mask: !0,
        alpha_to_coverage_enabled: false,
      },
      multiview: None,
    })
  }
}
