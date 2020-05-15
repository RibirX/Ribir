use super::PhysicSize;

/// `Surface` is a thing presentable canvas visual display.
pub trait Surface {
  type V: FrameView;
  fn size(&self) -> PhysicSize;
  fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32);
  fn format(&self) -> wgpu::TextureFormat;
  fn get_next_view(&mut self) -> Self::V;
}

pub trait FrameView {
  fn view(&self) -> &wgpu::TextureView;
}

/// A `Surface` represents a platform-specific surface (e.g. a window).
pub struct PhysicSurface {
  swap_chain: wgpu::SwapChain,
  surface: wgpu::Surface,
  sc_desc: wgpu::SwapChainDescriptor,
}

impl Surface for PhysicSurface {
  type V = TView<wgpu::SwapChainOutput>;

  #[inline]
  fn size(&self) -> PhysicSize {
    PhysicSize::new(self.sc_desc.width, self.sc_desc.height)
  }

  fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
    self.sc_desc.width = width;
    self.sc_desc.height = height;
    self.swap_chain = device.create_swap_chain(&self.surface, &self.sc_desc);
  }

  #[inline]
  fn format(&self) -> wgpu::TextureFormat { self.sc_desc.format }

  fn get_next_view(&mut self) -> Self::V {
    TView(
      self
        .swap_chain
        .get_next_texture()
        .expect("Timeout getting texture"),
    )
  }
}

pub struct TView<T>(T);

impl FrameView for TView<wgpu::SwapChainOutput> {
  #[inline]
  fn view(&self) -> &wgpu::TextureView { &self.0.view }
}

impl PhysicSurface {
  pub(crate) fn new(
    surface: wgpu::Surface,
    device: &wgpu::Device,
    width: u32,
    height: u32,
  ) -> Self {
    let sc_desc = wgpu::SwapChainDescriptor {
      usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
      format: wgpu::TextureFormat::Bgra8UnormSrgb,
      width,
      height,
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

/// A surface use a texture to present
pub(crate) struct SoftSurface {
  size: PhysicSize,
  format: wgpu::TextureFormat,
  texture: wgpu::Texture,
  view: wgpu::TextureView,
}

// impl Surface for SoftSurface {
//   type V = TView<wgpu::TextureView>;

//   #[inline(lint)]
//   fn size(&self) -> PhysicSize { self.size }

//   #[inline]
//   fn format(&self) -> wgpu::TextureFormat { self.format }

//   fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
//     unimplemented!();
//   }

//   fn get_next_view(&mut self) -> Self::V {
//     unimplemented!();
//   }
// }
