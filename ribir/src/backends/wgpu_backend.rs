use ribir_core::prelude::{
  AntiAliasing, Color, DeviceRect, DeviceSize, PaintCommand, PainterBackend,
};
use ribir_gpu::WgpuTexture;

use crate::winit_shell_wnd::WinitBackend;

pub struct WgpuBackend<'a> {
  size: DeviceSize,
  surface: wgpu::Surface<'a>,
  backend: ribir_gpu::GPUBackend<ribir_gpu::WgpuImpl>,
  current_texture: Option<ribir_gpu::WgpuTexture>,
}

impl<'a> WinitBackend<'a> for WgpuBackend<'a> {
  async fn new(window: &'a winit::window::Window) -> WgpuBackend<'a> {
    let instance = wgpu::Instance::new(<_>::default());
    let surface = instance.create_surface(window).unwrap();
    let wgpu = ribir_gpu::WgpuImpl::new(instance, Some(&surface)).await;
    let size = window.inner_size();
    surface.configure(
      wgpu.device(),
      &Self::surface_config(size.width, size.height),
    );

    WgpuBackend {
      size: DeviceSize::new(size.width as i32, size.height as i32),
      surface,
      backend: ribir_gpu::GPUBackend::new(wgpu, AntiAliasing::Msaa4X),
      current_texture: None,
    }
  }

  fn on_resize(&mut self, size: DeviceSize) {
    if !size.is_empty() && size != self.size {
      self.size = size;
      self.surface.configure(
        self.backend.get_impl().device(),
        &Self::surface_config(size.width as u32, size.height as u32),
      );
    }
  }

  fn begin_frame(&mut self) {
    self.backend.begin_frame();
    assert!(self.current_texture.is_none());
    let surface_tex = self.surface.get_current_texture().unwrap();
    self.current_texture = Some(WgpuTexture::from_surface_tex(surface_tex));
  }

  fn draw_commands(
    &mut self,
    viewport: DeviceRect,
    commands: Vec<PaintCommand>,
    surface_color: Color,
  ) {
    let surface = self.current_texture.as_mut().unwrap();

    self
      .backend
      .draw_commands(viewport, commands, surface_color, surface);
  }

  fn end_frame(&mut self) {
    self.backend.end_frame();
    let surface = self
      .current_texture
      .take()
      .unwrap()
      .into_surface_texture()
      .unwrap();
    surface.present();
  }
}

impl<'a> WgpuBackend<'a> {
  fn surface_config(width: u32, height: u32) -> wgpu::SurfaceConfiguration {
    wgpu::SurfaceConfiguration {
      usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
      format: wgpu::TextureFormat::Bgra8Unorm,
      width,
      height,
      present_mode: wgpu::PresentMode::Fifo,
      alpha_mode: wgpu::CompositeAlphaMode::Auto,
      view_formats: vec![wgpu::TextureFormat::Bgra8Unorm],
      desired_maximum_frame_latency: 2,
    }
  }
}
