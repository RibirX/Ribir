use std::{cell::RefCell, error::Error, rc::Rc};

use crate::{
  context::Context,
  events::dispatcher::Dispatcher,
  prelude::{widget_tree::WidgetTree, *},
};

pub use winit::window::CursorIcon;
use winit::{event::WindowEvent, window::WindowId};

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
  pub(crate) context: Rc<RefCell<Context>>,
  pub(crate) painter: Painter,
  pub(crate) dispatcher: Dispatcher,
  pub(crate) widget_tree: WidgetTree,
  p_backend: Box<dyn PainterBackend>,
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
        self.painter.reset(Some(factor));
      }
      event => {
        self
          .dispatcher
          .dispatch(event, &mut self.widget_tree, self.raw_window.scale_factor())
      }
    };
    if let Some(icon) = self.dispatcher.take_cursor_icon() {
      self.raw_window.set_cursor(icon);
    }
  }

  /// Draw an image what current render tree represent.
  pub(crate) fn draw_frame(&mut self) {
    if self.need_draw() {
      let Self {
        raw_window,
        context,
        dispatcher,
        widget_tree,
        p_backend,
        painter,
      } = self;

      context.borrow_mut().begin_frame();

      widget_tree.tree_repair();
      widget_tree.layout(raw_window.inner_size());
      if context.borrow().expr_widgets_dirty() {
        dispatcher.refresh_focus(widget_tree);
      }
      widget_tree.draw(painter);
      let commands = painter.finish();
      p_backend.submit(commands);

      context.borrow_mut().end_frame();
    }
  }

  pub(crate) fn need_draw(&self) -> bool {
    self.widget_tree.any_state_modified() || self.context.borrow().expr_widgets_dirty()
  }

  pub(crate) fn context(&self) -> &Rc<RefCell<Context>> { &self.context }

  fn new<W, P>(wnd: W, p_backend: P, root: Widget, context: Rc<RefCell<Context>>) -> Self
  where
    W: RawWindow + 'static,
    P: PainterBackend + 'static,
  {
    let mut widget_tree = WidgetTree::new(root, Rc::downgrade(&context));
    let mut dispatcher = Dispatcher::default();
    dispatcher.refresh_focus(&mut widget_tree);
    if let Some(auto_focusing) = dispatcher.auto_focus(&widget_tree) {
      dispatcher.focus(auto_focusing, &mut widget_tree)
    }
    let painter = Painter::new(
      wnd.scale_factor() as f32,
      context.borrow().typography_store.clone(),
    );
    Self {
      dispatcher,
      raw_window: Box::new(wnd),
      context,
      widget_tree,
      p_backend: Box::new(p_backend),
      painter,
    }
  }

  fn resize(&mut self, size: DeviceSize) {
    self.widget_tree.mark_dirty(self.widget_tree.root());
    self.p_backend.resize(size);
    self.raw_window.request_redraw();
  }

  #[cfg(feature = "wgpu_gl")]
  pub(crate) fn from_event_loop(
    root: Widget,
    event_loop: &winit::event_loop::EventLoop<()>,
  ) -> Self {
    let native_window = winit::window::WindowBuilder::new()
      .with_inner_size(winit::dpi::LogicalSize::new(512., 512.))
      .build(event_loop)
      .unwrap();
    let ctx = Rc::new(RefCell::new(Context::default()));
    let size = native_window.inner_size();
    let p_backend = futures::executor::block_on(gpu::wgpu_backend_with_wnd(
      &native_window,
      DeviceSize::new(size.width, size.height),
      None,
      None,
      ctx.borrow().shaper.clone(),
    ));
    Self::new(native_window, p_backend, root, ctx)
  }

  /// Emits a `WindowEvent::RedrawRequested` event in the associated event loop
  /// after all OS events have been processed by the event loop.
  #[inline]
  pub(crate) fn request_redraw(&self) { self.raw_window.request_redraw(); }

  pub fn capture_image<F>(&mut self, image_data_callback: F) -> Result<(), Box<dyn Error>>
  where
    F: for<'r> FnOnce(DeviceSize, Box<dyn Iterator<Item = &[u8]> + 'r>),
  {
    self.widget_tree.draw(&mut self.painter);
    let commands = self.painter.finish();
    self
      .p_backend
      .commands_to_image(commands, Box::new(image_data_callback))
  }

  #[cfg(feature = "png")]
  pub fn write_as_png<P>(&mut self, path: P) -> Result<(), Box<dyn Error>>
  where
    P: std::convert::AsRef<std::path::Path>,
  {
    use std::io::Write;
    let writer = std::fs::File::create(path.as_ref()).map_err(|e| e.to_string())?;
    self.capture_image(move |size, rows| {
      let mut png_encoder = png::Encoder::new(writer, size.width, size.height);
      png_encoder.set_depth(png::BitDepth::Eight);
      png_encoder.set_color(png::ColorType::Rgba);

      let mut writer = png_encoder.write_header().unwrap();
      let mut stream_writer = writer
        .stream_writer_with_size(size.width as usize * 4)
        .unwrap();

      rows.for_each(|data| {
        stream_writer.write(data).unwrap();
      });
      stream_writer.finish().unwrap();
    })
  }

  #[cfg(feature = "png")]
  pub fn same_as_png<P>(&mut self, path: P) -> bool
  where
    P: std::convert::AsRef<std::path::Path>,
  {
    let file = std::fs::File::open(path.as_ref());
    file
      .and_then(|f| {
        let decoder = png::Decoder::new(f);
        let mut reader = decoder.read_info()?;

        let mut same = false;
        self
          .capture_image(|size, rows| {
            // Allocate the output buffer.
            let mut buf = vec![0; reader.output_buffer_size()];
            let info = reader.next_frame(&mut buf).unwrap();
            if info.width == size.width && info.height == size.height {
              same = rows.enumerate().all(|(i, bytes)| {
                let offset = i * info.line_size;
                &buf[offset..offset + info.line_size] == bytes
              });
            }
          })
          .map_or(Ok(false), |_| Ok(same))
      })
      .unwrap_or(false)
  }
}

pub struct MockBackend;

#[derive(Default)]
pub struct MockRawWindow {
  pub size: Size,
  pub cursor: Option<CursorIcon>,
}

impl PainterBackend for MockBackend {
  fn submit<'a>(&mut self, _: Vec<PaintCommand>) {}

  fn resize(&mut self, _: DeviceSize) {}

  fn commands_to_image(
    &mut self,
    _: Vec<PaintCommand>,
    _: CaptureCallback,
  ) -> Result<(), Box<dyn Error>> {
    Ok(())
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
  pub fn wgpu_headless(root: Widget, size: DeviceSize) -> Self {
    let ctx = Rc::new(RefCell::new(Context::default()));
    let p_backend = futures::executor::block_on(gpu::wgpu_backend_headless(
      size,
      None,
      None,
      ctx.borrow().shaper.clone(),
    ));
    Self::new(
      MockRawWindow {
        size: Size::from_untyped(size.to_f32().to_untyped()),
        ..Default::default()
      },
      p_backend,
      root,
      ctx,
    )
  }

  pub fn without_render(root: Widget, size: Size) -> Self {
    Self::new(
      MockRawWindow { size, ..Default::default() },
      MockBackend,
      root,
      <_>::default(),
    )
  }
}
