use crate::{context::Context, events::dispatcher::Dispatcher, prelude::*};

pub use winit::window::CursorIcon;
use winit::{event::WindowEvent, window::WindowId};

const TOLERANCE: f32 = 0.01;
pub trait RawWindow {
  fn inner_size(&self) -> Size;
  fn outer_size(&self) -> Size;
  fn inner_position(&self) -> Point;
  fn outer_position(&self) -> Point;
  fn id(&self) -> WindowId;

  fn request_redraw(&self);
  /// Modify the native window if cursor modified.
  fn set_cursor(&mut self, cursor: CursorIcon);
  fn scale_factor(&self) -> f64;
}

impl RawWindow for winit::window::Window {
  fn inner_size(&self) -> Size {
    let size = self.inner_size().to_logical(self.scale_factor());
    Size::new(size.width, size.height)
  }

  fn outer_size(&self) -> Size {
    let size = self.outer_size().to_logical(self.scale_factor());
    Size::new(size.width, size.height)
  }

  fn inner_position(&self) -> Point {
    let pos = self
      .inner_position()
      .expect(" Can only be called on the main thread")
      .to_logical(self.scale_factor());

    Point::new(pos.x, pos.y)
  }
  #[inline]
  fn id(&self) -> WindowId { self.id() }

  fn outer_position(&self) -> Point {
    let pos = self
      .outer_position()
      .expect(" Can only be called on the main thread")
      .to_logical(self.scale_factor());
    Point::new(pos.x, pos.y)
  }

  #[inline]
  fn request_redraw(&self) { winit::window::Window::request_redraw(self) }

  fn set_cursor(&mut self, cursor: CursorIcon) { self.set_cursor_icon(cursor) }

  #[inline]
  fn scale_factor(&self) -> f64 { winit::window::Window::scale_factor(self) }
}

/// Window is the root to represent.
pub struct Window {
  pub raw_window: Box<dyn RawWindow>,
  pub(crate) context: Context,
  p_backend: Box<dyn PainterBackend>,
  pub(crate) dispatcher: Dispatcher,
}

impl Window {
  /// processes native events from this native window
  #[inline]
  pub fn processes_native_event(&mut self, event: WindowEvent) {
    match event {
      WindowEvent::Resized(size) => {
        self.resize(DeviceSize::new(size.width, size.height));
      }
      WindowEvent::ScaleFactorChanged { new_inner_size, scale_factor } => {
        self.resize(DeviceSize::new(new_inner_size.width, new_inner_size.height));
        let factor = scale_factor as f32;
        self.context.painter.reset(Some(factor));
      }
      event => self.dispatcher.dispatch(event, &mut self.context),
    };
    if let Some(icon) = self.context.cursor.take() {
      self.raw_window.set_cursor(icon);
    }
  }

  /// This method ensure render tree is ready to paint, three things it's have
  /// to do:
  /// 1. every need rebuild widgets has rebuild and correspond render tree
  /// construct.
  /// 2. every dirty widget has flush to render tree so render tree's data
  /// represent the latest application state.
  /// 3. every render objet need layout has done, so every render object is in
  /// the correct position.
  pub fn render_ready(&mut self) -> bool {
    self.context.state_change_dispatch();

    let tree_changed = self.context.tree_repair();
    let performed_layout = self.context.layout_store.layout(
      self.raw_window.inner_size(),
      &self.context.widget_tree,
      &mut self.context.shaper,
    );

    if tree_changed {
      self.dispatcher.focus_mgr.update(&mut self.context);
    }
    tree_changed || performed_layout
  }

  /// Draw an image what current render tree represent.
  pub(crate) fn draw_frame(&mut self) {
    let commands = self.context.draw_tree();
    if !commands.is_empty() {
      self.p_backend.submit(commands);
    }
  }

  pub(crate) fn context(&self) -> &Context { &self.context }

  fn new<W, P>(wnd: W, p_backend: P, mut context: Context) -> Self
  where
    W: RawWindow + 'static,
    P: PainterBackend + 'static,
  {
    let factor = wnd.scale_factor() as f32;
    context.painter.reset(Some(factor));
    let mut dispatcher = Dispatcher::default();
    let focus_mgr = &mut dispatcher.focus_mgr;
    focus_mgr.update(&mut context);
    if let Some(auto_focusing) = focus_mgr.auto_focus(&context) {
      focus_mgr.focus(auto_focusing, &mut context)
    }

    Self {
      dispatcher,
      raw_window: Box::new(wnd),
      context,
      p_backend: Box::new(p_backend),
    }
  }

  fn resize(&mut self, size: DeviceSize) {
    self.context.mark_layout_from_root();
    self.p_backend.resize(size);
    self.raw_window.request_redraw();
  }

  #[cfg(feature = "wgpu_gl")]
  pub(crate) fn from_event_loop(
    root: BoxedWidget,
    event_loop: &winit::event_loop::EventLoop<()>,
  ) -> Self {
    let native_window = winit::window::WindowBuilder::new()
      .build(event_loop)
      .unwrap();
    let size = native_window.inner_size();
    let ctx = Context::new(root, native_window.scale_factor() as f32);
    let p_backend = futures::executor::block_on(gpu::wgpu_backend_with_wnd(
      &native_window,
      DeviceSize::new(size.width, size.height),
      None,
      None,
      TOLERANCE,
      ctx.shaper.clone(),
    ));

    Self::new(native_window, p_backend, ctx)
  }

  /// Emits a `WindowEvent::RedrawRequested` event in the associated event loop
  /// after all OS events have been processed by the event loop.
  #[inline]
  pub(crate) fn request_redraw(&self) { self.raw_window.request_redraw(); }

  pub fn painter_backend(&self) -> &dyn PainterBackend { &*self.p_backend }
}

pub struct MockBackend;

#[derive(Default)]
pub struct MockRawWindow {
  pub size: Size,
  pub cursor: Option<CursorIcon>,
}

impl PainterBackend for MockBackend {
  fn submit(&mut self, _: Vec<PaintCommand>) {}

  fn resize(&mut self, _: DeviceSize) {}

  fn pixels_image(&self) -> Result<Box<dyn Image>, &str> {
    unreachable!("try to capture image from a mock backend")
  }
}

impl RawWindow for MockRawWindow {
  fn inner_size(&self) -> Size { self.size }
  fn outer_size(&self) -> Size { self.size }
  fn inner_position(&self) -> Point { Point::zero() }
  fn outer_position(&self) -> Point { Point::zero() }
  fn id(&self) -> WindowId { unsafe { WindowId::dummy() } }
  fn set_cursor(&mut self, cursor: CursorIcon) { self.cursor = Some(cursor); }
  fn request_redraw(&self) {}
  fn scale_factor(&self) -> f64 { 1. }
}

impl Window {
  #[cfg(feature = "wgpu_gl")]
  pub fn wgpu_headless(root: BoxedWidget, size: DeviceSize) -> Self {
    let ctx = Context::new(root, 1.);
    let p_backend = futures::executor::block_on(gpu::wgpu_backend_headless(
      size,
      None,
      None,
      TOLERANCE,
      ctx.shaper.clone(),
    ));
    Self::new(
      MockRawWindow {
        size: Size::from_untyped(size.to_f32().to_untyped()),
        ..Default::default()
      },
      p_backend,
      ctx,
    )
  }

  pub fn without_render(root: BoxedWidget, size: Size) -> Self {
    let p_backend = MockBackend;
    Self::new(
      MockRawWindow { size, ..Default::default() },
      p_backend,
      Context::new(root, 1.),
    )
  }
}
