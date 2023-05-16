use super::buffer_pool::BufferPool;
use crate::{command_encoder, gpu_backend::Texture, vertices_coord, WgpuImpl, WgpuTexture};
use ribir_geom::{rect_corners, DevicePoint, DeviceRect, DeviceSize};
use ribir_painter::Vertex;
use std::mem::size_of;
const POOL_SIZE: usize = 256;

pub struct DrawTexturePass {
  pipeline: Option<wgpu::RenderPipeline>,
  shader: wgpu::ShaderModule,
  layout: wgpu::PipelineLayout,
  pub(crate) bind_layout: wgpu::BindGroupLayout,
  format: Option<wgpu::TextureFormat>,
  vertices_pool: BufferPool<[Vertex<[f32; 2]>; 4]>,
}

impl DrawTexturePass {
  pub fn new(device: &wgpu::Device) -> Self {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
      label: Some("Draw texture to texture"),
      source: wgpu::ShaderSource::Wgsl(include_str!("./shaders/tex_2_tex.wgsl").into()),
    });

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
      label: Some("Texture to texture"),
    });

    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
      label: Some("Draw texture to texture"),
      bind_group_layouts: &[&bind_layout],
      push_constant_ranges: &[],
    });
    let vertices_pool = BufferPool::new(
      POOL_SIZE,
      wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
      device,
    );
    Self {
      pipeline: None,
      shader,
      format: None,
      bind_layout,
      layout,
      vertices_pool,
    }
  }

  pub fn update(&mut self, format: wgpu::TextureFormat, device: &wgpu::Device) {
    if Some(format) != self.format {
      self.format = Some(format);
      self.pipeline.take();
    }

    if self.pipeline.is_none() {
      let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Draw texture to texture"),
        layout: Some(&self.layout),
        vertex: wgpu::VertexState {
          module: &self.shader,
          entry_point: "vs_main",
          buffers: &[wgpu::VertexBufferLayout {
            array_stride: size_of::<Vertex<[f32; 2]>>() as wgpu::BufferAddress,
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
                format: wgpu::VertexFormat::Float32x2,
              },
            ],
          }],
        },
        fragment: Some(wgpu::FragmentState {
          module: &self.shader,
          entry_point: "fs_main",
          targets: &[Some(wgpu::ColorTargetState {
            format,
            blend: Some(wgpu::BlendState::REPLACE),
            write_mask: wgpu::ColorWrites::all(),
          })],
        }),
        primitive: wgpu::PrimitiveState {
          topology: wgpu::PrimitiveTopology::TriangleStrip,
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
      });

      self.pipeline = Some(pipeline);
    }
  }

  pub fn submit(&mut self, queue: &mut wgpu::Queue) { self.vertices_pool.submit_buffer(queue); }

  pub fn clear(&mut self) { self.vertices_pool.clear() }
}

impl WgpuImpl {
  pub(crate) fn draw_texture_to_texture(
    &mut self,
    dist_tex: &WgpuTexture,
    dist_at: DevicePoint,
    from_tex: &WgpuTexture,
    src_rect: &DeviceRect,
  ) {
    if self.draw_tex_pass.vertices_pool.is_full() {
      self.submit();
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

    let draw_tex_pass = &mut self.draw_tex_pass;
    draw_tex_pass.update(dist_tex.format(), &self.device);

    let [d_lt, d_rt, d_rb, d_lb] = vertices_corners(
      &DeviceRect::new(dist_at, src_rect.size),
      Texture::size(dist_tex),
    );

    let [s_lt, s_rt, s_rb, s_lb] = vertices_corners(src_rect, Texture::size(from_tex));

    let address = draw_tex_pass
      .vertices_pool
      .push_value([
        Vertex::new(d_lt, s_lt),
        Vertex::new(d_lb, s_lb),
        Vertex::new(d_rt, s_rt),
        Vertex::new(d_rb, s_rb),
      ])
      .unwrap();

    let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
      layout: &draw_tex_pass.bind_layout,
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
      ops: wgpu::Operations {
        load: wgpu::LoadOp::Load,
        store: true,
      },
    };

    let encoder = command_encoder!(self);
    let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
      label: Some("draw texture to texture"),
      color_attachments: &[Some(color_attachments)],
      depth_stencil_attachment: None,
    });

    rpass.set_vertex_buffer(0, draw_tex_pass.vertices_pool.buffer().slice(address..));
    rpass.set_bind_group(0, &bind_group, &[]);

    rpass.set_scissor_rect(
      dist_at.x as u32,
      dist_at.y as u32,
      src_rect.width() as u32,
      src_rect.height() as u32,
    );
    rpass.set_pipeline(draw_tex_pass.pipeline.as_ref().unwrap());

    rpass.draw(0..4, 0..1)
  }
}
