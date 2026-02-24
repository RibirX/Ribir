use std::marker::PhantomData;

use zerocopy::AsBytes;

pub struct UniformVar {
  layout: wgpu::BindGroupLayout,
  buffer: wgpu::Buffer,
  bind: wgpu::BindGroup,
  current_offset: wgpu::BufferAddress,
}

impl UniformVar {
  const DYNAMIC_OFFSET_ALIGNMENT: wgpu::BufferAddress = 256;

  pub fn new(
    device: &wgpu::Device, visibility: wgpu::ShaderStages, segment_size: usize, total_size: usize,
  ) -> Self {
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
      entries: &[wgpu::BindGroupLayoutEntry {
        binding: 0,
        visibility,
        ty: wgpu::BindingType::Buffer {
          ty: wgpu::BufferBindingType::Uniform,
          has_dynamic_offset: true,
          min_binding_size: None,
        },
        count: None,
      }],
      label: Some("UniformVar layout"),
    });

    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
      label: Some("UniformVar buffer"),
      size: total_size as u64,
      usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
      mapped_at_creation: false,
    });
    let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
      label: Some("UniformVar bind"),
      layout: &layout,
      entries: &[wgpu::BindGroupEntry {
        binding: 0,
        resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
          buffer: &buffer,
          offset: 0,
          size: wgpu::BufferSize::new(segment_size as u64),
        }),
      }],
    });

    Self { layout, buffer, bind, current_offset: 0 }
  }

  pub fn reset(&mut self) { self.current_offset = 0; }

  /// Write data to the buffer. Returns `None` if the buffer can't fit the
  /// data and a flush is needed.
  pub fn write_buffer(&mut self, queue: &wgpu::Queue, data: &[u8]) -> Option<u32> {
    debug_assert!(Self::DYNAMIC_OFFSET_ALIGNMENT.is_power_of_two());
    debug_assert_eq!(self.current_offset % Self::DYNAMIC_OFFSET_ALIGNMENT, 0);
    debug_assert!(!data.is_empty(), "uniform write with empty payload");

    if self.buffer.size() < self.current_offset + data.len() as u64 {
      return None;
    }

    let dynamic_offset = self.current_offset as u32;
    queue.write_buffer(&self.buffer, self.current_offset, data);

    // Uniform offsets must be aligned to 256 bytes
    self.current_offset =
      (self.current_offset + data.len() as u64 + Self::DYNAMIC_OFFSET_ALIGNMENT - 1)
        & !(Self::DYNAMIC_OFFSET_ALIGNMENT - 1);
    debug_assert_eq!(self.current_offset % Self::DYNAMIC_OFFSET_ALIGNMENT, 0);
    Some(dynamic_offset)
  }

  pub fn write_at(&mut self, queue: &wgpu::Queue, offset: u64, data: &[u8]) {
    queue.write_buffer(&self.buffer, self.current_offset + offset, data);
  }

  pub fn advance(&mut self, size: u64) -> u32 {
    debug_assert_eq!(self.current_offset % Self::DYNAMIC_OFFSET_ALIGNMENT, 0);
    let dynamic_offset = self.current_offset as u32;
    self.current_offset = (self.current_offset + size + Self::DYNAMIC_OFFSET_ALIGNMENT - 1)
      & !(Self::DYNAMIC_OFFSET_ALIGNMENT - 1);
    debug_assert_eq!(self.current_offset % Self::DYNAMIC_OFFSET_ALIGNMENT, 0);
    dynamic_offset
  }

  pub fn bind_group(&self) -> &wgpu::BindGroup { &self.bind }
  pub fn layout(&self) -> &wgpu::BindGroupLayout { &self.layout }

  pub fn needs_flush(&self, bytes: u64) -> bool { self.buffer.size() < self.current_offset + bytes }
}

pub struct StorageVar {
  layout: wgpu::BindGroupLayout,
  buffer: wgpu::Buffer,
  bind: wgpu::BindGroup,
  current_offset: wgpu::BufferAddress,
}

impl StorageVar {
  const DYNAMIC_OFFSET_ALIGNMENT: wgpu::BufferAddress = 256;

