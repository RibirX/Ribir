use crate::{
  context::AppContext, events::dispatcher::Dispatcher, prelude::*, timer::new_timer,
  widget_tree::WidgetTree,
};

use winit::event::WindowEvent;
pub use winit::window::CursorIcon;

/// A rx scheduler pool that block until all task finished before every frame
/// end.
struct FramePool(FuturesLocalSchedulerPool);
/// Window is the root to represent.
pub struct Window {
  pub(crate) context: WindowCtx,
  pub(crate) painter: Painter,
  pub(crate) dispatcher: Dispatcher,
  pub(crate) widget_tree: WidgetTree,
  /// A task pool use to process `Future` or `rxRust` task, and will block until
  /// all task finished before current frame end.
  frame_pool: FramePool,
  shell_wnd: Box<dyn ShellWindow>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Hash)]
pub struct WindowId(u64);

pub trait ShellWindow {
  fn id(&self) -> WindowId;
  fn inner_size(&self) -> Size;
  fn outer_size(&self) -> Size;
  fn device_scale(&self) -> f32;
  fn set_size(&mut self, size: Size);
  fn set_cursor(&mut self, cursor: CursorIcon);
  fn set_title(&mut self, str: &str);

  fn as_any(&self) -> &dyn Any;

  fn begin_frame(&mut self);
  fn draw_commands(&mut self, viewport: DeviceRect, commands: Vec<PaintCommand>);
  fn end_frame(&mut self);
}

impl Window {
  /// processes native events from this native window
  #[inline]
  pub fn processes_native_event(&mut self, event: WindowEvent) {
    match event {
      WindowEvent::Resized(size) => {
        let size = size.to_logical(self.painter.device_scale() as f64);
        self.resize(Size::new(size.width, size.height));
      }
      WindowEvent::ScaleFactorChanged { new_inner_size, scale_factor } => {
        let size = new_inner_size.to_logical(scale_factor);
        self
          .set_device_factor(scale_factor as f32)
          .resize(Size::new(size.width, size.height));
        self.resize(Size::new(size.width, size.height));
      }
      event => self.dispatcher.dispatch(
        event,
        &mut self.widget_tree,
        self.painter.device_scale() as f64,
      ),
    };
    if let Some(icon) = self.dispatcher.take_cursor_icon() {
      self.shell_wnd.set_cursor(icon);
    }
  }

  /// Draw an image what current render tree represent.
  pub fn draw_frame(&mut self) {
    if !self.need_draw() {
      return;
    }

    self.context.begin_frame();
    self.shell_wnd.begin_frame();

    loop {
      self.layout();

      // wait all frame task finished.
      self.run_futures();

      if !self.widget_tree.is_dirty() {
        break;
      }
    }

    self.dispatcher.refresh_focus(&self.widget_tree);

    self.widget_tree.draw(&mut self.painter);
    let scale = self.shell_wnd.device_scale();
    let wnd_size = (self.shell_wnd.inner_size() * scale).to_i32().cast_unit();

    self
      .shell_wnd
      .draw_commands(DeviceRect::from_size(wnd_size), self.painter.finish());

    self.shell_wnd.end_frame();
    self.context.end_frame();
  }

  pub fn run_futures(&mut self) { self.frame_pool.0.run_until_stalled(); }

  pub fn layout(&mut self) {
    self.widget_tree.layout(self.shell_wnd.inner_size());
    self.context.layout_ready();
  }

  pub fn need_draw(&self) -> bool {
    self.widget_tree.is_dirty() || self.context.has_actived_animate()
  }

  pub fn new(root: Widget, shell_wnd: Box<dyn ShellWindow>, context: AppContext) -> Self {
    let typography = context.typography_store.clone();
    let frame_pool = FramePool(FuturesLocalSchedulerPool::new());
    let wnd_ctx = WindowCtx::new(context, frame_pool.0.spawner());
    let widget_tree = WidgetTree::new(root, wnd_ctx.clone());
    let dispatcher = Dispatcher::new(wnd_ctx.focus_mgr.clone());
    let size = shell_wnd.inner_size();
    let mut painter = Painter::new(shell_wnd.device_scale(), Rect::from_size(size), typography);
    painter.set_bounds(Rect::from_size(size));
    Self {
      dispatcher,
      context: wnd_ctx,
      widget_tree,
      painter,
      frame_pool,
      shell_wnd,
    }
  }

  #[inline]
  pub fn id(&self) -> WindowId { self.shell_wnd.id() }

  pub fn set_title(&mut self, title: &str) -> &mut Self {
    self.shell_wnd.set_title(title);
    self
  }

  pub fn set_device_factor(&mut self, device_scale: f32) -> &mut Self {
    self.painter.set_device_scale(device_scale);
    self
  }

  fn resize(&mut self, size: Size) {
    if self.shell_wnd.inner_size() != size {
      self.shell_wnd.set_size(size);
    }
    self.painter.finish();
    self.widget_tree.mark_dirty(self.widget_tree.root());
    self.widget_tree.store.remove(self.widget_tree.root());
    self.painter.set_bounds(Rect::from_size(size));
  }

  pub fn shell_wnd(&self) -> &dyn ShellWindow { &*self.shell_wnd }
}

impl From<u64> for WindowId {
  #[inline]
  fn from(value: u64) -> Self { WindowId(value) }
}

impl From<WindowId> for u64 {
  #[inline]
  fn from(value: WindowId) -> Self { value.0 }
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
    let mut wnd = mock_window(w, Size::new(100., 100.), <_>::default());
    wnd.draw_frame();
    assert_layout_result(&wnd, &[0], &ExpectRect::from_size(Size::new(100., 100.)));

    let new_size = Size::new(200., 200.);
    wnd.resize(new_size);
    wnd.draw_frame();
    assert_layout_result(&wnd, &[0], &ExpectRect::from_size(new_size));
  }
}
