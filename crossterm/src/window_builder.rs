use crossterm::{
  cursor::{Hide, MoveTo, Show},
  event::EnableMouseCapture,
  execute, queue,
  style::Print,
  terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use ribir_core::{
  widget::Widget,
  window::{Window as RibirWindow, WindowConfig},
};
use ribir_painter::PainterBackend;
use std::io::{stdout, Write};

use ribir_geometry::DeviceSize;

use crate::from_window::CrosstermWindow;
use crate::prelude::{CrosstermWindowId, PlatformShellWindow};

struct CrosstermBackend {
  write: Box<dyn Write>,
}

impl CrosstermBackend {
  fn new(mut write: Box<dyn Write>) -> Self {
    let _ = execute!(write, EnterAlternateScreen, Hide); //"PainterBackend commands_to_image");
    let _ = execute!(write, EnableMouseCapture);
    let _ = terminal::enable_raw_mode();
    CrosstermBackend { write }
  }
}

impl Drop for CrosstermBackend {
  fn drop(&mut self) {
    println!("CrosstermBackend dropped");
    let _ = execute!(self.write, Show, LeaveAlternateScreen); // restore the cursor and leave the alternate screen
    let _ = terminal::disable_raw_mode();
  }
}

impl PainterBackend for CrosstermBackend {
  fn submit(&mut self, commands: Vec<ribir_painter::PaintCommand>) {
    if !commands.is_empty() {
      println!("PainterBackend submit {:?}", commands.len());
      for c in commands {
        println!("Command {c:?}");
      }
    }
    match self.write.flush() {
      Ok(()) => (),
      Err(err) => println!("PainterBackend submit {err:?}"),
    }
  }

  fn commands_to_image(
    &mut self,
    commands: Vec<ribir_painter::PaintCommand>,
    _capture: ribir_painter::CaptureCallback,
  ) -> Result<(), Box<dyn std::error::Error>> {
    println!("PainterBackend commands_to_image {:?}", commands.len());
    queue!(self.write, EnterAlternateScreen, Hide)?; //"PainterBackend commands_to_image");
    queue!(
      self.write,
      MoveTo(1, 1),
      Print("PainterBackend commands_to_image")
    )?; //"PainterBackend commands_to_image");
    println!("PainterBackend commands_to_image");
    Ok(())
  }

  fn resize(&mut self, size: DeviceSize) {
    println!("PainterBackend resize {size:?}");
  }
}

pub struct WindowBuilder {
  root: Widget,
}

impl WindowBuilder {
  #[inline]
  pub fn new(root: Widget, _config: WindowConfig) -> WindowBuilder {
    WindowBuilder {
      root,
      // inner_builder: winit::window::WindowBuilder::default(),
    }
    // TODO(zoechi) apply config
  }

  #[inline]
  pub fn build(self, shell_window: &PlatformShellWindow) -> RibirWindow {
    let native_wnd = CrosstermWindow::new(CrosstermWindowId::zero());
    let ctx = shell_window.context().clone();
    let p_backend = CrosstermBackend::new(Box::new(stdout()));
    RibirWindow::new(native_wnd, p_backend, self.root, ctx)
  }
}
