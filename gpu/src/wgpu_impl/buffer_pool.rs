use std::{any::type_name, mem::size_of};

use zerocopy::AsBytes;

/// A pool of `T`, help you batch mini buffer.
pub struct BufferPool<T: AsBytes> {
  data: Vec<T>,
  buffer: wgpu::Buffer,
}

impl<T: AsBytes> BufferPool<T> {
  pub fn new(max_size: usize, usage: wgpu::BufferUsages, device: &wgpu::Device) -> Self {
    let label = format!("{} pool buffer", type_name::<T>());
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
      label: Some(&label),
      size: (max_size * size_of::<T>()) as u64,
      mapped_at_creation: false,
      usage,
    });

    Self {
      data: Vec::with_capacity(max_size),
      buffer,
    }
  }

  /// Push value to the pool and return the buffer address of the value, return
  /// None if is full.
  ///
  /// Remember to call `submit_buffer` method of the poll before you submit your
  /// render command that use the pool data.
  pub fn push_value(&mut self, value: T) -> Option<u64> {
    (self.data.len() < self.data.capacity()).then(|| {
      let address = self.data.len() * size_of::<T>();
      self.data.push(value);
      address as u64
    })
  }

  pub fn submit_buffer(&mut self, queue: &mut wgpu::Queue) {
    if !self.data.is_empty() {
      queue.write_buffer(&self.buffer, 0, self.data.as_bytes());
      self.data.clear();
    }
  }

  pub fn buffer(&self) -> &wgpu::Buffer { &self.buffer }

  pub fn is_full(&self) -> bool { self.data.len() == self.data.capacity() }

  pub fn clear(&mut self) { self.data.clear(); }
}
