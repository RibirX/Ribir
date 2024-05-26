use ribir_core::prelude::{Color, DeviceRect, DeviceSize, PaintCommand, PainterBackend, Transform};
use ribir_gpu::Surface;

use crate::winit_shell_wnd::WinitBackend;

pub struct WgpuBackend<'a> {
  surface: Surface<'a>,
  backend: ribir_gpu::GPUBackend<ribir_gpu::WgpuImpl>,
}

impl<'a> WinitBackend<'a> for WgpuBackend<'a> {
  async fn new(window: &'a winit::window::Window) -> WgpuBackend<'a> {
    let (wgpu, surface) = ribir_gpu::WgpuImpl::new(window).await;
    let size = window.inner_size();
    let size = DeviceSize::new(size.width as i32, size.height as i32);

    let mut wgpu = WgpuBackend { surface, backend: ribir_gpu::GPUBackend::new(wgpu) };
    wgpu.on_resize(size);

    wgpu
  }

  fn on_resize(&mut self, size: DeviceSize) {
    if size != self.surface.size() {
      self.surface.resize(size, self.backend.get_impl());
    }
  }

  fn begin_frame(&mut self, surface_color: Color) { self.backend.begin_frame(surface_color); }

  fn draw_commands(
    &mut self, viewport: DeviceRect, global_matrix: &Transform, commands: &[PaintCommand],
  ) {
    self.backend.draw_commands(
      viewport,
      commands,
      global_matrix,
      self.surface.get_current_texture(),
    );
  }

  fn end_frame(&mut self) {
    self.backend.end_frame();
    self.surface.present();
  }
}
