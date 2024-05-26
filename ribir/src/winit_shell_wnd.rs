use std::future::Future;

use ribir_core::{
  prelude::{image::ColorFormat, *},
  window::{ShellWindow, WindowId},
};
use winit::{
  dpi::{LogicalPosition, LogicalSize},
  event_loop::EventLoopWindowTarget,
};

use crate::{
  backends::*,
  prelude::{request_redraw, WindowAttributes},
};
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
    self.winit_wnd.set_cursor_icon(cursor)
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
  pub(crate) async fn new_with_canvas<T>(
    canvas: web_sys::HtmlCanvasElement, window_target: &EventLoopWindowTarget<T>,
    attrs: WindowAttributes,
  ) -> Self {
    use winit::platform::web::WindowBuilderExtWebSys;
    let builder = winit::window::WindowBuilder::new().with_canvas(Some(canvas));

    Self::inner_wnd(builder, window_target, attrs).await
  }

  #[cfg(target_family = "wasm")]
  pub(crate) async fn new<T>(
    window_target: &EventLoopWindowTarget<T>, attrs: WindowAttributes,
  ) -> Self {
    const RIBIR_CANVAS: &str = "ribir_canvas";
    const RIBIR_CANVAS_USED: &str = "ribir_canvas_used";

    use web_sys::{wasm_bindgen::JsCast, HtmlCanvasElement};
    let document = web_sys::window().unwrap().document().unwrap();
    let elems = document.get_elements_by_class_name(RIBIR_CANVAS);

    let mut canvas = None;
    let len = elems.length();
    for idx in 0..len {
      if let Some(elem) = elems.get_with_index(idx) {
        let mut classes_name = elem.class_name();
        if !classes_name
          .split(" ")
          .any(|v| v == RIBIR_CANVAS_USED)
        {
          if let Ok(c) = elem.clone().dyn_into::<HtmlCanvasElement>() {
            classes_name.push_str(&format!(" {}", RIBIR_CANVAS_USED));
            elem.set_class_name(&classes_name);
            canvas = Some(c);
          } else {
            let child = document.create_element("canvas").unwrap();
            elem.append_child(&child).unwrap();
            canvas = Some(child.dyn_into::<HtmlCanvasElement>().unwrap())
          }
          break;
        }
      }
    }

    let canvas = canvas.expect("No unused 'ribir_canvas' class element found.");

    return Self::new_with_canvas(canvas, window_target, attrs).await;
  }

  #[cfg(not(target_family = "wasm"))]
  pub(crate) async fn new<T>(
    window_target: &EventLoopWindowTarget<T>, attrs: WindowAttributes,
  ) -> Self {
    Self::inner_wnd(winit::window::WindowBuilder::new(), window_target, attrs).await
  }

  async fn inner_wnd<T>(
    mut builder: winit::window::WindowBuilder, window_target: &EventLoopWindowTarget<T>,
    attrs: WindowAttributes,
  ) -> Self {
    builder = builder
      .with_title(attrs.title)
      .with_maximized(attrs.maximized)
      .with_resizable(attrs.resizable)
      // hide the window until the render backend is ready
      .with_visible(false)
      .with_decorations(attrs.decorations);

    if let Some(size) = attrs.size {
      builder = builder.with_inner_size(LogicalSize::new(size.width, size.height));
    }
    if let Some(min_size) = attrs.min_size {
      builder = builder.with_min_inner_size(LogicalSize::new(min_size.width, min_size.height));
    }
    if let Some(max_size) = attrs.max_size {
      builder = builder.with_max_inner_size(LogicalSize::new(max_size.width, max_size.height));
    }
    if let Some(pos) = attrs.position {
      builder = builder.with_position(LogicalPosition::new(pos.x, pos.y));
    }
    if let Some(icon) = attrs.icon {
      builder = builder.with_window_icon(Some(img_to_winit_icon(&icon)));
    }

    let winit_wnd: winit::window::Window = builder.build(window_target).unwrap();
    let ptr = &winit_wnd as *const winit::window::Window;
    // Safety: a reference to winit_wnd is valid as long as the WinitShellWnd is
    // alive.
    let backend = Backend::new(unsafe { &*ptr }).await;

    // show the window after the render backend is ready
    if attrs.visible {
      winit_wnd.set_visible(attrs.visible);
    }
    WinitShellWnd { backend, winit_wnd, cursor: CursorIcon::Default }
  }
}

fn img_to_winit_icon(icon: &PixelImage) -> winit::window::Icon {
  assert!(icon.color_format() == ColorFormat::Rgba8);
  winit::window::Icon::from_rgba(icon.pixel_bytes().to_vec(), icon.width(), icon.height()).unwrap()
}
