use std::sync::atomic::{AtomicU64, Ordering};

#[cfg(test)]
#[cfg(target_family = "wasm")]
pub use wasm_bindgen_test::wasm_bindgen_test;

#[cfg(test)]
#[cfg(target_family = "wasm")]
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

pub use crate::timer::Timer;
use crate::{
  prelude::*,
  window::{ShellWindow, WindowFlags, WindowId},
};

pub struct Frame {
  pub commands: Vec<PaintCommand>,
  pub viewport: Rect,
  pub surface: Color,
}

pub fn split_value<T: 'static>(v: T) -> (Watcher<Reader<T>>, Stateful<T>) {
  let src = Stateful::new(v);
  (src.clone_watcher(), src.clone_writer())
}

/// The Window assists in writing unit tests; animations are disabled by
/// default.
#[derive(Clone)]
pub struct TestWindow(pub Sc<Window>);

#[macro_export]
macro_rules! reset_test_env {
  () => {
    let _ = $crate::prelude::NEW_TIMER_FN.set($crate::timer::Timer::new_timer_future);
    let _guard = $crate::prelude::AppCtx::new_lock_scope();
  };
}

impl TestWindow {
  /// Create a 1024x1024 window for test
  pub fn new(root: impl Into<GenWidget>) -> Self { Self::new_wnd(root, None) }

  pub fn new_with_size(root: impl Into<GenWidget>, size: Size) -> Self {
    Self::new_wnd(root, Some(size))
  }

  #[track_caller]
  pub fn assert_root_size(&self, size: Size) {
    let info = self.layout_info_by_path(&[0]).unwrap();
    assert_eq!(info.size.unwrap(), size);
  }

  fn new_wnd(root: impl Into<GenWidget>, size: Option<Size>) -> Self {
    let _ = NEW_TIMER_FN.set(Timer::new_timer_future);
    AppCtx::run_until_stalled();

    let wnd = AppCtx::new_window(Box::new(TestShellWindow::new(size)), root.into());
    let mut flags = wnd.flags();
    flags.remove(WindowFlags::ANIMATIONS);
    wnd.set_flags(flags);
    wnd.run_frame_tasks();
    Self(wnd)
  }

