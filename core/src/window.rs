use crate::{context::AppCtx, events::dispatcher::Dispatcher, prelude::*, widget_tree::WidgetTree};

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
  fn set_ime_pos(&mut self, pos: Point);
  fn set_size(&mut self, size: Size);
  fn set_min_size(&mut self, size: Size);
  fn set_cursor(&mut self, cursor: CursorIcon);
  fn set_title(&mut self, str: &str);
  fn set_icon(&mut self, icon: &PixelImage);
  fn as_any(&self) -> &dyn Any;
  fn as_any_mut(&mut self) -> &mut dyn Any;
  /// The device pixel ratio of Window interface returns the ratio of the
  /// resolution in physical pixels to the logic pixels for the current display
  /// device.
  fn device_pixel_ratio(&self) -> f32;
  fn begin_frame(&mut self);
  fn draw_commands(&mut self, viewport: Rect, commands: Vec<PaintCommand>, surface: Color);
  fn end_frame(&mut self);
}

impl Window {
  #[deprecated(note = "The core window should not depends on shell window event.")]
  #[inline]
  /// processes native events from this native window
  pub fn processes_native_event(&mut self, event: WindowEvent) {
    let ratio = self.device_pixel_ratio() as f64;
    self
      .dispatcher
      .dispatch(event, &mut self.widget_tree, ratio);
    if let Some(icon) = self.dispatcher.take_cursor_icon() {
      self.shell_wnd.set_cursor(icon);
    }
  }

  pub fn wnd_ctx(&self) -> &WindowCtx { &self.context }

  /// Draw an image what current render tree represent.
  #[track_caller]
  pub fn draw_frame(&mut self) {
    if !self.need_draw() {
      return;
    }

    self.context.begin_frame();
    self.shell_wnd.begin_frame();

    loop {
      self.layout();

      // wait all frame task finished.
      self.frame_pool.0.run();

      if !self.widget_tree.is_dirty() {
        break;
      }
    }

    self.dispatcher.refresh_focus(&self.widget_tree);
    self.shell_wnd.set_ime_pos(*self.context.ime_pos.borrow());

    self.widget_tree.draw(&mut self.painter);

    let surface = match AppCtx::app_theme() {
      Theme::Full(theme) => theme.palette.surface(),
      Theme::Inherit(_) => unreachable!(),
    };

    self.shell_wnd.draw_commands(
      Rect::from_size(self.shell_wnd.inner_size()),
      self.painter.finish(),
      surface,
    );

    self.shell_wnd.end_frame();
    self.context.end_frame();
    AppCtx::end_frame();
  }

  pub fn layout(&mut self) {
    self.widget_tree.layout(self.shell_wnd.inner_size());
    self.context.layout_ready();
  }

  pub fn need_draw(&self) -> bool {
    self.widget_tree.is_dirty() || self.context.has_actived_animate()
  }

  pub fn new(root: Widget, shell_wnd: Box<dyn ShellWindow>) -> Self {
    let frame_pool = FramePool(FuturesLocalSchedulerPool::new());
    let wnd_ctx = WindowCtx::new(frame_pool.0.spawner());
    let widget_tree = WidgetTree::new(root, wnd_ctx.clone());
    let dispatcher = Dispatcher::new(wnd_ctx.focus_mgr.clone());
    let size = shell_wnd.inner_size();
    let mut painter = Painter::new(Rect::from_size(size));
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

  /// The device pixel ratio of Window interface returns the ratio of the
  /// resolution in physical pixels to the logic pixels for the current display
  /// device.
  pub fn device_pixel_ratio(&self) -> f32 { self.shell_wnd.device_pixel_ratio() }

  pub fn set_title(&mut self, title: &str) -> &mut Self {
    self.shell_wnd.set_title(title);
    self
  }

  pub fn set_icon(&mut self, icon: &PixelImage) -> &mut Self {
    self.shell_wnd.set_icon(icon);
    self
  }

  pub fn set_size(&mut self, size: Size) { self.shell_wnd.set_size(size); }

  pub fn set_min_size(&mut self, size: Size) -> &mut Self {
    self.shell_wnd.set_min_size(size);
    self
  }

  pub fn on_wnd_resize_event(&mut self, size: Size) {
    self.widget_tree.mark_dirty(self.widget_tree.root());
    self.widget_tree.store.remove(self.widget_tree.root());
    self.painter.set_bounds(Rect::from_size(size));
    self.painter.reset();
  }

  pub fn shell_wnd(&self) -> &dyn ShellWindow { &*self.shell_wnd }

  pub fn shell_wnd_mut(&mut self) -> &mut dyn ShellWindow { &mut *self.shell_wnd }
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
  use crate::test_helper::*;
  use ribir_dev_helper::assert_layout_result_by_path;

  #[test]
  fn layout_after_wnd_resize() {
    let _guard = unsafe { AppCtx::new_lock_scope() };

    let w = widget! {
       MockBox { size: INFINITY_SIZE }
    };
    let size = Size::new(100., 100.);
    let mut wnd = TestWindow::new_with_size(w, size);
    wnd.draw_frame();
    assert_layout_result_by_path!(wnd, { path = [0], size == size, });

    let new_size = Size::new(200., 200.);
    wnd.set_size(new_size);
    // not have a shell window, trigger the resize manually.
    wnd.on_wnd_resize_event(new_size);
    wnd.draw_frame();
    assert_layout_result_by_path!(wnd, { path = [0], size == new_size, });
  }
}
