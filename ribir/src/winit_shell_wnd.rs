use ribir_core::{
  prelude::*,
  window::{ShellWindow, WindowId},
};

pub struct WinitShellWnd {
  winit_wnd: winit::window::Window,
}

impl ShellWindow for WinitShellWnd {
  fn new(size: Option<Size>) -> Self { todo!() }

  fn id(&self) -> WindowId { new_id(self.winit_wnd.id()) }

  fn size(&self) -> Size { todo!() }

  fn device_scale(&self) -> f32 { todo!() }

  fn set_size(&mut self, size: Size) { todo!() }

  fn set_cursor(&mut self, cursor: CursorIcon) { todo!() }

  fn as_any(&self) -> &dyn std::any::Any { todo!() }

  fn begin_frame(&mut self) { todo!() }

  fn draw_commands(&mut self, commands: Vec<PaintCommand>) { todo!() }

  fn end_frame(&mut self) { todo!() }
}

pub(crate) fn new_id(id: winit::window::WindowId) -> WindowId {
  let id: u64 = id.into();
  id.into()
}
