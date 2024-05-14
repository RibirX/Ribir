use std::{any::type_name, marker::PhantomData};

use zerocopy::AsBytes;

pub struct Uniform<T> {
  layout: wgpu::BindGroupLayout,
  buffer: wgpu::Buffer,
  bind: wgpu::BindGroup,
  _phantom: PhantomData<T>,
}

impl<T: AsBytes> Uniform<T> {
  pub fn new(device: &wgpu::Device, visibility: wgpu::ShaderStages, elements: usize) -> Self {
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
      label: Some(&format!("{} uniform layout", type_name::<T>())),
    });
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
      label: Some(&format!("{} uniform buffer", type_name::<T>())),
      size: (std::mem::size_of::<T>() * elements) as u64,
      usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
      mapped_at_creation: false,
    });
    let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
      label: Some(&format!("{} uniform bind", type_name::<T>())),
      layout: &layout,
      entries: &[wgpu::BindGroupEntry {
        binding: 0,
        resource: wgpu::BindingResource::Buffer(buffer.as_entire_buffer_binding()),
      }],
    });

    let _phantom = PhantomData;
    Self { layout, buffer, bind, _phantom }
  }

  pub fn write_buffer(&mut self, queue: &wgpu::Queue, data: &[T]) {
    queue.write_buffer(&self.buffer, 0, data.as_bytes());
  }

  pub fn bind_group(&self) -> &wgpu::BindGroup { &self.bind }
  pub fn layout(&self) -> &wgpu::BindGroupLayout { &self.layout }
}
