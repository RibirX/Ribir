use std::{future::Future, sync::Arc};

use ribir_core::{
  prelude::{image::ColorFormat, *},
  scheduler::BoxFuture,
  window::{BoxShellWindow, Shell, ShellWindow, WindowId, WindowLevel},
};
use winit::{
  dpi::{LogicalPosition, LogicalSize},
  window::WindowAttributes,
};

#[cfg(target_arch = "wasm32")]
pub const RIBIR_CANVAS: &str = "ribir_canvas";
#[cfg(target_arch = "wasm32")]
pub const RIBIR_CONTAINER: &str = "ribir_container";

use crate::{
  app::{App, CmdSender},
  backends::*,
};

pub enum ShellCmd {
  Exit,
  RequestDraw {
    id: WindowId,
  },
  Draw {
    id: WindowId,
    wnd_size: Size,
    viewport: Rect,
    surface_color: Color,
    commands: Vec<PaintCommand>,
  },
  Close {
    id: WindowId,
  },
  RunAsync {
    fut: BoxFuture<'static, ()>,
  },
}

impl ShellCmd {
  pub fn wnd_id(&self) -> Option<WindowId> {
    match self {
      ShellCmd::RequestDraw { id } | ShellCmd::Draw { id, .. } | ShellCmd::Close { id } => {
        Some(*id)
      }
      ShellCmd::RunAsync { .. } | ShellCmd::Exit => None,
    }
  }
}

pub(crate) struct RibirShell {
  pub(crate) cmd_sender: CmdSender,
}

impl Shell for RibirShell {
  fn new_shell_window(
    &self, attrs: ribir_core::window::WindowAttributes,
  ) -> scheduler::BoxFuture<'static, BoxShellWindow> {
    let (sender, receiver) = tokio::sync::oneshot::channel::<BoxShellWindow>();

    self.run_in_shell(Box::pin(async move {
      let _ = sender.send(App::new_window(attrs).await);
    }));

    Box::pin(async move { receiver.await.unwrap() })
  }

  fn exit(&self) { self.cmd_sender.send(ShellCmd::Exit); }

  fn run_in_shell(&self, fut: BoxFuture<'static, ()>) {
    self.cmd_sender.send(ShellCmd::RunAsync { fut });
  }
}

pub(crate) struct ShellWndHandle {
  pub(crate) sender: CmdSender,
  pub(crate) winit_wnd: Arc<winit::window::Window>,
  pub(crate) cursor: CursorIcon,
}

pub trait WinitBackend<'a>: Sized {
  fn new(window: &'a winit::window::Window) -> impl Future<Output = Self>;

  fn on_resize(&mut self, size: DeviceSize);

  fn begin_frame(&mut self, surface_color: Color);

  fn draw_commands(
    &mut self, viewport: DeviceRect, global_matrix: &Transform, commands: &[PaintCommand],
  );

  fn end_frame(&mut self);
}

pub(crate) struct WinitShellWnd {
  pub(crate) winit_wnd: Arc<winit::window::Window>,
  backend: Backend<'static>,
}

fn window_size(winit_wnd: &winit::window::Window) -> Size {
  let size = winit_wnd
    .inner_size()
    .to_logical(winit_wnd.scale_factor());
  Size::new(size.width, size.height)
}

impl WinitShellWnd {
  pub(crate) fn id(&self) -> WindowId { new_id(self.winit_wnd.id()) }

  pub(crate) fn deal_cmd(&mut self, cmd: ShellCmd) {
    match cmd {
      ShellCmd::RequestDraw { .. } => self.winit_wnd.request_redraw(),
      ShellCmd::Draw { viewport, surface_color, commands, wnd_size, .. } => {
        if wnd_size == window_size(&self.winit_wnd) {
          self.backend.begin_frame(surface_color);
          let scale_factor = self.winit_wnd.scale_factor() as f32;

          let viewport: DeviceRect = viewport
            .scale(scale_factor, scale_factor)
            .round_out()
            .to_i32()
            .cast_unit();

          self.backend.draw_commands(
            viewport,
            &Transform::scale(scale_factor, scale_factor),
            &commands,
          );
          self.backend.end_frame();
        } else {
          self.winit_wnd.request_redraw();
        }
      }
      ShellCmd::Close { id } => {
        App::remove_shell_window(id);
      }
      _ => (),
    }
  }

  pub(crate) fn on_resize(&mut self, size: DeviceSize) { self.backend.on_resize(size); }
}

impl ShellWindow for ShellWndHandle {
  fn id(&self) -> WindowId { new_id(self.winit_wnd.id()) }

  fn inner_size(&self) -> Size { window_size(&self.winit_wnd) }

  fn as_any(&self) -> &dyn Any { self }

  fn as_any_mut(&mut self) -> &mut dyn Any { self }

  fn set_cursor(&mut self, cursor: CursorIcon) {
    self.cursor = cursor;
    self.winit_wnd.set_cursor(cursor);
  }

  fn cursor(&self) -> CursorIcon { self.cursor }

  fn focus_window(&mut self) {
    self.winit_wnd.focus_window();
    self.winit_wnd.request_redraw();
  }

  fn set_minimized(&mut self, minimized: bool) { self.winit_wnd.set_minimized(minimized); }

