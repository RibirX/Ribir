use std::{
  rc::Rc,
  sync::atomic::{AtomicU64, Ordering},
};

#[cfg(test)]
#[cfg(target_family = "wasm")]
pub use wasm_bindgen_test::wasm_bindgen_test;

#[cfg(test)]
#[cfg(target_family = "wasm")]
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

pub use crate::timer::Timer;
use crate::{
  prelude::*,
  window::{ShellWindow, WindowId},
};

pub struct Frame {
  pub commands: Vec<PaintCommand>,
  pub viewport: Rect,
  pub surface: Color,
}

pub fn split_value<T: 'static>(v: T) -> (Watcher<Reader<T>>, impl StateWriter<Value = T>) {
  let src = Stateful::new(v);
  (src.clone_watcher(), src.clone_writer())
}

#[derive(Clone)]
pub struct TestWindow(pub Rc<Window>);

#[macro_export]
macro_rules! reset_test_env {
  () => {
    let _ = $crate::prelude::NEW_TIMER_FN.set($crate::timer::Timer::new_timer_future);
    let _guard = unsafe { $crate::prelude::AppCtx::new_lock_scope() };
  };
}

impl TestWindow {
  /// Create a 1024x1024 window for test
  pub fn new(root: impl WidgetBuilder) -> Self { Self::new_wnd(root, None) }

  pub fn new_with_size(root: impl WidgetBuilder, size: Size) -> Self {
    Self::new_wnd(root, Some(size))
  }

  fn new_wnd(root: impl WidgetBuilder, size: Option<Size>) -> Self {
    let _ = NEW_TIMER_FN.set(Timer::new_timer_future);
    let wnd = AppCtx::new_window(Box::new(TestShellWindow::new(size)), root);
    wnd.run_frame_tasks();
    Self(wnd)
  }

  /// Ues a index path to access widget tree and return the layout info,
  /// [0, 1] means use the second child of the root.
  /// [0, 1, 2] the first node at the root level (must be 0), then down to its
  /// second child, then down to third child.
  pub fn layout_info_by_path(&self, path: &[usize]) -> Option<LayoutInfo> {
    let tree = self.0.widget_tree.borrow();
    let mut node = tree.root();
    for (level, idx) in path[..].iter().enumerate() {
      node = node
        .children(&tree.arena)
        .nth(*idx)
        .unwrap_or_else(|| {
          panic!("node no exist: {:?}", &path[0..level]);
        });
    }
    tree.store.layout_info(node).cloned()
  }

  pub fn take_last_frame(&mut self) -> Option<Frame> {
    self
      .shell_wnd()
      .borrow_mut()
      .as_any_mut()
      .downcast_mut::<TestShellWindow>()
      .unwrap()
      .last_frame
      .take()
  }

  pub fn content_count(&self) -> usize {
    let widget_tree = self.0.widget_tree.borrow();
    let root = widget_tree.root();
    let content = root.first_child(&widget_tree.arena).unwrap();
    widget_tree.count(content)
  }

  #[track_caller]
  pub fn draw_frame(&mut self) {
    // Test window not have a eventloop, manually wake-up every frame.
    Timer::wake_timeout_futures();
    self.run_frame_tasks();
    self.0.draw_frame();
  }
}

impl std::ops::Deref for TestWindow {
  type Target = Window;

  fn deref(&self) -> &Self::Target { &self.0 }
}

pub struct TestShellWindow {
  pub size: Size,
  pub cursor: CursorIcon,
  pub id: WindowId,
  pub surface_color: Color,
  pub last_frame: Option<Frame>,
}

impl ShellWindow for TestShellWindow {
  fn inner_size(&self) -> Size { self.size }

  fn outer_size(&self) -> Size { self.size }

  fn request_resize(&mut self, size: Size) { self.on_resize(size); }

  fn on_resize(&mut self, size: Size) {
    self.size = size;
    self.last_frame = None;
  }

  fn set_min_size(&mut self, _: Size) {}

  fn set_cursor(&mut self, cursor: CursorIcon) { self.cursor = cursor; }

  fn cursor(&self) -> CursorIcon { self.cursor }

  fn set_title(&mut self, _: &str) {}

  fn set_icon(&mut self, _: &PixelImage) {}

  fn set_ime_cursor_area(&mut self, _: &Rect) {}

  fn set_visible(&mut self, _: bool) {}

  fn is_visible(&self) -> Option<bool> { Some(true) }

  fn set_resizable(&mut self, _: bool) {}

  fn is_resizable(&self) -> bool { true }

  fn focus_window(&mut self) {}

  fn set_decorations(&mut self, _: bool) {}

  fn is_minimized(&self) -> bool { false }

  fn set_minimized(&mut self, _: bool) {}

  fn set_ime_allowed(&mut self, _: bool) {}

  fn as_any(&self) -> &dyn Any { self }

  fn as_any_mut(&mut self) -> &mut dyn Any { self }

  fn begin_frame(&mut self, surface: Color) { self.surface_color = surface; }

  fn draw_commands(&mut self, viewport: Rect, commands: &[PaintCommand]) {
    self.last_frame =
      Some(Frame { commands: commands.to_owned(), viewport, surface: self.surface_color });
  }

  fn end_frame(&mut self) {}

  fn id(&self) -> WindowId { self.id }

  fn device_pixel_ratio(&self) -> f32 { 1. }
}

impl TestShellWindow {
  fn new(size: Option<Size>) -> Self {
    static ID: AtomicU64 = AtomicU64::new(0);
    let size = size.unwrap_or_else(|| Size::new(1024., 1024.));
    TestShellWindow {
      size,
      cursor: CursorIcon::Default,
      id: ID.fetch_add(1, Ordering::Relaxed).into(),
      last_frame: None,
      surface_color: Color::WHITE,
    }
  }
}

#[derive(Declare, Query, MultiChild)]
pub struct MockStack {
  child_pos: Vec<Point>,
}

impl Render for MockStack {
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    let mut layouter = ctx.first_child_layouter();
    let mut size = ZERO_SIZE;
    let mut i = 0;
    while let Some(mut l) = layouter {
      let mut child_size = l.perform_widget_layout(clamp);
      if let Some(offset) = self.child_pos.get(i) {
        l.update_position(*offset);
        child_size = Size::new(offset.x + child_size.width, offset.y + child_size.height);
      } else {
        l.update_position(Point::zero());
      }
      size = size.max(child_size);
      layouter = l.into_next_sibling();

      i += 1;
    }

    size
  }
  fn paint(&self, _: &mut PaintingCtx) {}
}

#[derive(Declare, Query, MultiChild, Default)]
pub struct MockMulti;

#[derive(Declare, Query, Clone, SingleChild)]
pub struct MockBox {
  pub size: Size,
}

impl Render for MockMulti {
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    let mut layouter = ctx.first_child_layouter();
    let mut size = ZERO_SIZE;
    while let Some(mut l) = layouter {
      let child_size = l.perform_widget_layout(clamp);
      l.update_position(Point::new(size.width, 0.));
      size.width += child_size.width;
      size.height = size.height.max(child_size.height);
      layouter = l.into_next_sibling();
    }

    size
  }

  fn paint(&self, _: &mut PaintingCtx) {}
}

impl Render for MockBox {
  fn perform_layout(&self, mut clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    clamp.max = clamp.max.min(self.size);
    ctx.perform_single_child_layout(clamp);

    self.size
  }
  #[inline]
  fn only_sized_by_parent(&self) -> bool { true }

  #[inline]
  fn paint(&self, _: &mut PaintingCtx) {}
}
