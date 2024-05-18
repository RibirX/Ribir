use std::{any::type_name, marker::PhantomData, mem::size_of};

use ribir_painter::Vertex;
use zerocopy::AsBytes;

pub struct VerticesBuffer<T: AsBytes> {
  vertices: wgpu::Buffer,
  indices: wgpu::Buffer,
  _phantom: PhantomData<T>,
}

impl<T: AsBytes> VerticesBuffer<T> {
  pub fn new(init_vertices_cnt: usize, init_indices_cnt: usize, device: &wgpu::Device) -> Self {
    Self {
      vertices: new_vertices::<T>(device, init_vertices_cnt),
      indices: new_indices(device, init_indices_cnt),
      _phantom: PhantomData,
    }
  }

  pub fn write_buffer(
    &mut self, data: &ribir_painter::VertexBuffers<T>, device: &wgpu::Device, queue: &wgpu::Queue,
  ) {
    let vertices_data = data.vertices.as_bytes();
    let indices_data = data.indices.as_bytes();

    if self.vertices.size() < vertices_data.len() as wgpu::BufferAddress {
      self.vertices = new_vertices::<T>(device, data.vertices.len());
    }
    queue.write_buffer(&self.vertices, 0, vertices_data);

    if self.indices.size() < indices_data.len() as wgpu::BufferAddress {
      self.indices = new_indices(device, data.indices.len());
    }
    queue.write_buffer(&self.indices, 0, indices_data);
  }

  pub fn vertices(&self) -> &wgpu::Buffer { &self.vertices }

  pub fn indices(&self) -> &wgpu::Buffer { &self.indices }
}

pub(crate) fn new_vertices<T>(device: &wgpu::Device, len: usize) -> wgpu::Buffer {
  let label = format!("{} vertices buffer", type_name::<T>());
  device.create_buffer(&wgpu::BufferDescriptor {
    label: Some(&label),
    size: (len * size_of::<Vertex<T>>()) as u64,
    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
    mapped_at_creation: false,
  })
}

fn new_indices(device: &wgpu::Device, len: usize) -> wgpu::Buffer {
  device.create_buffer(&wgpu::BufferDescriptor {
    label: Some("indices buffer"),
    size: (len * size_of::<u32>()) as u64,
    usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
    mapped_at_creation: false,
  })
}
