use crate::{backends::*, prelude::request_redraw};
use ribir_core::{
  prelude::{image::ColorFormat, *},
  window::{ShellWindow, WindowId},
};
use std::future::Future;
use winit::{
  dpi::{LogicalPosition, LogicalSize},
  event_loop::EventLoopWindowTarget,
};
pub trait WinitBackend<'a>: Sized {
  fn new(window: &'a winit::window::Window) -> impl Future<Output = Self>;

  fn on_resize(&mut self, size: DeviceSize);

  fn begin_frame(&mut self);

  fn draw_commands(
    &mut self,
    viewport: DeviceRect,
    commands: Vec<PaintCommand>,
    surface_color: Color,
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

  #[inline]
  fn set_icon(&mut self, icon: &PixelImage) {
    assert!(icon.color_format() == ColorFormat::Rgba8);
    let win_icon =
      winit::window::Icon::from_rgba(icon.pixel_bytes().to_vec(), icon.width(), icon.height())
        .unwrap();
    self.winit_wnd.set_window_icon(Some(win_icon));
  }

  #[inline]
  fn set_ime_cursor_area(&mut self, rect: &Rect) {
    let position: LogicalPosition<f32> = LogicalPosition::new(rect.origin.x, rect.origin.y);
    let size: LogicalSize<f32> = LogicalSize::new(rect.size.width, rect.size.height);
    self.winit_wnd.set_ime_cursor_area(position, size);
  }

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
  fn begin_frame(&mut self) { self.backend.begin_frame() }

  #[inline]
  fn draw_commands(&mut self, viewport: Rect, mut commands: Vec<PaintCommand>, surface: Color) {
    commands.iter_mut().for_each(|c| match c {
      PaintCommand::ColorPath { path, .. }
      | PaintCommand::ImgPath { path, .. }
      | PaintCommand::RadialGradient { path, .. }
      | PaintCommand::LinearGradient { path, .. }
      | PaintCommand::Clip(path) => path.scale(self.winit_wnd.scale_factor() as f32),
      PaintCommand::PopClip => {}
    });

    let scale = self.winit_wnd.scale_factor() as f32;
    let viewport: DeviceRect = viewport
      .scale(scale, scale)
      .round_out()
      .to_i32()
      .cast_unit();

    self.backend.draw_commands(viewport, commands, surface);
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
  pub(crate) fn new_with_canvas<T>(
    canvas: web_sys::HtmlCanvasElement,
    window_target: &EventLoopWindowTarget<T>,
  ) -> Self {
    use winit::platform::web::WindowBuilderExtWebSys;
    let wnd = winit::window::WindowBuilder::new()
      .with_canvas(Some(canvas))
      .build(window_target)
      .unwrap();

    Self::inner_wnd(wnd)
  }

  pub(crate) fn new<T>(size: Option<Size>, window_target: &EventLoopWindowTarget<T>) -> Self {
    let mut winit_wnd = winit::window::WindowBuilder::new();
    if let Some(size) = size {
      winit_wnd = winit_wnd.with_inner_size(LogicalSize::new(size.width, size.height));
    }

    // A canvas need configure in web platform.
    #[cfg(target_family = "wasm")]
    {
      use winit::platform::web::WindowBuilderExtWebSys;
      const RIBIR_CANVAS: &str = "ribir_canvas";
      const RIBIR_CANVAS_USED: &str = "ribir_canvas_used";

      use web_sys::wasm_bindgen::JsCast;
      let document = web_sys::window().unwrap().document().unwrap();
      let elems = document.get_elements_by_class_name(RIBIR_CANVAS);

      let len = elems.length();
      for idx in 0..len {
        if let Some(elem) = elems.get_with_index(idx) {
          let mut classes_name = elem.class_name();
          if !classes_name.split(" ").any(|v| v == RIBIR_CANVAS_USED) {
            let mut canvas = elem.clone().dyn_into::<web_sys::HtmlCanvasElement>();
            if canvas.is_err() {
              let child = document.create_element("canvas").unwrap();
              if elem.append_child(&child).is_ok() {
                canvas = child.dyn_into::<web_sys::HtmlCanvasElement>();
              }
            }
            if let Ok(canvas) = canvas {
              classes_name.push_str(&format!(" {}", RIBIR_CANVAS_USED));
              elem.set_class_name(&classes_name);
              winit_wnd = winit_wnd.with_canvas(Some(canvas));
            }
          }
        }
      }
    }

    let wnd = winit_wnd.build(window_target).unwrap();
    Self::inner_wnd(wnd)
  }

  fn inner_wnd(winit_wnd: winit::window::Window) -> Self {
    let ptr = &winit_wnd as *const winit::window::Window;
    // Safety: a reference to winit_wnd is valid as long as the WinitShellWnd is
    // alive.
    let backend = Backend::new(unsafe { &*ptr });
    let backend = AppCtx::wait_future(backend);
    WinitShellWnd {
      backend,
      winit_wnd,
      cursor: CursorIcon::Default,
    }
  }
}
