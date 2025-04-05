use std::future::Future;

use ribir_core::{
  prelude::{image::ColorFormat, *},
  window::{ShellWindow, WindowId},
};
use winit::{
  dpi::{LogicalPosition, LogicalSize},
  window::WindowAttributes,
};

#[cfg(target_family = "wasm")]
pub const RIBIR_CANVAS: &str = "ribir_canvas";
#[cfg(target_family = "wasm")]
pub const RIBIR_CONTAINER: &str = "ribir_container";

use crate::{app::App, backends::*, prelude::request_redraw};
pub trait WinitBackend<'a>: Sized {
  fn new(window: &'a winit::window::Window) -> impl Future<Output = Self>;

  fn on_resize(&mut self, size: DeviceSize);

  fn begin_frame(&mut self, surface_color: Color);

  fn draw_commands(
    &mut self, viewport: DeviceRect, global_matrix: &Transform, commands: &[PaintCommand],
  );

  fn end_frame(&mut self);
}

pub struct WinitShellWnd {
  pub(crate) winit_wnd: winit::window::Window,
  backend: Backend<'static>,
  cursor: CursorIcon,
}

impl ShellWindow for WinitShellWnd {
  fn id(&self) -> WindowId { new_id(self.winit_wnd.id()) }

  fn device_pixel_ratio(&self) -> f32 { self.winit_wnd.scale_factor() as f32 }

  fn inner_size(&self) -> Size {
    let size = self
      .winit_wnd
      .inner_size()
      .to_logical(self.winit_wnd.scale_factor());
    Size::new(size.width, size.height)
  }

  fn outer_size(&self) -> Size {
    let size = self
      .winit_wnd
      .outer_size()
      .to_logical(self.winit_wnd.scale_factor());
    Size::new(size.width, size.height)
  }

  fn request_resize(&mut self, size: Size) {
    let size = self
      .winit_wnd
      .request_inner_size(LogicalSize::new(size.width, size.height))
      .map(|size| Size::new(size.width as f32, size.height as f32));
    if let Some(size) = size {
      self.on_resize(size);
      if let Some(wnd) = AppCtx::get_window(self.id()) {
        request_redraw(&wnd);
      }
    }
  }

  fn on_resize(&mut self, size: Size) {
    let size: DeviceSize = (size * self.device_pixel_ratio())
      .ceil()
      .to_i32()
      .cast_unit();
    self.backend.on_resize(size);
  }

  fn set_min_size(&mut self, size: Size) {
    self
      .winit_wnd
      .set_min_inner_size(Some(LogicalSize::new(size.width, size.height)))
  }

  fn set_cursor(&mut self, cursor: CursorIcon) {
    self.cursor = cursor;
    self.winit_wnd.set_cursor(cursor)
  }

  #[inline]
  fn cursor(&self) -> CursorIcon { self.cursor }

  #[inline]
  fn set_title(&mut self, title: &str) { self.winit_wnd.set_title(title) }

  fn set_icon(&mut self, icon: &PixelImage) {
    self
      .winit_wnd
      .set_window_icon(Some(img_to_winit_icon(icon)));
  }

  #[inline]
  fn set_ime_cursor_area(&mut self, rect: &Rect) {
    let position: LogicalPosition<f32> = LogicalPosition::new(rect.origin.x, rect.origin.y);
    let size: LogicalSize<f32> = LogicalSize::new(rect.size.width, rect.size.height);
    self.winit_wnd.set_ime_cursor_area(position, size);
  }

  #[inline]
  fn is_visible(&self) -> Option<bool> { self.winit_wnd.is_visible() }

  #[inline]
  fn set_visible(&mut self, visible: bool) { self.winit_wnd.set_visible(visible) }

  #[inline]
  fn set_resizable(&mut self, resizable: bool) { self.winit_wnd.set_resizable(resizable) }

  #[inline]
  fn is_resizable(&self) -> bool { self.winit_wnd.is_resizable() }

  #[inline]
  fn is_minimized(&self) -> bool { self.winit_wnd.is_minimized().unwrap_or_default() }

  #[inline]
  fn set_minimized(&mut self, minimized: bool) {
    if minimized {
      self.winit_wnd.set_minimized(minimized);
    } else {
      self.winit_wnd.set_visible(true);
      self.winit_wnd.set_minimized(minimized);
    }
  }

  #[inline]
  fn focus_window(&mut self) { self.winit_wnd.focus_window() }

  #[inline]
  fn set_decorations(&mut self, decorations: bool) { self.winit_wnd.set_decorations(decorations) }

  #[inline]
  fn set_ime_allowed(&mut self, allowed: bool) { self.winit_wnd.set_ime_allowed(allowed); }

  #[inline]
  fn as_any(&self) -> &dyn std::any::Any { self }

  #[inline]
  fn as_any_mut(&mut self) -> &mut dyn Any { self }

  #[inline]
  fn begin_frame(&mut self, surface: Color) { self.backend.begin_frame(surface) }

  #[inline]
  fn draw_commands(&mut self, viewport: Rect, commands: &[PaintCommand]) {
    let scale = self.winit_wnd.scale_factor() as f32;
    let viewport: DeviceRect = viewport
      .scale(scale, scale)
      .round_out()
      .to_i32()
      .cast_unit();

    self.winit_wnd.pre_present_notify();
    self
      .backend
      .draw_commands(viewport, &Transform::scale(scale, scale), commands);
  }

  #[inline]
  fn end_frame(&mut self) { self.backend.end_frame() }
}

pub(crate) fn new_id(id: winit::window::WindowId) -> WindowId {
  let id: u64 = id.into();
  id.into()
}

impl WinitShellWnd {
  #[cfg(target_family = "wasm")]
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

  #[cfg(not(target_family = "wasm"))]
  pub(crate) async fn new(attrs: WindowAttributes) -> Self { Self::inner_new(attrs).await }

  async fn inner_new(attrs: WindowAttributes) -> Self {
    let winit_wnd = App::active_event_loop()
      .create_window(attrs)
      .unwrap();
    let ptr = &winit_wnd as *const winit::window::Window;
    // Safety: a reference to winit_wnd is valid as long as the WinitShellWnd is
    // alive.
    let backend = Backend::new(unsafe { &*ptr }).await;
    WinitShellWnd { backend, winit_wnd, cursor: CursorIcon::Default }
  }
}

fn img_to_winit_icon(icon: &PixelImage) -> winit::window::Icon {
  assert!(icon.color_format() == ColorFormat::Rgba8);
  winit::window::Icon::from_rgba(icon.pixel_bytes().to_vec(), icon.width(), icon.height()).unwrap()
}
