use zerocopy::AsBytes;

use super::uniform::{StorageVar, UniformVar};

#[derive(Clone, Copy)]
pub(super) struct PrimitiveSlice {
  pub(super) byte_offset: u32,
  pub(super) element_offset: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PrimitivePoolMode {
  Uniform,
  Storage,
}

enum PrimitivePoolBackend {
  Uniform { uniform: UniformVar },
  Storage { storage: StorageVar },
}

pub(super) struct PrimitivePool {
  mode: PrimitivePoolMode,
  backend: PrimitivePoolBackend,
}

impl PrimitivePool {
  pub(super) fn new_uniform(
    device: &wgpu::Device, visibility: wgpu::ShaderStages, segment_size: usize, total_size: usize,
  ) -> Self {
    let uniform = UniformVar::new(device, visibility, segment_size, total_size);
    Self { mode: PrimitivePoolMode::Uniform, backend: PrimitivePoolBackend::Uniform { uniform } }
  }

  pub(super) fn new_storage(
    device: &wgpu::Device, visibility: wgpu::ShaderStages, segment_size: usize, total_size: usize,
  ) -> Self {
    let storage = StorageVar::new(device, visibility, segment_size, total_size);
    Self { mode: PrimitivePoolMode::Storage, backend: PrimitivePoolBackend::Storage { storage } }
  }

  pub(super) fn mode(&self) -> PrimitivePoolMode { self.mode }

  pub(super) fn bind_offset(&self, offset: u32) -> u32 {
    match self.mode {
      PrimitivePoolMode::Uniform | PrimitivePoolMode::Storage => offset,
    }
  }

  pub(super) fn resolve_load_offset(&self, slice: PrimitiveSlice) -> u32 {
    match self.mode {
      PrimitivePoolMode::Uniform | PrimitivePoolMode::Storage => slice.byte_offset,
    }
  }

  pub(super) fn index_base(&self, slice: PrimitiveSlice) -> u32 { slice.element_offset }

  pub(super) fn reset(&mut self) {
    match &mut self.backend {
      PrimitivePoolBackend::Uniform { uniform } => uniform.reset(),
      PrimitivePoolBackend::Storage { storage } => storage.reset(),
    }
  }

  pub(super) fn layout(&self) -> &wgpu::BindGroupLayout {
    match &self.backend {
      PrimitivePoolBackend::Uniform { uniform } => uniform.layout(),
      PrimitivePoolBackend::Storage { storage } => storage.layout(),
    }
  }

  pub(super) fn bind_group(&self) -> &wgpu::BindGroup {
    match &self.backend {
      PrimitivePoolBackend::Uniform { uniform } => uniform.bind_group(),
      PrimitivePoolBackend::Storage { storage } => storage.bind_group(),
    }
  }

  pub(super) fn write_typed<T: AsBytes>(&mut self, queue: &wgpu::Queue, data: &[T]) -> Option<u32> {
    self.write_bytes_internal(queue, data.as_bytes())
  }

  pub(super) fn write_typed_slice<T: AsBytes>(
    &mut self, queue: &wgpu::Queue, data: &[T],
  ) -> Option<PrimitiveSlice> {
    let byte_offset = self.write_typed(queue, data)?;
    let element_size = std::mem::size_of::<T>() as u32;
    let element_offset = if element_size == 0 { 0 } else { byte_offset / element_size };
    Some(PrimitiveSlice { byte_offset, element_offset })
  }

  pub(super) fn write_at(&mut self, queue: &wgpu::Queue, offset: u64, data: &[u8]) {
    match &mut self.backend {
      PrimitivePoolBackend::Uniform { uniform } => uniform.write_at(queue, offset, data),
      PrimitivePoolBackend::Storage { storage } => storage.write_at(queue, offset, data),
    }
  }

  pub(super) fn advance(&mut self, size: u64) -> u32 {
    match &mut self.backend {
      PrimitivePoolBackend::Uniform { uniform } => uniform.advance(size),
      PrimitivePoolBackend::Storage { storage } => storage.advance(size),
    }
  }

  pub(super) fn needs_flush(&self, bytes: u64) -> bool {
    match &self.backend {
      PrimitivePoolBackend::Uniform { uniform } => uniform.needs_flush(bytes),
      PrimitivePoolBackend::Storage { storage } => storage.needs_flush(bytes),
    }
  }

  fn write_bytes_internal(&mut self, queue: &wgpu::Queue, data: &[u8]) -> Option<u32> {
    match &mut self.backend {
      PrimitivePoolBackend::Uniform { uniform } => uniform.write_buffer(queue, data),
      PrimitivePoolBackend::Storage { storage } => storage.write_buffer(queue, data),
    }
  }
}
