use ribir_core::prelude::{
  AntiAliasing, AppCtx, Color, DeviceRect, DeviceSize, PaintCommand, PainterBackend,
};
use ribir_gpu::WgpuTexture;

use crate::winit_shell_wnd::WinitBackend;

pub struct WgpuBackend {
  surface: wgpu::Surface,
  backend: ribir_gpu::GPUBackend<ribir_gpu::WgpuImpl>,
  current_texture: Option<ribir_gpu::WgpuTexture>,
}

impl WinitBackend for WgpuBackend {
  fn new(window: &winit::window::Window) -> WgpuBackend {
    let instance = wgpu::Instance::new(<_>::default());
    let surface = unsafe { instance.create_surface(window).unwrap() };
    let wgpu = AppCtx::wait_future(ribir_gpu::WgpuImpl::new(instance, Some(&surface)));
    let size = window.inner_size();
    surface.configure(
      wgpu.device(),
      &Self::surface_config(size.width, size.height),
    );

    WgpuBackend {
      surface,
      backend: ribir_gpu::GPUBackend::new(wgpu, AntiAliasing::Msaa4X),
      current_texture: None,
    }
  }

  fn on_resize(&mut self, size: DeviceSize) {
    if !size.is_empty() {
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

impl WgpuBackend {
  fn surface_config(width: u32, height: u32) -> wgpu::SurfaceConfiguration {
    wgpu::SurfaceConfiguration {
      usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
      format: wgpu::TextureFormat::Bgra8Unorm,
      width,
      height,
      present_mode: wgpu::PresentMode::Fifo,
      alpha_mode: wgpu::CompositeAlphaMode::Auto,
      view_formats: vec![wgpu::TextureFormat::Bgra8Unorm],
    }
  }
}
