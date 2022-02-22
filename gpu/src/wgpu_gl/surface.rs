use std::cell::{Ref, RefCell};

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

  fn current_texture(&self) -> SurfaceTexture;

  fn present(&mut self);
}

/// A `Surface` represents a platform-specific surface (e.g. a window).
pub struct WindowSurface {
  surface: wgpu::Surface,
  s_config: wgpu::SurfaceConfiguration,
  current_texture: RefCell<Option<wgpu::SurfaceTexture>>,
}

/// A `Surface` present in a texture. Usually `PhysicSurface` display things to
/// screen(window eg.), But `TextureSurface` is soft, may not display in any
/// device, bug only in memory.
pub type TextureSurface = Texture;

impl Surface for WindowSurface {
  fn update(
    &mut self,
    device: &wgpu::Device,
    _queue: &wgpu::Queue,
    s_config: &wgpu::SurfaceConfiguration,
  ) {
    self.surface.configure(device, s_config);
  }

  fn view_size(&self) -> DeviceSize { DeviceSize::new(self.s_config.width, self.s_config.height) }

  fn current_texture(&self) -> SurfaceTexture {
    self.current_texture.borrow_mut().get_or_insert_with(|| {
      self
        .surface
        .get_current_texture()
        .expect("Timeout getting texture")
    });
    SurfaceTexture::RefCell(Ref::map(self.current_texture.borrow(), |t| {
      &t.as_ref().unwrap().texture
    }))
  }

  fn present(&mut self) {
    if let Some(texture) = self.current_texture.take() {
      texture.present()
    }
  }
}

pub enum SurfaceTexture<'a> {
  RefCell(Ref<'a, wgpu::Texture>),
  Ref(&'a wgpu::Texture),
}

impl<'a> std::ops::Deref for SurfaceTexture<'a> {
  type Target = wgpu::Texture;

  fn deref(&self) -> &Self::Target {
    match self {
      SurfaceTexture::RefCell(t) => &*t,
      SurfaceTexture::Ref(t) => t,
    }
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

  fn view_size(&self) -> DeviceSize { self.size }

  fn current_texture(&self) -> SurfaceTexture { SurfaceTexture::Ref(&self.raw_texture) }

  fn present(&mut self) {}
}

impl WindowSurface {
  pub(crate) fn new(
    surface: wgpu::Surface,
    adapter: &wgpu::Adapter,
    device: &wgpu::Device,
    size: DeviceSize,
  ) -> Self {
    let s_config = wgpu::SurfaceConfiguration {
      usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
      format: surface.get_preferred_format(adapter).unwrap(),
      width: size.width,
      height: size.height,
      present_mode: wgpu::PresentMode::Fifo,
    };

    surface.configure(device, &s_config);

    Self {
      surface,
      s_config,
      current_texture: RefCell::new(None),
    }
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
      format: wgpu::TextureFormat::Rgba8UnormSrgb,
      usage,
      mip_level_count: 1,
      sample_count: 1,
    })
  }
}