  /// Ues a index path to access widget tree and return the layout info,
  /// [0, 1] means use the second child of the root.
  /// [0, 1, 2] the first node at the root level (must be 0), then down to its
  /// second child, then down to third child.
  pub fn layout_info_by_path(&self, path: &[usize]) -> Option<LayoutInfo> {
    let tree = self.0.tree();
    let mut node = tree.root();
    for (level, idx) in path[..].iter().enumerate() {
      node = node.children(tree).nth(*idx).unwrap_or_else(|| {
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
    let tree = self.0.tree();
    let root = tree.root();
    let content = root.first_child(tree).unwrap();
    tree.count(content)
  }

  #[track_caller]
  pub fn draw_frame(&mut self) {
    // Test window not have a eventloop, manually wake-up every frame.
    Timer::wake_timeout_futures();
    self.run_frame_tasks();

    AppCtx::frame_ticks().clone().next(Instant::now());
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

#[derive(Declare, MultiChild)]
pub struct MockStack {}

impl Render for MockStack {
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    let mut size = ZERO_SIZE;
    let (ctx, children) = ctx.split_children();
    for c in children {
      let child_size = ctx.perform_child_layout(c, clamp);
      size = size.max(child_size);
    }

    size
  }
  fn paint(&self, _: &mut PaintingCtx) {}
}

#[derive(Declare, MultiChild, Default)]
pub struct MockMulti;

#[derive(Declare, Clone, SingleChild)]
pub struct MockBox {
  pub size: Size,
}

impl Render for MockMulti {
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    let mut size = ZERO_SIZE;
    let (ctx, children) = ctx.split_children();
    for c in children {
      let child_size = ctx.perform_child_layout(c, clamp);
      ctx.update_position(c, Point::new(size.width, 0.));
      size.width += child_size.width;
      size.height = size.height.max(child_size.height);
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

/// The layout case describes the expected position and size of a widget node
/// based on its index path.
///
/// In terms of the index path:
///   - [0, 1] indicates the second child of the root, where the root level is
///     denoted as 0.
///   - [0, 1, 2] signifies the first node at the root level (which is 0),
///     followed by its second child, and then the third child below that.
pub struct LayoutCase {
  path: &'static [usize],
  x: Option<f32>,
  y: Option<f32>,
  width: Option<f32>,
  height: Option<f32>,
}

impl LayoutCase {
  #[track_caller]
  pub fn expect_x(wnd: &TestWindow, path: &'static [usize], x: f32) {
    LayoutCase::new(path).with_x(x).check(wnd);
  }

  #[track_caller]
  pub fn expect_y(wnd: &TestWindow, path: &'static [usize], y: f32) {
    LayoutCase::new(path).with_y(y).check(wnd);
  }

  #[track_caller]
  pub fn expect_size(wnd: &TestWindow, path: &'static [usize], size: Size) {
    LayoutCase::new(path).with_size(size).check(wnd);
  }

  #[track_caller]
  pub fn expect_pos(wnd: &TestWindow, path: &'static [usize], pos: Point) {
    LayoutCase::new(path).with_pos(pos).check(wnd);
  }

  #[track_caller]
  pub fn expect_rect(wnd: &TestWindow, path: &'static [usize], rect: Rect) {
    LayoutCase::new(path)
      .with_pos(rect.origin)
      .with_size(rect.size)
      .check(wnd);
  }

  pub fn new(path: &'static [usize]) -> Self { LayoutCase { path, ..<_>::default() } }

  pub fn with_pos(mut self, pos: Point) -> Self {
    self.x = Some(pos.x);
    self.y = Some(pos.y);
    self
  }
  pub fn with_x(mut self, x: f32) -> Self {
    self.x = Some(x);
    self
  }

  pub fn with_y(mut self, y: f32) -> Self {
    self.y = Some(y);
    self
  }

  pub fn with_width(mut self, width: f32) -> Self {
    self.width = Some(width);
    self
  }

  pub fn with_height(mut self, height: f32) -> Self {
    self.height = Some(height);
    self
  }

  pub fn with_size(mut self, size: Size) -> Self {
    self.width = Some(size.width);
    self.height = Some(size.height);
    self
  }

  pub fn with_rect(self, rect: Rect) -> Self { self.with_pos(rect.origin).with_size(rect.size) }

  #[track_caller]
  pub fn check(&self, wnd: &TestWindow) {
    let Self { path, x, y, width, height } = self;

    let info = wnd.layout_info_by_path(path).unwrap();
    if let Some(x) = x {
      assert_eq!(*x, info.pos.x, "unexpected x");
    }
    if let Some(y) = y {
      assert_eq!(*y, info.pos.y, "unexpected y");
    }
    if let Some(w) = width {
      assert_eq!(*w, info.size.unwrap().width, "unexpected width");
    }
    if let Some(h) = height {
      assert_eq!(*h, info.size.unwrap().height, "unexpected height");
    }
  }
}

pub struct WidgetTester {
  pub widget: GenWidget,
  pub wnd_size: Option<Size>,
  pub on_initd: Option<InitdFn>,
  pub comparison: Option<f64>,
}

type InitdFn = Box<dyn Fn(&mut TestWindow)>;

impl WidgetTester {
  pub fn new(widget: impl Into<GenWidget>) -> Self {
    Self { wnd_size: None, widget: widget.into(), on_initd: None, comparison: None }
  }

  /// This callback runs after creating the window and drawing the first frame.
  pub fn on_initd(mut self, on_initd: impl Fn(&mut TestWindow) + 'static) -> Self {
    self.on_initd = Some(Box::new(on_initd));
    self
  }

  pub fn with_wnd_size(mut self, size: Size) -> Self {
    self.wnd_size = Some(size);
    self
  }

  pub fn with_comparison(mut self, comparison: f64) -> Self {
    self.comparison = Some(comparison);
    self
  }

  pub fn create_wnd(&self) -> TestWindow {
    let wnd_size = self.wnd_size.unwrap_or(Size::new(1024., 1024.));
    let mut wnd = TestWindow::new_with_size(self.widget.clone(), wnd_size);
    wnd.draw_frame();
    if let Some(initd) = self.on_initd.as_ref() {
      initd(&mut wnd);
      wnd.draw_frame();
    }
    wnd
  }

  #[track_caller]
  pub fn layout_check(&self, cases: &[LayoutCase]) {
    let wnd = self.create_wnd();
    cases.iter().for_each(|c| c.check(&wnd));
  }
}

impl Default for LayoutCase {
  fn default() -> Self { Self { path: &[0], x: None, y: None, width: None, height: None } }
}
