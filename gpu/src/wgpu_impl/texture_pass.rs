use std::mem::size_of;

use ribir_geom::{rect_corners, DevicePoint, DeviceRect, DeviceSize};
use ribir_painter::Vertex;
use wgpu::{include_wgsl, StoreOp};
use zerocopy::AsBytes;

use super::vertex_buffer::new_vertices;
use crate::{command_encoder, gpu_backend::Texture, vertices_coord, WgpuImpl, WgpuTexture};

pub struct CopyTexturePass {
  pipeline: Option<wgpu::RenderPipeline>,
  shader: wgpu::ShaderModule,
  layout: wgpu::PipelineLayout,
  pub(crate) bind_layout: wgpu::BindGroupLayout,
  format: Option<wgpu::TextureFormat>,
  vertices_buffer: wgpu::Buffer,
}

impl CopyTexturePass {
  pub fn new(device: &wgpu::Device) -> Self {
    let shader = device.create_shader_module(include_wgsl!("./shaders/copy_texture.wgsl"));

    let bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
      entries: &[
        wgpu::BindGroupLayoutEntry {
          binding: 0,
          visibility: wgpu::ShaderStages::FRAGMENT,
          ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
          },
          count: None,
        },
        wgpu::BindGroupLayoutEntry {
          binding: 1,
          visibility: wgpu::ShaderStages::FRAGMENT,
          ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
          count: None,
        },
      ],
      label: Some("Copy texture"),
    });

    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
      label: Some("Copy texture"),
      bind_group_layouts: &[&bind_layout],
      push_constant_ranges: &[],
    });
    let vertices_buffer = new_vertices::<[f32; 2]>(device, 4);
    Self { pipeline: None, shader, format: None, bind_layout, layout, vertices_buffer }
  }

  pub fn update(&mut self, format: wgpu::TextureFormat, device: &wgpu::Device) {
    if Some(format) != self.format {
      self.format = Some(format);
      self.pipeline.take();
    }

    if self.pipeline.is_none() {
      let pipeline = tex_render_pipeline::<[f32; 2]>(
        "Copy texture",
        device,
        &self.layout,
        &self.shader,
        &[
          wgpu::VertexAttribute {
            offset: 0,
            shader_location: 0,
            format: wgpu::VertexFormat::Float32x2,
          },
          wgpu::VertexAttribute {
            offset: (size_of::<[f32; 2]>()) as wgpu::BufferAddress,
            shader_location: 1,
            format: wgpu::VertexFormat::Float32x2,
          },
        ],
        format,
        wgpu::PrimitiveTopology::TriangleStrip,
      );
      self.pipeline = Some(pipeline);
    }
  }
}

pub struct ClearTexturePass {
  pipeline: Option<wgpu::RenderPipeline>,
  shader: wgpu::ShaderModule,
  layout: wgpu::PipelineLayout,
  format: Option<wgpu::TextureFormat>,
  vertices_buffer: wgpu::Buffer,
}

impl ClearTexturePass {
  pub fn new(device: &wgpu::Device) -> Self {
    let shader = device.create_shader_module(include_wgsl!("./shaders/clear_texture.wgsl"));

    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
      label: Some("Clear texture areas"),
      bind_group_layouts: &[],
      push_constant_ranges: &[],
    });

    let vertices_buffer = new_vertices::<()>(device, 256);
    Self { pipeline: None, shader, format: None, layout, vertices_buffer }
  }

  pub fn update(&mut self, format: wgpu::TextureFormat, device: &wgpu::Device) {
    if Some(format) != self.format {
      self.format = Some(format);
      self.pipeline.take();
    }

    if self.pipeline.is_none() {
      let pipeline = tex_render_pipeline::<()>(
        "Clear texture areas",
        device,
        &self.layout,
        &self.shader,
        &[wgpu::VertexAttribute {
          offset: 0,
          shader_location: 0,
          format: wgpu::VertexFormat::Float32x2,
        }],
        format,
        wgpu::PrimitiveTopology::TriangleList,
      );

      self.pipeline = Some(pipeline);
    }
  }
}

