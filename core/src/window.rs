use std::error::Error;

use crate::{
  context::AppContext, events::dispatcher::Dispatcher, prelude::*, widget_tree::WidgetTree,
};

pub trait ShellWindow {
  fn set_theme(self, theme: Theme);

  fn context(&self) -> &AppContext;

  // fn event_loop(&self) -> &EventLoop<()>;

  fn exec(self, wnd_id: Box<dyn WindowId>);

  fn add_window(&mut self, wnd: Window) -> Box<dyn WindowId>;
}

#[derive(Debug, Clone, Default)]
pub struct WindowConfig {
  // Requests the window to be of specific dimensions.
  pub inner_size: Option<Size>,

  /// Sets a minimum dimension size for the window.
  pub min_inner_size: Option<Size>,

  /// Sets a maximum dimension size for the window.
  pub max_inner_size: Option<Size>,

  /// Sets a desired initial position for the window.
  pub position: Option<Point>,

  /// Sets whether the window is resizable or not.
  pub resizable: Option<bool>,

  /// Requests a specific title for the window.
  pub title: Option<String>,

  /// Requests maximized mode.
  pub maximized: Option<bool>,

  /// Sets whether the window will be initially hidden or visible.
  pub visible: Option<bool>,

  /// Sets whether the background of the window should be transparent.
  pub transparent: Option<bool>,

  /// Sets whether the window should have a border, a title bar, etc.
  pub decorations: Option<bool>,
  // /// Sets the window icon.
  // pub window_icon:Option<winit::window::Icon>,
}

pub trait WindowId: std::fmt::Debug {
  fn into_any(self: Box<Self>) -> Box<dyn Any>;
  #[allow(clippy::borrowed_box)] // to make downcast work
  fn equals(&self, other: &Box<dyn WindowId>) -> bool;
  fn box_clone(&self) -> Box<dyn WindowId>;
}

impl Clone for Box<dyn WindowId> {
  fn clone(&self) -> Self { self.box_clone() }
}

impl PartialEq for Box<dyn WindowId> {
  fn eq(&self, other: &Self) -> bool { self.equals(other) }
}

pub trait RawWindow {
  fn inner_size(&self) -> Size;
  fn set_inner_size(&mut self, size: Size);
  fn outer_size(&self) -> Size;
  fn inner_position(&self) -> Point;
  fn outer_position(&self) -> Point;
  fn id(&self) -> Box<dyn WindowId>;

  fn request_redraw(&self);
  /// Modify the native window if cursor modified.
  fn set_cursor(&mut self, cursor: CursorIcon);
  fn scale_factor(&self) -> f64;
  // fn as_any(&self) -> &dyn Any;
  fn into_any(self: Box<Self>) -> Box<dyn Any>;
}

/// A rx scheduler pool that block until all task finished before every frame
/// end.
struct FramePool(FuturesLocalSchedulerPool);

/// Window is the root to represent.
pub struct Window {
  pub raw_window: Box<dyn RawWindow>,
  pub(crate) context: WindowCtx,
  pub(crate) painter: Painter,
  pub(crate) dispatcher: Dispatcher,
  pub(crate) widget_tree: WidgetTree,
  p_backend: Box<dyn PainterBackend>,
  /// A task pool use to process `Future` or `rxRust` task, and will block until
  /// all task finished before current frame end.
  frame_pool: FramePool,
}

impl Window {
  /// processes native events from this native window
  #[inline]
  pub fn processes_native_event(&mut self, event: WindowEvent) {
    match event {
      WindowEvent::Resized(size) => {
        self.on_resize(
          ScaleToPhysic::new(self.raw_window.scale_factor() as f32)
            .transform_size(size)
            .cast(),
        );
      }
      WindowEvent::ScaleFactorChanged { new_inner_size, scale_factor } => {
        self.on_resize(
          ScaleToPhysic::new(self.raw_window.scale_factor() as f32)
            .transform_size(new_inner_size)
            .cast(),
        );
        let factor = scale_factor as f32;
        self.painter.reset(Some(factor));
      }
      event => self.dispatcher.dispatch(event, &mut self.widget_tree),
    };
    if let Some(icon) = self.dispatcher.take_cursor_icon() {
      self.raw_window.set_cursor(icon);
    }
  }

  /// Draw an image what current render tree represent.
  pub fn draw_frame(&mut self) {
    if self.need_draw() {
      self.context.begin_frame();

      loop {
        self.layout();

        // wait all frame task finished.
        self.frame_pool.0.run();

        if !self.widget_tree.is_dirty() {
          break;
        }
      }

      self.dispatcher.refresh_focus(&self.widget_tree);

      self.widget_tree.draw(&mut self.painter);
      let commands = self.painter.finish();
      self.p_backend.submit(commands);

      self.context.end_frame();
    }
  }

  pub fn layout(&mut self) {
    self.widget_tree.layout(self.raw_window.inner_size());
    self.context.layout_ready();
  }

  pub fn need_draw(&self) -> bool {
    self.widget_tree.is_dirty() || self.context.has_actived_animate()
  }

  pub fn new<W, P>(wnd: W, p_backend: P, root: Widget, context: AppContext) -> Self
  where
    W: RawWindow + 'static,
    P: PainterBackend + 'static,
  {
    let typography = context.typography_store.clone();
    let frame_pool = FramePool(FuturesLocalSchedulerPool::new());
    let wnd_ctx = WindowCtx::new(context, frame_pool.0.spawner());
    let widget_tree = WidgetTree::new(root, wnd_ctx.clone());
    let dispatcher = Dispatcher::new(wnd_ctx.focus_mgr.clone());
    let painter = Painter::new(wnd.scale_factor() as f32, typography, wnd.inner_size());
    Self {
      dispatcher,
      raw_window: Box::new(wnd),
      context: wnd_ctx,
      widget_tree,
      p_backend: Box::new(p_backend),
      painter,
      frame_pool,
    }
  }

  fn on_resize(&mut self, size: DeviceSize) {
    self.painter.finish();
    self.widget_tree.mark_dirty(self.widget_tree.root());
    self.widget_tree.store.remove(self.widget_tree.root());
    self.p_backend.resize(size);
    self.painter.resize(self.raw_window.inner_size());
    self.raw_window.request_redraw();
  }

  /// Emits a `WindowEvent::RedrawRequested` event in the associated event loop
  /// after all OS events have been processed by the event loop.
  #[inline]
  pub fn request_redraw(&self) { self.raw_window.request_redraw(); }

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
        stream_writer.write_all(data).unwrap();
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

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test::*;

  #[test]
  fn layout_after_wnd_resize() {
    let w = widget! {
       MockBox { size: INFINITY_SIZE }
    };
    let mut wnd = Window::mock_window(w, Size::new(100., 100.), <_>::default());
    wnd.draw_frame();
    assert_layout_result(&wnd, &[0], &ExpectRect::from_size(Size::new(100., 100.)));

    let new_size = DeviceSize::new(200, 200);
    wnd.raw_window.set_inner_size(new_size.to_f32().cast_unit());
    wnd.on_resize(new_size);
    wnd.draw_frame();
    assert_layout_result(
      &wnd,
      &[0],
      &ExpectRect::from_size(new_size.to_f32().cast_unit()),
    );
  }
}
