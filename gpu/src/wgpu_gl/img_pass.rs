use algo::FrameCache;

use super::primitives_layout;
use crate::Vertex;

pub struct ImagePass {
  pub uniform_layout: wgpu::BindGroupLayout,
  pub resources: FrameCache<usize, wgpu::Texture>,
  pub pipeline: wgpu::RenderPipeline,
  pub sampler: wgpu::Sampler,
}

impl ImagePass {
  pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
    let uniform_layout = uniform_layout(device);
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
      address_mode_u: wgpu::AddressMode::ClampToEdge,
      address_mode_v: wgpu::AddressMode::ClampToEdge,
      address_mode_w: wgpu::AddressMode::ClampToEdge,
      mag_filter: wgpu::FilterMode::Nearest,
      min_filter: wgpu::FilterMode::Nearest,
      mipmap_filter: wgpu::FilterMode::Nearest,
      lod_min_clamp: 0.0,
      lod_max_clamp: 0.0,
      label: Some("Texture atlas sampler"),
      ..Default::default()
    });
    let pipeline = pipeline(device, format, &uniform_layout);
    ImagePass {
      uniform_layout,
      resources: <_>::default(),
      pipeline,
      sampler,
    }
  }

  pub fn create_texture_uniform(
    &mut self,
    device: &wgpu::Device,
    mem_texture: crate::Texture,
    coordinate_matrix: &wgpu::Buffer,
    queue: &wgpu::Queue,
  ) -> wgpu::BindGroup {
    let view = self
      .resources
      .get_or_insert_with(&mem_texture.id, || {
        let format = match mem_texture.format {
          painter::image::ColorFormat::Rgba8 => wgpu::TextureFormat::Rgba8UnormSrgb,
        };
        let size = wgpu::Extent3d {
          width: mem_texture.size.width,
          height: mem_texture.size.height,
          depth_or_array_layers: 1,
        };
        let texture_descriptor = &wgpu::TextureDescriptor {
          label: Some("Create wgpu texture"),
          size,
          dimension: wgpu::TextureDimension::D2,
          format,
          usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
          mip_level_count: 1,
          sample_count: 1,
        };
        let texture = device.create_texture(texture_descriptor);

        let bytes_per_pixel = mem_texture.format.pixel_per_bytes();
        queue.write_texture(
          // Tells wgpu where to copy the pixel data
          wgpu::ImageCopyTexture {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
          },
          // The actual pixel data
          mem_texture
            .data
            .expect("should have image data, if no cache have"),
          // The layout of the texture
          wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: std::num::NonZeroU32::new(bytes_per_pixel as u32 * size.width),
            rows_per_image: std::num::NonZeroU32::new(size.height),
          },
          size,
        );

        texture
      })
      .create_view(&wgpu::TextureViewDescriptor::default());

    device.create_bind_group(&wgpu::BindGroupDescriptor {
      layout: &self.uniform_layout,
      entries: &[
        wgpu::BindGroupEntry {
          binding: 0,
          resource: coordinate_matrix.as_entire_binding(),
        },
        wgpu::BindGroupEntry {
          binding: 1,
          resource: wgpu::BindingResource::TextureView(&view),
        },
        wgpu::BindGroupEntry {
          binding: 2,
          resource: wgpu::BindingResource::Sampler(&self.sampler),
        },
      ],
      label: Some("uniform_bind_group"),
    })
  }

  pub fn end_frame(&mut self) { self.resources.end_frame("wgpu texture"); }
}

fn uniform_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
  device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
    entries: &[
      wgpu::BindGroupLayoutEntry {
        binding: 0,
        visibility: wgpu::ShaderStages::VERTEX,
        ty: wgpu::BindingType::Buffer {
          ty: wgpu::BufferBindingType::Uniform,
          has_dynamic_offset: false,
          min_binding_size: None,
        },
        count: None,
      },
      wgpu::BindGroupLayoutEntry {
        binding: 1,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
          sample_type: wgpu::TextureSampleType::Float { filterable: true },
          view_dimension: wgpu::TextureViewDimension::D2,
          multisampled: false,
        },
        count: None,
      },
      wgpu::BindGroupLayoutEntry {
        binding: 2,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        count: None,
      },
    ],
    label: Some("uniforms stable layout"),
  })
}

pub fn pipeline(
  device: &wgpu::Device,
  format: wgpu::TextureFormat,
  uniform_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
  let primitive_layout = primitives_layout(&device);

  let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
    label: Some("Create render pipeline"),
    bind_group_layouts: &[uniform_layout, &primitive_layout],
    push_constant_ranges: &[],
  });

  let module = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
    label: Some("Image Shader"),
    source: wgpu::ShaderSource::Wgsl(include_str!("./shaders/img_geometry.wgsl").into()),
  });

  device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
    label: Some("Render Pipeline"),
    layout: Some(&pipeline_layout),
    vertex: wgpu::VertexState {
      module: &module,
      entry_point: "vs_main",
      buffers: &[Vertex::desc()],
    },
    fragment: Some(wgpu::FragmentState {
      module: &module,
      entry_point: "fs_main",
      targets: &[wgpu::ColorTargetState {
        format,
        blend: Some(wgpu::BlendState::REPLACE),
        write_mask: wgpu::ColorWrites::all(),
      }],
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
    multisample: wgpu::MultisampleState::default(),
    multiview: None,
  })
}