impl WgpuImpl {
  pub(crate) fn draw_texture_to_texture(
    &mut self, dist_tex: &WgpuTexture, dist_at: DevicePoint, from_tex: &WgpuTexture,
    src_rect: &DeviceRect,
  ) {
    let pass = self
      .copy_tex_pass
      .get_or_insert_with(|| CopyTexturePass::new(&self.device));

    pass.update(dist_tex.format(), &self.device);

    let [d_lt, d_rt, d_rb, d_lb] =
      vertices_corners(&DeviceRect::new(dist_at, src_rect.size), Texture::size(dist_tex));

    let [s_lt, s_rt, s_rb, s_lb] = vertices_corners(src_rect, Texture::size(from_tex));

    self.queue.write_buffer(
      &pass.vertices_buffer,
      0,
      [
        Vertex::new(d_lt, s_lt),
        Vertex::new(d_lb, s_lb),
        Vertex::new(d_rt, s_rt),
        Vertex::new(d_rb, s_rb),
      ]
      .as_bytes(),
    );

    let bind_group = self
      .device
      .create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &pass.bind_layout,
        entries: &[
          wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::TextureView(from_tex.view()),
          },
          wgpu::BindGroupEntry {
            binding: 1,
            resource: wgpu::BindingResource::Sampler(&self.sampler),
          },
        ],
        label: Some("Color primitives storage bind group"),
      });

    let color_attachments = wgpu::RenderPassColorAttachment {
      view: dist_tex.view(),
      resolve_target: None,
      ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: StoreOp::Store },
    };

    let encoder = command_encoder!(self);
    let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
      label: Some("Copy texture"),
      color_attachments: &[Some(color_attachments)],
      depth_stencil_attachment: None,
      timestamp_writes: None,
      occlusion_query_set: None,
    });

    rpass.set_vertex_buffer(0, pass.vertices_buffer.slice(..));
    rpass.set_bind_group(0, &bind_group, &[]);

    rpass.set_scissor_rect(
      dist_at.x as u32,
      dist_at.y as u32,
      src_rect.width() as u32,
      src_rect.height() as u32,
    );
    rpass.set_pipeline(pass.pipeline.as_ref().unwrap());

    rpass.draw(0..4, 0..1)
  }

  pub(crate) fn clear_tex_areas(&mut self, clear_areas: &[DeviceRect], tex: &WgpuTexture) {
    self.finish_command();

    let Self { clear_tex_pass: pass, device, queue, .. } = self;
    pass.update(tex.format(), device);

    let tex_size = tex.size();
    let mut vertices: Vec<[f32; 2]> = Vec::with_capacity(clear_areas.len() * 4);
    for area in clear_areas {
      let [d_lt, d_rt, d_rb, d_lb] = vertices_corners(area, tex_size);
      vertices.push(d_lt);
      vertices.push(d_lb);
      vertices.push(d_rb);
      vertices.push(d_rb);
      vertices.push(d_rt);
      vertices.push(d_lt);
    }

    let vertices_data = vertices.as_bytes();
    if pass.vertices_buffer.size() < vertices_data.len() as wgpu::BufferAddress {
      pass.vertices_buffer = new_vertices::<()>(device, vertices.len());
    }

    queue.write_buffer(&pass.vertices_buffer, 0, vertices_data);

    let color_attachments = wgpu::RenderPassColorAttachment {
      view: tex.view(),
      resolve_target: None,
      ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: StoreOp::Store },
    };

    let encoder = command_encoder!(self);
    let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
      label: Some("Clear texture areas"),
      color_attachments: &[Some(color_attachments)],
      depth_stencil_attachment: None,
      timestamp_writes: None,
      occlusion_query_set: None,
    });

    rpass.set_vertex_buffer(0, pass.vertices_buffer.slice(..));
    rpass.set_pipeline(pass.pipeline.as_ref().unwrap());
    rpass.draw(0..vertices.len() as u32, 0..1);
  }
}

fn tex_render_pipeline<T>(
  label: &str, device: &wgpu::Device, layout: &wgpu::PipelineLayout, shader: &wgpu::ShaderModule,
  vertex_attrs: &[wgpu::VertexAttribute], format: wgpu::TextureFormat,
  topology: wgpu::PrimitiveTopology,
) -> wgpu::RenderPipeline {
  device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
    label: Some(label),
    layout: Some(layout),
    vertex: wgpu::VertexState {
      module: shader,
      entry_point: "vs_main",
      buffers: &[wgpu::VertexBufferLayout {
        array_stride: size_of::<Vertex<T>>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: vertex_attrs,
      }],
      compilation_options: Default::default(),
    },
    fragment: Some(wgpu::FragmentState {
      module: shader,
      entry_point: "fs_main",
      targets: &[Some(wgpu::ColorTargetState {
        format,
        blend: Some(wgpu::BlendState::REPLACE),
        write_mask: wgpu::ColorWrites::all(),
      })],
      compilation_options: Default::default(),
    }),
    primitive: wgpu::PrimitiveState {
      topology,
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
  })
}

fn vertices_corners(rect: &DeviceRect, tex_size: DeviceSize) -> [[f32; 2]; 4] {
  let [a, b, c, d] = rect_corners(&rect.to_f32().cast_unit());
  [
    vertices_coord(a, tex_size),
    vertices_coord(b, tex_size),
    vertices_coord(c, tex_size),
    vertices_coord(d, tex_size),
  ]
}
