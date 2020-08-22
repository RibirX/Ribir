use super::{DeviceRect, DeviceSize};

pub(crate) struct RgbaConvert {
  group_layout: wgpu::BindGroupLayout,
  pipeline: wgpu::ComputePipeline,
}

impl RgbaConvert {
  pub(crate) fn new(device: &wgpu::Device) -> Self {
    let group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
      entries: &[wgpu::BindGroupLayoutEntry {
        binding: 0,
        visibility: wgpu::ShaderStage::COMPUTE,
        ty: wgpu::BindingType::StorageBuffer {
          dynamic: false,
          readonly: false,
          min_binding_size: None,
        },
        count: None,
      }],
      label: None,
    });

    let cs_module =
      device.create_shader_module(wgpu::include_spirv!("./shaders/bgra_2_rgba.comp.spv"));

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
      bind_group_layouts: &[&group_layout],
      push_constant_ranges: &[],
      label: Some("RGBA convert render pipeline layout"),
    });

    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
      label: Some("image convert pipeline"),
      layout: Some(&pipeline_layout),
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
    size: DeviceSize,
  ) {
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
      layout: &self.group_layout,
      entries: &[wgpu::BindGroupEntry {
        binding: 0,
        resource: wgpu::BindingResource::Buffer(bgra_buffer.slice(..)),
      }],
      label: None,
    });

    {
      let mut c_pass = encoder.begin_compute_pass();
      c_pass.set_pipeline(&self.pipeline);
      c_pass.set_bind_group(0, &bind_group, &[]);
      c_pass.dispatch(size.area(), 1, 1);
    }
  }
}

pub(crate) async fn bgra_texture_to_png<W: std::io::Write>(
  texture: &wgpu::Texture,
  rect: DeviceRect,
  device: &wgpu::Device,
  queue: &wgpu::Queue,
  convert: &RgbaConvert,
  writer: W,
) -> Result<(), &'static str> {
  let DeviceSize { width, height, .. } = rect.size;
  const PX_BYTES: usize = std::mem::size_of::<u32>();
  // align to 256 bytes by WebGPU require.
  const WGPU_ALIGN: usize = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize;
  const ALIGN_BYTES: u32 = (WGPU_ALIGN / PX_BYTES) as u32;
  let align_width = {
    match width % ALIGN_BYTES {
      0 => width,
      other => width - other + ALIGN_BYTES,
    }
  };

  let size = align_width as u64 * height as u64 * PX_BYTES as u64;

  let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
    label: Some("Encoder for encoding texture as png"),
  });

  // The output buffer lets us retrieve the data as an array
  let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
    size,
    usage: wgpu::BufferUsage::STORAGE | wgpu::BufferUsage::COPY_DST | wgpu::BufferUsage::COPY_SRC,
    mapped_at_creation: false,
    label: None,
  });

  encoder.copy_texture_to_buffer(
    wgpu::TextureCopyView {
      texture,
      mip_level: 0,
      origin: wgpu::Origin3d {
        x: rect.min_x(),
        y: rect.min_y(),
        z: 0,
      },
    },
    wgpu::BufferCopyView {
      buffer: &output_buffer,
      layout: wgpu::TextureDataLayout {
        offset: 0,
        bytes_per_row: PX_BYTES as u32 * align_width as u32,
        rows_per_image: 0,
      },
    },
    wgpu::Extent3d {
      width,
      height,
      depth: 1,
    },
  );
  convert.compute_shader_convert(
    device,
    &mut encoder,
    &output_buffer,
    DeviceSize::new(align_width, height),
  );

  let map_buffer = device.create_buffer(&wgpu::BufferDescriptor {
    size,
    usage: wgpu::BufferUsage::MAP_READ | wgpu::BufferUsage::COPY_DST,
    mapped_at_creation: false,
    label: None,
  });

  encoder.copy_buffer_to_buffer(&output_buffer, 0, &map_buffer, 0, size);

  queue.submit(Some(encoder.finish()));

  let buffer_slice = map_buffer.slice(..);
  // Note that we're not calling `.await` here.

  let buffer_future = buffer_slice.map_async(wgpu::MapMode::Read);

  // Poll the device in a blocking manner so that our future resolves.
  device.poll(wgpu::Maintain::Wait);
  buffer_future.await.map_err(|_| "Async buffer error")?;

  let data = buffer_slice.get_mapped_range();

  let mut png_encoder = png::Encoder::new(writer, width, height);
  png_encoder.set_depth(png::BitDepth::Eight);
  png_encoder.set_color(png::ColorType::RGBA);

  let data: Vec<_> = (0..height)
    .map(|i| {
      let start = (i * align_width) as usize * PX_BYTES;
      data[start..(start + width as usize * PX_BYTES)].iter()
    })
    .flatten()
    .cloned()
    .collect();

  png_encoder
    .write_header()
    .unwrap()
    .write_image_data(data.as_slice())
    .unwrap();

  Ok(())
}