  fn is_minimized(&self) -> bool { self.winit_wnd.is_minimized().unwrap_or_default() }

  fn set_min_size(&mut self, size: Size) {
    self
      .winit_wnd
      .set_min_inner_size(Some(LogicalSize::new(size.width, size.height)));
  }

  fn set_icon(&mut self, icon: &PixelImage) {
    self
      .winit_wnd
      .set_window_icon(Some(img_to_winit_icon(icon)));
  }

  fn close(&self) {
    self
      .sender
      .send(ShellCmd::Close { id: self.id() });
  }

  fn set_title(&mut self, str: &str) { self.winit_wnd.set_title(str); }

  fn set_resizable(&mut self, resizable: bool) { self.winit_wnd.set_resizable(resizable); }

  fn is_resizable(&self) -> bool { self.winit_wnd.is_resizable() }

  fn set_visible(&mut self, visible: bool) { self.winit_wnd.set_visible(visible); }

  fn is_visible(&self) -> Option<bool> { self.winit_wnd.is_visible() }

  fn set_ime_allowed(&mut self, allowed: bool) { self.winit_wnd.set_ime_allowed(allowed); }

  fn set_window_level(&mut self, level: WindowLevel) { self.winit_wnd.set_window_level(level); }

  fn set_ime_cursor_area(&mut self, rect: &Rect) {
    let position: LogicalPosition<f32> = LogicalPosition::new(rect.origin.x, rect.origin.y);
    let size: LogicalSize<f32> = LogicalSize::new(rect.size.width, rect.size.height);
    self.winit_wnd.set_ime_cursor_area(position, size);
  }

  fn request_resize(&mut self, size: Size) {
    let size = self
      .winit_wnd
      .request_inner_size(LogicalSize::new(size.width, size.height))
      .map(|size| Size::new(size.width as f32, size.height as f32));
    if size.is_some()
      && let Some(wnd) = AppCtx::get_window(self.id())
    {
      wnd.shell_wnd().borrow().request_draw();
    }
  }

  fn draw_commands(
    &mut self, wnd_size: Size, viewport: Rect, surface_color: Color, commands: &[PaintCommand],
  ) {
    self.sender.send(ShellCmd::Draw {
      id: self.id(),
      wnd_size,
      viewport,
      surface_color,
      commands: commands.to_vec(),
    });
  }

  fn request_draw(&self) {
    self
      .sender
      .send(ShellCmd::RequestDraw { id: self.id() });
  }

  fn position(&self) -> Point {
    let scale_factor = self.winit_wnd.scale_factor() as f32;
    self
      .winit_wnd
      .outer_position()
      .map(|pos| Point::new(pos.x as f32 / scale_factor, pos.y as f32 / scale_factor))
      .unwrap_or_default()
  }

  fn set_position(&mut self, point: Point) {
    let pos = self.position();
    if pos != point {
      self
        .winit_wnd
        .set_outer_position(LogicalPosition::new(point.x, point.y));
    }
  }
}

pub(crate) fn new_id(id: winit::window::WindowId) -> WindowId {
  let id: u64 = id.into();
  id.into()
}

impl WinitShellWnd {
  #[cfg(target_arch = "wasm32")]
  pub(crate) async fn new(mut attrs: WindowAttributes) -> Self {
    use web_sys::wasm_bindgen::JsCast;
    use winit::platform::web::WindowAttributesExtWebSys;

    let document = web_sys::window().unwrap().document().unwrap();
    let canvas = document
      .create_element("canvas")
      .unwrap()
      .dyn_into::<web_sys::HtmlCanvasElement>()
      .unwrap();
    canvas.set_class_name(RIBIR_CANVAS);
    let style = canvas.style();
    let _ = style.set_property("width", "100%");
    let _ = style.set_property("height", "100%");
    let elems = document.get_elements_by_class_name(RIBIR_CONTAINER);

    if let Some(elem) = elems.item(0) {
      elem.set_class_name(&elem.class_name().replace(RIBIR_CONTAINER, ""));
      elem.append_child(&canvas).unwrap();
    } else if let Some(body) = document.body() {
      body.append_child(&canvas).unwrap();
    } else {
      document.append_child(&canvas).unwrap();
    }

    attrs = attrs.with_canvas(Some(canvas));
    let wnd = Self::inner_new(attrs).await;

    wnd
  }

  #[cfg(not(target_arch = "wasm32"))]
  pub(crate) async fn new(attrs: WindowAttributes) -> Self { Self::inner_new(attrs).await }

  async fn inner_new(attrs: WindowAttributes) -> Self {
    let winit_wnd = Arc::new(
      App::active_event_loop()
        .create_window(attrs)
        .unwrap(),
    );
    let ptr = winit_wnd.as_ref() as *const winit::window::Window;
    // Safety: a reference to winit_wnd is valid as long as the WinitShellWnd is
    // alive.
    let backend = Backend::new(unsafe { &*ptr }).await;
    WinitShellWnd { backend, winit_wnd }
  }
}

fn img_to_winit_icon(icon: &PixelImage) -> winit::window::Icon {
  assert!(icon.color_format() == ColorFormat::Rgba8);
  winit::window::Icon::from_rgba(icon.pixel_bytes().to_vec(), icon.width(), icon.height()).unwrap()
}
