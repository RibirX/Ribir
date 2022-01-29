use super::DeviceSize;
/// `Surface` is a thing presentable canvas visual display.
pub trait Surface {
  fn update(
    &mut self,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    s_config: &wgpu::SurfaceConfiguration,
  );

  fn view_size(&self) -> DeviceSize;

  fn on_texture<F: FnOnce(&wgpu::Texture) -> U, U>(&self, f: F) -> U;

  fn get_texture_view(&self) -> wgpu::TextureView {
    self.on_texture(|t| t.create_view(&wgpu::TextureViewDescriptor::default()))
  }
}

/// A `Surface` represents a platform-specific surface (e.g. a window).
pub struct PhysicSurface {
  surface: wgpu::Surface,
  s_config: wgpu::SurfaceConfiguration,
}

/// A `Surface` present in a texture. Usually `PhysicSurface` display things to
/// screen(window eg.), But `TextureSurface` is soft, may not display in any
/// device, bug only in memory.
pub type TextureSurface = Texture;

impl Surface for PhysicSurface {
  fn update(
    &mut self,
    device: &wgpu::Device,
    _queue: &wgpu::Queue,
    s_config: &wgpu::SurfaceConfiguration,
  ) {
    self.surface.configure(device, s_config);
  }

  #[inline]
  fn view_size(&self) -> DeviceSize { DeviceSize::new(self.s_config.width, self.s_config.height) }

  fn on_texture<F: FnOnce(&wgpu::Texture) -> U, U>(&self, f: F) -> U {
    let t = self
      .surface
      .get_current_texture()
      .expect("Timeout getting texture");
    f(&t.texture)
  }
}

impl Surface for TextureSurface {
  fn update(
    &mut self,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    s_config: &wgpu::SurfaceConfiguration,
  ) {
    self.resize(
      device,
      queue,
      DeviceSize::new(s_config.width, s_config.height),
    );
  }

  #[inline]
  fn view_size(&self) -> DeviceSize { self.size }

  #[inline]
  fn on_texture<F: FnOnce(&wgpu::Texture) -> U, U>(&self, f: F) -> U { f(&self.raw_texture) }
}

impl PhysicSurface {
  pub(crate) fn new(surface: wgpu::Surface, device: &wgpu::Device, size: DeviceSize) -> Self {
    let s_config = wgpu::SurfaceConfiguration {
      usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
      format: wgpu::TextureFormat::Bgra8UnormSrgb,
      width: size.width,
      height: size.height,
      present_mode: wgpu::PresentMode::Fifo,
    };

    surface.configure(device, &s_config);

    Self { surface, s_config }
  }
}

pub struct Texture {
  pub(crate) raw_texture: wgpu::Texture,
  size: DeviceSize,
  usage: wgpu::TextureUsages,
}

impl Texture {
  pub(crate) fn new(device: &wgpu::Device, size: DeviceSize, usage: wgpu::TextureUsages) -> Self {
    let raw_texture = Self::new_texture(device, size, usage);
    Texture { raw_texture, size, usage }
  }

  pub(crate) fn resize(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, size: DeviceSize) {
    let new_texture = Self::new_texture(device, size, self.usage);

    let size = size.min(self.size);
    let mut encoder = device
      .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Render Encoder") });
    encoder.copy_texture_to_texture(
      wgpu::ImageCopyTexture {
        texture: &self.raw_texture,
        mip_level: 0,
        origin: wgpu::Origin3d::ZERO,
        aspect: wgpu::TextureAspect::All,
      },
      wgpu::ImageCopyTexture {
        texture: &new_texture,
        mip_level: 0,
        origin: wgpu::Origin3d::ZERO,
        aspect: wgpu::TextureAspect::All,
      },
      wgpu::Extent3d {
        width: size.width,
        height: size.height,
        depth_or_array_layers: 1,
      },
    );

    queue.submit(Some(encoder.finish()));

    self.size = size;
    self.raw_texture = new_texture;
  }

  fn new_texture(
    device: &wgpu::Device,
    size: DeviceSize,
    usage: wgpu::TextureUsages,
  ) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
      label: Some("new texture"),
      size: wgpu::Extent3d {
        width: size.width,
        height: size.height,
        depth_or_array_layers: 1,
      },
      dimension: wgpu::TextureDimension::D2,
      format: wgpu::TextureFormat::Bgra8UnormSrgb,
      usage,
      mip_level_count: 1,
      sample_count: 1,
    })
  }
}
