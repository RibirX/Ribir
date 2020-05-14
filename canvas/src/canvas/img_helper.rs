use super::{spv_2_shader_module, PhysicRect, PhysicSize};
pub(crate) struct RgbaConvert {
  group_layout: wgpu::BindGroupLayout,
  pipeline: wgpu::ComputePipeline,
}

impl RgbaConvert {
  pub(crate) fn new(device: &wgpu::Device) -> Self {
    let group_layout =
      device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        bindings: &[wgpu::BindGroupLayoutEntry {
          binding: 0,
          visibility: wgpu::ShaderStage::COMPUTE,
          ty: wgpu::BindingType::StorageBuffer {
            dynamic: false,
            readonly: false,
          },
        }],
        label: None,
      });

    let cs_module =
      spv_2_shader_module!(device, "../shaders/bgra_2_rgba.comp.spv");

    let pipeline_layout =
      device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        bind_group_layouts: &[&group_layout],
      });

    let pipeline =
      device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        layout: &pipeline_layout,
        compute_stage: wgpu::ProgrammableStageDescriptor {
          module: &cs_module,
          entry_point: "main",
        },
      });

    Self {
      group_layout,
      pipeline,
    }
  }

  /// Use compute shader to convert a image from bgra to rgba.
  pub(crate) fn compute_shader_convert(
    &self,
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    bgra_buffer: &wgpu::Buffer,
    size: PhysicSize,
  ) {
    let slice_size = size.area() as u64 * std::mem::size_of::<u32>() as u64;

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
      layout: &self.group_layout,
      bindings: &[wgpu::Binding {
        binding: 0,
        resource: wgpu::BindingResource::Buffer {
          buffer: &bgra_buffer,
          range: 0..slice_size,
        },
      }],
      label: None,
    });

    {
      let mut cpass = encoder.begin_compute_pass();
      cpass.set_pipeline(&self.pipeline);
      cpass.set_bind_group(0, &bind_group, &[]);
      cpass.dispatch(size.area(), 1, 1);
    }
  }
}

pub(crate) async fn texture_to_png<W: std::io::Write>(
  texture: &wgpu::Texture,
  rect: PhysicRect,
  device: &wgpu::Device,
  queue: &wgpu::Queue,
  convert: &RgbaConvert,
  writer: W,
) -> Result<(), &'static str> {
  let PhysicSize { width, height, .. } = rect.size;
  let size = width as u64 * height as u64 * std::mem::size_of::<u32>() as u64;

  // The output buffer lets us retrieve the data as an array
  let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
    size,
    usage: wgpu::BufferUsage::MAP_READ
      | wgpu::BufferUsage::STORAGE
      | wgpu::BufferUsage::COPY_DST,

    label: None,
  });

  let mut encoder =
    device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
      label: Some("Encoder for encoding texture as png"),
    });

  encoder.copy_texture_to_buffer(
    wgpu::TextureCopyView {
      texture,
      mip_level: 0,
      array_layer: 0,
      origin: wgpu::Origin3d {
        x: rect.min_x(),
        y: rect.min_y(),
        z: 0,
      },
    },
    wgpu::BufferCopyView {
      buffer: &output_buffer,
      offset: 0,
      bytes_per_row: std::mem::size_of::<u32>() as u32 * width as u32,
      rows_per_image: 0,
    },
    wgpu::Extent3d {
      width: width,
      height: height,
      depth: 1,
    },
  );
  convert.compute_shader_convert(
    device,
    &mut encoder,
    &output_buffer,
    rect.size,
  );

  queue.submit(&[encoder.finish()]);

  // Note that we're not calling `.await` here.
  let buffer_future = output_buffer.map_read(0, size);

  // Poll the device in a blocking manner so that our future resolves.
  device.poll(wgpu::Maintain::Wait);

  let mapping = buffer_future.await.map_err(|_| "Async buffer error")?;
  let mut png_encoder = png::Encoder::new(writer, width, height);
  png_encoder.set_depth(png::BitDepth::Eight);
  png_encoder.set_color(png::ColorType::RGBA);
  png_encoder
    .write_header()
    .unwrap()
    .write_image_data(mapping.as_slice())
    .unwrap();

  Ok(())
}
