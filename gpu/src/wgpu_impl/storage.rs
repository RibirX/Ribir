use std::{any::type_name, marker::PhantomData, mem::size_of};

use zerocopy::AsBytes;

pub struct Storage<T> {
  layout: wgpu::BindGroupLayout,
  buffer: wgpu::Buffer,
  bind: wgpu::BindGroup,
  _phantom: PhantomData<T>,
}

impl<T: AsBytes> Storage<T> {
  pub fn new(device: &wgpu::Device, visibility: wgpu::ShaderStages, init_count: usize) -> Self {
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
      entries: &[wgpu::BindGroupLayoutEntry {
        binding: 0,
        visibility,
        ty: wgpu::BindingType::Buffer {
          ty: wgpu::BufferBindingType::Storage { read_only: true },
          has_dynamic_offset: false,
          min_binding_size: None,
        },
        count: None,
      }],
      label: Some(&format!("{} storage layout", type_name::<T>())),
    });
    let (buffer, bind) = Self::new_bind(
      device,
      &layout,
      (size_of::<T>() * init_count) as wgpu::BufferAddress,
    );
    let _phantom = PhantomData;
    Self { layout, buffer, bind, _phantom }
  }

  pub fn write_buffer(&mut self, device: &wgpu::Device, queue: &mut wgpu::Queue, data: &[T]) {
    let buffer_size = (std::mem::size_of_val(data)) as wgpu::BufferAddress;
    if self.buffer.size() < buffer_size {
      (self.buffer, self.bind) = Self::new_bind(device, &self.layout, buffer_size);
    }

    queue.write_buffer(&self.buffer, 0, data.as_bytes());
  }

  pub fn bind_group(&self) -> &wgpu::BindGroup { &self.bind }
  pub fn layout(&self) -> &wgpu::BindGroupLayout { &self.layout }

  fn new_bind(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    bytes: wgpu::BufferAddress,
  ) -> (wgpu::Buffer, wgpu::BindGroup) {
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
      label: Some(&format!("{} storage buffer", type_name::<T>())),
      size: bytes,
      usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
      mapped_at_creation: false,
    });
    let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
      label: Some(&format!("{} storage bind", type_name::<T>())),
      layout,
      entries: &[wgpu::BindGroupEntry {
        binding: 0,
        resource: wgpu::BindingResource::Buffer(buffer.as_entire_buffer_binding()),
      }],
    });

    (buffer, bind)
  }
}