  pub fn new(
    device: &wgpu::Device, visibility: wgpu::ShaderStages, segment_size: usize, total_size: usize,
  ) -> Self {
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
      entries: &[wgpu::BindGroupLayoutEntry {
        binding: 0,
        visibility,
        ty: wgpu::BindingType::Buffer {
          ty: wgpu::BufferBindingType::Storage { read_only: true },
          has_dynamic_offset: true,
          min_binding_size: None,
        },
        count: None,
      }],
      label: Some("StorageVar layout"),
    });

    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
      label: Some("StorageVar buffer"),
      size: total_size as u64,
      usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
      mapped_at_creation: false,
    });
    let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
      label: Some("StorageVar bind"),
      layout: &layout,
      entries: &[wgpu::BindGroupEntry {
        binding: 0,
        resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
          buffer: &buffer,
          offset: 0,
          size: wgpu::BufferSize::new(segment_size as u64),
        }),
      }],
    });

    Self { layout, buffer, bind, current_offset: 0 }
  }

  pub fn reset(&mut self) { self.current_offset = 0; }

  /// Write data to the buffer. Returns `None` if the buffer can't fit the
  /// data and a flush is needed.
  pub fn write_buffer(&mut self, queue: &wgpu::Queue, data: &[u8]) -> Option<u32> {
    debug_assert!(Self::DYNAMIC_OFFSET_ALIGNMENT.is_power_of_two());
    debug_assert_eq!(self.current_offset % Self::DYNAMIC_OFFSET_ALIGNMENT, 0);
    debug_assert!(!data.is_empty(), "storage write with empty payload");

    if self.buffer.size() < self.current_offset + data.len() as u64 {
      return None;
    }

    let dynamic_offset = self.current_offset as u32;
    queue.write_buffer(&self.buffer, self.current_offset, data);

    // Dynamic offsets must be aligned to 256 bytes
    self.current_offset =
      (self.current_offset + data.len() as u64 + Self::DYNAMIC_OFFSET_ALIGNMENT - 1)
        & !(Self::DYNAMIC_OFFSET_ALIGNMENT - 1);
    debug_assert_eq!(self.current_offset % Self::DYNAMIC_OFFSET_ALIGNMENT, 0);
    Some(dynamic_offset)
  }

  pub fn write_at(&mut self, queue: &wgpu::Queue, offset: u64, data: &[u8]) {
    queue.write_buffer(&self.buffer, self.current_offset + offset, data);
  }

  pub fn advance(&mut self, size: u64) -> u32 {
    debug_assert_eq!(self.current_offset % Self::DYNAMIC_OFFSET_ALIGNMENT, 0);
    let dynamic_offset = self.current_offset as u32;
    self.current_offset = (self.current_offset + size + Self::DYNAMIC_OFFSET_ALIGNMENT - 1)
      & !(Self::DYNAMIC_OFFSET_ALIGNMENT - 1);
    debug_assert_eq!(self.current_offset % Self::DYNAMIC_OFFSET_ALIGNMENT, 0);
    dynamic_offset
  }

  pub fn bind_group(&self) -> &wgpu::BindGroup { &self.bind }
  pub fn layout(&self) -> &wgpu::BindGroupLayout { &self.layout }
  #[allow(dead_code)]
  pub fn current_offset(&self) -> u64 { self.current_offset }

  pub fn needs_flush(&self, bytes: u64) -> bool { self.buffer.size() < self.current_offset + bytes }
}

pub struct Uniform<T> {
  var: UniformVar,
  _phantom: PhantomData<T>,
}

impl<T: AsBytes> Uniform<T> {
  /// Number of phases we preallocate buffer capacity for. Each frame may have
  /// multiple draw phases that each require a fresh slice of the uniform
  /// buffer.
  const STREAMING_PHASES: usize = 16;

  pub fn new(device: &wgpu::Device, visibility: wgpu::ShaderStages, elements: usize) -> Self {
    let segment_size = elements * std::mem::size_of::<T>();
    let aligned_segment_size = (segment_size + 255) & !255;
    // Preallocate enough room for STREAMING_PHASES draw phases per frame so
    // that streaming writes rarely need a mid-frame flush.
    let total_size = aligned_segment_size * Self::STREAMING_PHASES;
    let var = UniformVar::new(device, visibility, segment_size, total_size);

    let _phantom = PhantomData;
    Self { var, _phantom }
  }

  pub fn reset(&mut self) { self.var.reset(); }

  pub fn write_buffer(&mut self, queue: &wgpu::Queue, data: &[T]) -> Option<u32> {
    self.var.write_buffer(queue, data.as_bytes())
  }

  pub fn bind_group(&self) -> &wgpu::BindGroup { &self.var.bind }
  pub fn layout(&self) -> &wgpu::BindGroupLayout { &self.var.layout }
}
