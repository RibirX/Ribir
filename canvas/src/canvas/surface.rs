use super::DeviceSize;
use std::borrow::Borrow;

/// `Surface` is a thing presentable canvas visual display.
pub trait Surface {
  type V: Borrow<wgpu::TextureView>;

  fn resize(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, width: u32, height: u32);

  fn size(&self) -> DeviceSize;

  fn get_next_view(&mut self) -> Self::V;
}

/// A `Surface` represents a platform-specific surface (e.g. a window).
pub struct PhysicSurface {
  swap_chain: wgpu::SwapChain,
  surface: wgpu::Surface,
  sc_desc: wgpu::SwapChainDescriptor,
}

/// A `Surface` present in a texture. Usually `PhysicSurface` display things to
/// screen(window eg.), But `TextureSurface` is soft, may not display in any
/// device, bug only in memory.
pub type TextureSurface = Texture;

impl Surface for PhysicSurface {
  type V = FrameView<wgpu::SwapChainFrame>;

  #[inline]
  fn size(&self) -> DeviceSize { DeviceSize::new(self.sc_desc.width, self.sc_desc.height) }

  fn resize(&mut self, device: &wgpu::Device, _queue: &wgpu::Queue, width: u32, height: u32) {
    self.sc_desc.width = width;
    self.sc_desc.height = height;
    self.swap_chain = device.create_swap_chain(&self.surface, &self.sc_desc);
  }

  fn get_next_view(&mut self) -> Self::V {
    FrameView(
      self
        .swap_chain
        .get_next_frame()
        .expect("Timeout getting texture"),
    )
  }
}

impl Surface for TextureSurface {
  type V = FrameView<wgpu::TextureView>;

  #[inline]
  fn size(&self) -> DeviceSize { self.size }

  fn resize(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, width: u32, height: u32) {
    self.resize(device, queue, DeviceSize::new(width, height));
  }

  #[inline]
  fn get_next_view(&mut self) -> Self::V { FrameView(self.raw_texture.create_default_view()) }
}

pub struct FrameView<T>(T);

impl Borrow<wgpu::TextureView> for FrameView<wgpu::SwapChainFrame> {
  fn borrow(&self) -> &wgpu::TextureView { &self.0.output.view }
}

impl Borrow<wgpu::TextureView> for FrameView<wgpu::TextureView> {
  #[inline]
  fn borrow(&self) -> &wgpu::TextureView { &self.0 }
}

impl PhysicSurface {
  pub(crate) fn new(surface: wgpu::Surface, device: &wgpu::Device, size: DeviceSize) -> Self {
    let sc_desc = wgpu::SwapChainDescriptor {
      usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
      format: wgpu::TextureFormat::Bgra8UnormSrgb,
      width: size.width,
      height: size.height,
      present_mode: wgpu::PresentMode::Fifo,
    };

    let swap_chain = device.create_swap_chain(&surface, &sc_desc);

    Self {
      swap_chain,
      sc_desc,
      surface,
    }
  }
}

pub struct Texture {
  pub(crate) raw_texture: wgpu::Texture,
  size: DeviceSize,
  usage: wgpu::TextureUsage,
}

impl Texture {
  pub(crate) const INIT_DIMENSION: u32 = 512;
  pub(crate) const MAX_DIMENSION: u32 = 4096;

  pub(crate) fn new(device: &wgpu::Device, size: DeviceSize, usage: wgpu::TextureUsage) -> Self {
    let raw_texture = Self::new_texture(device, size, usage);
    Texture {
      size,
      raw_texture,
      usage,
    }
  }

  #[inline]
  pub(crate) fn size(&self) -> DeviceSize { self.size }

  pub(crate) fn resize(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, size: DeviceSize) {
    let new_texture = Self::new_texture(device, size, self.usage);

    let size = size.min(self.size);
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
      label: Some("Render Encoder"),
    });
    encoder.copy_texture_to_texture(
      wgpu::TextureCopyView {
        texture: &self.raw_texture,
        mip_level: 0,
        origin: wgpu::Origin3d::ZERO,
      },
      wgpu::TextureCopyView {
        texture: &new_texture,
        mip_level: 0,
        origin: wgpu::Origin3d::ZERO,
      },
      wgpu::Extent3d {
        width: size.width,
        height: size.height,
        depth: 1,
      },
    );

    queue.submit(Some(encoder.finish()));

    self.size = size;
    self.raw_texture = new_texture;
  }

  fn new_texture(
    device: &wgpu::Device,
    size: DeviceSize,
    usage: wgpu::TextureUsage,
  ) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
      label: Some("new texture"),
      size: wgpu::Extent3d {
        width: size.width,
        height: size.height,
        depth: 1,
      },
      dimension: wgpu::TextureDimension::D2,
      format: wgpu::TextureFormat::Bgra8UnormSrgb,
      usage,
      mip_level_count: 1,
      sample_count: 1,
    })
  }
}
