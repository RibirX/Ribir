use std::mem::size_of;

use ribir_painter::{DevicePoint, DeviceRect, Vertex};

use crate::{GPUBackendImpl, WgpuImpl};

use super::buffer_pool::BufferPool;
const POOL_SIZE: usize = 256;

pub struct DrawTexWithMaskPass {
  pipeline: Option<wgpu::RenderPipeline>,
  shader: wgpu::ShaderModule,
  layout: wgpu::PipelineLayout,
  bind_layout: wgpu::BindGroupLayout,
  format: Option<wgpu::TextureFormat>,
  vertices_pool: BufferPool<[Vertex<[[f32; 2]; 2]>; 4]>,
}

impl DrawTexWithMaskPass {
  pub fn new(device: &wgpu::Device) -> Self {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
      label: Some("Draw texture with mask"),
      source: wgpu::ShaderSource::Wgsl(include_str!("./shaders/draw_text_with_mask.wgsl").into()),
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
        wgpu::BindGroupLayoutEntry {
          binding: 2,
          visibility: wgpu::ShaderStages::FRAGMENT,
          ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
          },
          count: None,
        },
        wgpu::BindGroupLayoutEntry {
          binding: 3,
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
      &device,
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

  pub fn submit(&mut self, queue: &mut wgpu::Queue) { self.vertices_pool.submit_buffer(queue); }

  pub fn clear(&mut self) { self.vertices_pool.clear() }

  fn update_pipeline(&mut self, format: wgpu::TextureFormat, device: &wgpu::Device) {
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
            array_stride: size_of::<Vertex<[[f32; 2]; 2]>>() as wgpu::BufferAddress,
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
              wgpu::VertexAttribute {
                offset: (size_of::<[[f32; 2]; 2]>()) as wgpu::BufferAddress,
                shader_location: 2,
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
            blend: Some(wgpu::BlendState::ALPHA_BLENDING),
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
}

impl WgpuImpl {
  pub(crate) fn draw_texture_with_mask(
    &mut self,
    dist_tex: &wgpu::Texture,
    dist_start_at: DevicePoint,
    src_tex: &wgpu::Texture,
    src_start_at: DevicePoint,
    mask_tex: &wgpu::Texture,
    mask_rect: &DeviceRect,
  ) {
    let pass = &mut self.draw_text_with_mask_pass;
    pass.update_pipeline(dist_tex.format(), &self.device);

    let dist_start_at = dist_start_at.to_f32();
    let src_start_at = src_start_at.to_f32();
    let mask_rect = mask_rect.to_f32();

    let dist_tex_width = dist_tex.width() as f32;
    let dist_tex_height = dist_tex.height() as f32;
    let src_tex_width = src_tex.width() as f32;
    let src_tex_height = src_tex.height() as f32;
    let mask_tex_width = mask_tex.width() as f32;
    let mask_tex_height = mask_tex.height() as f32;

    let address = pass
      .vertices_pool
      .push_value([
        Vertex::new(
          [
            WgpuImpl::map_x(dist_start_at.x, dist_tex_width),
            WgpuImpl::map_y(dist_start_at.y, dist_tex_height),
          ],
          [
            [
              WgpuImpl::map_x(src_start_at.x, src_tex_width),
              WgpuImpl::map_y(src_start_at.y, src_tex_height),
            ],
            [
              WgpuImpl::map_x(mask_rect.origin.x, mask_tex_width),
              WgpuImpl::map_y(mask_rect.origin.y, mask_tex_height),
            ],
          ],
        ),
        Vertex::new(
          [
            WgpuImpl::map_x(dist_start_at.x, dist_tex_width),
            WgpuImpl::map_y(dist_start_at.y + mask_rect.height(), dist_tex_height),
          ],
          [
            [
              WgpuImpl::map_x(src_start_at.x, src_tex_width),
              WgpuImpl::map_y(src_start_at.y + mask_rect.height(), src_tex_height),
            ],
            [
              WgpuImpl::map_x(mask_rect.origin.x, mask_tex_width),
              WgpuImpl::map_y(mask_rect.max_y(), mask_tex_height),
            ],
          ],
        ),
        Vertex::new(
          [
            WgpuImpl::map_x(dist_start_at.x + mask_rect.width(), dist_tex_width),
            WgpuImpl::map_y(dist_start_at.y, dist_tex_height),
          ],
          [
            [
              WgpuImpl::map_x(src_start_at.x + mask_rect.height(), src_tex_width),
              WgpuImpl::map_y(src_start_at.y, src_tex_height),
            ],
            [
              WgpuImpl::map_x(mask_rect.max_x(), mask_tex_width),
              WgpuImpl::map_y(mask_rect.origin.y, mask_tex_height),
            ],
          ],
        ),
        Vertex::new(
          [
            WgpuImpl::map_x(dist_start_at.x + mask_rect.width(), dist_tex_width),
            WgpuImpl::map_y(dist_start_at.y + mask_rect.height(), dist_tex_height),
          ],
          [
            [
              WgpuImpl::map_x(src_start_at.x + mask_rect.width(), src_tex_width),
              WgpuImpl::map_y(src_start_at.y + mask_rect.height(), src_tex_height),
            ],
            [
              WgpuImpl::map_x(mask_rect.max().x, mask_tex_width),
              WgpuImpl::map_y(mask_rect.max().y, mask_tex_height),
            ],
          ],
        ),
      ])
      .unwrap();

    let tex_view_desc = <_>::default();
    let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
      layout: &pass.bind_layout,
      entries: &[
        wgpu::BindGroupEntry {
          binding: 0,
          resource: wgpu::BindingResource::TextureView(&src_tex.create_view(&tex_view_desc)),
        },
        wgpu::BindGroupEntry {
          binding: 1,
          resource: wgpu::BindingResource::Sampler(&self.sampler),
        },
        wgpu::BindGroupEntry {
          binding: 2,
          resource: wgpu::BindingResource::TextureView(&src_tex.create_view(&tex_view_desc)),
        },
        wgpu::BindGroupEntry {
          binding: 3,
          resource: wgpu::BindingResource::Sampler(&self.sampler),
        },
      ],
      label: Some("Draw texture with mask"),
    });

    let view = dist_tex.create_view(&tex_view_desc);
    let color_attachments = wgpu::RenderPassColorAttachment {
      view: &view,
      resolve_target: None,
      ops: wgpu::Operations {
        load: wgpu::LoadOp::Load,
        store: true,
      },
    };

    let encoder = super::command_encoder!(self);
    let mut r_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
      label: Some("draw texture to texture"),
      color_attachments: &[Some(color_attachments)],
      depth_stencil_attachment: None,
    });

    r_pass.set_vertex_buffer(0, pass.vertices_pool.buffer().slice(address..));
    r_pass.set_bind_group(0, &bind_group, &[]);

    r_pass.set_scissor_rect(
      dist_start_at.x as u32,
      dist_start_at.y as u32,
      mask_rect.width() as u32,
      mask_rect.height() as u32,
    );

    let pipeline = pass.pipeline.as_ref().unwrap();
    r_pass.set_pipeline(pipeline);

    r_pass.draw(0..4, 0..1)
  }
}
