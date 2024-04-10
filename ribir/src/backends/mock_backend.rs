use crate::winit_shell_wnd::WinitBackend;

pub struct MockBackend;

impl WinitBackend for MockBackend {
  fn new(_: &winit::window::Window) -> Self { Self }

  fn on_resize(&mut self, _: ribir_core::prelude::DeviceSize) {}

  fn begin_frame(&mut self) {}

  fn draw_commands(
    &mut self, _: ribir_core::prelude::DeviceRect, _: Vec<ribir_core::prelude::PaintCommand>,
    _: ribir_core::prelude::Color,
  ) {
  }

  fn end_frame(&mut self) {}
}
