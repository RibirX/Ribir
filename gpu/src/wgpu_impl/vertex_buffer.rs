use std::{any::type_name, marker::PhantomData, mem::size_of, ops::Range};

use ribir_painter::Vertex;
use zerocopy::AsBytes;

pub struct VerticesBuffer<T: AsBytes> {
  vertices: wgpu::Buffer,
  indices: wgpu::Buffer,
  v_offset: wgpu::BufferAddress,
  i_offset: wgpu::BufferAddress,
  _phantom: PhantomData<T>,
}

impl<T: AsBytes> VerticesBuffer<T> {
  pub fn new(init_vertices_cnt: usize, init_indices_cnt: usize, device: &wgpu::Device) -> Self {
    Self {
      vertices: new_vertices::<T>(device, init_vertices_cnt),
      indices: new_indices(device, init_indices_cnt),
      v_offset: 0,
      i_offset: 0,
      _phantom: PhantomData,
    }
  }

  pub fn reset(&mut self) {
    self.v_offset = 0;
    self.i_offset = 0;
  }

  /// Write vertex and index data to the buffer. Returns `None` if a flush is
  /// needed before the data can be loaded (i.e., there is pending data and the
  /// buffer can't fit the new data). If the buffer is empty and still can't
  /// fit, it will be expanded.
  pub fn write_buffer(
    &mut self, data: &ribir_painter::VertexBuffers<T>, device: &wgpu::Device, queue: &wgpu::Queue,
  ) -> Option<(Range<wgpu::BufferAddress>, Range<wgpu::BufferAddress>)> {
    let vertices_data = data.vertices.as_bytes();
    let indices_data = data.indices.as_bytes();

    let v_fits = self.vertices.size() >= self.v_offset + vertices_data.len() as wgpu::BufferAddress;
    let i_fits = self.indices.size() >= self.i_offset + indices_data.len() as wgpu::BufferAddress;

    if !v_fits || !i_fits {
      // Has pending data, need to flush first.
      if self.v_offset > 0 || self.i_offset > 0 {
        return None;
      }
      // No pending data, expand the buffer.
      if !v_fits {
        let mut new_size = self.vertices.size() * 2;
        while new_size < vertices_data.len() as wgpu::BufferAddress {
          new_size *= 2;
        }
        self.vertices = device.create_buffer(&wgpu::BufferDescriptor {
          label: Some(&format!("{} vertices buffer", type_name::<T>())),
          size: new_size,
          usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
          mapped_at_creation: false,
        });
      }
      if !i_fits {
        let mut new_size = self.indices.size() * 2;
        while new_size < indices_data.len() as wgpu::BufferAddress {
          new_size *= 2;
        }
        self.indices = device.create_buffer(&wgpu::BufferDescriptor {
          label: Some("indices buffer"),
          size: new_size,
          usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
          mapped_at_creation: false,
        });
      }
    }

    queue.write_buffer(&self.vertices, self.v_offset, vertices_data);
    let v_range = self.v_offset..self.v_offset + vertices_data.len() as wgpu::BufferAddress;
    self.v_offset += vertices_data.len() as wgpu::BufferAddress;

    queue.write_buffer(&self.indices, self.i_offset, indices_data);
    let i_range = self.i_offset..self.i_offset + indices_data.len() as wgpu::BufferAddress;
    self.i_offset += indices_data.len() as wgpu::BufferAddress;

    Some((v_range, i_range))
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
