use std::marker::PhantomData;

use zerocopy::AsBytes;

pub struct UniformVar {
  layout: wgpu::BindGroupLayout,
  buffer: wgpu::Buffer,
  bind: wgpu::BindGroup,
}

impl UniformVar {
  pub fn new(device: &wgpu::Device, visibility: wgpu::ShaderStages, max_byte_size: usize) -> Self {
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
      entries: &[wgpu::BindGroupLayoutEntry {
        binding: 0,
        visibility,
        ty: wgpu::BindingType::Buffer {
          ty: wgpu::BufferBindingType::Uniform,
          has_dynamic_offset: false,
          min_binding_size: None,
        },
        count: None,
      }],
      label: Some("UniformVar layout"),
    });

    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
      label: Some("UniformVar buffer"),
      size: max_byte_size as u64,
      usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
      mapped_at_creation: false,
    });
    let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
      label: Some("UniformVar bind"),
      layout: &layout,
      entries: &[wgpu::BindGroupEntry {
        binding: 0,
        resource: wgpu::BindingResource::Buffer(buffer.as_entire_buffer_binding()),
      }],
    });

    Self { layout, buffer, bind }
  }

  pub fn write_buffer(&mut self, queue: &wgpu::Queue, offset: usize, data: &[u8]) {
    queue.write_buffer(&self.buffer, offset as u64, data);
  }

  pub fn bind_group(&self) -> &wgpu::BindGroup { &self.bind }
  pub fn layout(&self) -> &wgpu::BindGroupLayout { &self.layout }
}

pub struct Uniform<T> {
  var: UniformVar,
  _phantom: PhantomData<T>,
}

impl<T: AsBytes> Uniform<T> {
  pub fn new(device: &wgpu::Device, visibility: wgpu::ShaderStages, elements: usize) -> Self {
    let var = UniformVar::new(device, visibility, elements * std::mem::size_of::<T>());

    let _phantom = PhantomData;
    Self { var, _phantom }
  }

  pub fn write_buffer(&mut self, queue: &wgpu::Queue, data: &[T]) {
    self.var.write_buffer(queue, 0, data.as_bytes());
  }

  pub fn bind_group(&self) -> &wgpu::BindGroup { &self.var.bind }
  pub fn layout(&self) -> &wgpu::BindGroupLayout { &self.var.layout }
  pub fn buffer(&self) -> &wgpu::Buffer { &self.var.buffer }
}
