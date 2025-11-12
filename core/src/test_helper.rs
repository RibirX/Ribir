use std::sync::atomic::{AtomicU64, Ordering};

#[cfg(test)]
#[cfg(target_arch = "wasm32")]
pub use wasm_bindgen_test::wasm_bindgen_test;

#[cfg(test)]
#[cfg(target_arch = "wasm32")]
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

use crate::{
  prelude::*,
  window::{BoxShellWindow, Shell, ShellWindow, WindowFlags, WindowId},
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
    let _guard = $crate::prelude::AppCtx::new_lock_scope();
  };
}

pub struct TestShell {}

impl Shell for TestShell {
  fn exit(&self) {}

  fn new_shell_window(
    &self, attr: window::WindowAttributes,
  ) -> scheduler::BoxFuture<'static, BoxShellWindow> {
    Box::pin(async move {
      Box::new(TestShellWindow::new(attr.0.inner_size.map_or_else(
        || Size::new(1024., 1024.),
        |s| {
          let s = s.to_logical(1.);
          Size::new(s.width, s.height)
        },
      ))) as BoxShellWindow
    })
  }

  fn run_in_shell(&self, f: scheduler::BoxFuture<'static, ()>) { AppCtx::spawn_local(f); }
}

impl TestWindow {
  /// Create a 1024x1024 window for test
  pub fn from_widget<K: ?Sized>(root: impl RInto<GenWidget, K>) -> Self {
    Self::new(root, Size::new(1024., 1024.), WindowFlags::empty())
  }

  pub fn new_with_size<K: ?Sized>(root: impl RInto<GenWidget, K>, size: Size) -> Self {
    Self::new(root, size, WindowFlags::empty())
  }

  #[track_caller]
  pub fn assert_root_size(&self, size: Size) {
    let info = self.layout_info_by_path(&[0]).unwrap();
    assert_eq!(info.size.unwrap(), size);
  }

  pub fn new<K: ?Sized>(root: impl RInto<GenWidget, K>, size: Size, flags: WindowFlags) -> Self {
    let wnd = Window::new(Box::new(TestShellWindow::new(size)), flags);

    AppCtx::insert_window(wnd.clone());
    wnd.init(root.r_into());

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
  pub fn draw_frame(&self) {
    // Test window not have a eventloop, manually wake-up every frame.
    #[cfg(not(target_arch = "wasm32"))]
    AppCtx::new_test_frame(self);
  }

  pub fn fmt_tree(&self) -> String { self.tree().display_tree(self.tree().root()) }

  pub fn request_resize(&self, size: Size) {
    let root = self.0.tree().root();
    self
      .0
      .tree()
      .dirty_marker()
      .mark(root, DirtyPhase::Layout);
    self.0.request_resize(size);
  }
}

impl std::ops::Deref for TestWindow {
  type Target = Window;

  fn deref(&self) -> &Self::Target { &self.0 }
}

pub struct TestShellWindow {
  pub cursor: CursorIcon,
  pub id: WindowId,
  pub surface_color: Color,
  pub last_frame: Option<Frame>,
  pub size: Size,
}

impl ShellWindow for TestShellWindow {
  fn inner_size(&self) -> Size { self.size }

  fn request_resize(&mut self, size: Size) { self.on_resize(size); }

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

  // fn set_decorations(&mut self, _: bool) {}

  fn is_minimized(&self) -> bool { false }

  fn set_minimized(&mut self, _: bool) {}

  fn set_window_level(&mut self, _: bool) {}

  fn set_ime_allowed(&mut self, _: bool) {}

  fn as_any(&self) -> &dyn Any { self }

  fn as_any_mut(&mut self) -> &mut dyn Any { self }

  fn draw_commands(
    &mut self, _wnd_size: Size, viewport: Rect, surface_color: Color, commands: &[PaintCommand],
  ) {
    self.last_frame =
      Some(Frame { commands: commands.to_owned(), viewport, surface: surface_color });
  }

  fn request_draw(&self) {}

  fn id(&self) -> WindowId { self.id }

  fn position(&self) -> Point { Point::new(0., 0.) }

  fn set_position(&mut self, _: Point) {}

  fn close(&self) {}
}

impl TestShellWindow {
  fn new(size: Size) -> Self {
    static ID: AtomicU64 = AtomicU64::new(0);
    TestShellWindow {
      cursor: CursorIcon::Default,
      id: ID.fetch_add(1, Ordering::Relaxed).into(),
      last_frame: None,
      surface_color: Color::WHITE,
      size,
    }
  }

  fn on_resize(&mut self, size: Size) {
    self.size = size;
    self.last_frame = None;
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
    let size = clamp.clamp(self.size);
    clamp.max = clamp.max.min(size);
    ctx.perform_single_child_layout(clamp);

    size
  }
  #[inline]
  fn size_affected_by_child(&self) -> bool { false }

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
  visual_rect: Option<Rect>,
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

  pub fn with_visual_rect(mut self, rect: Rect) -> Self {
    self.visual_rect = Some(rect);
    self
  }

  pub fn with_rect(self, rect: Rect) -> Self { self.with_pos(rect.origin).with_size(rect.size) }

  #[track_caller]
  pub fn check(&self, wnd: &TestWindow) {
    let Self { path, x, y, width, height, visual_rect } = self;

    let info = wnd.layout_info_by_path(path).unwrap();
    if let Some(x) = x {
      assert_eq!(info.pos.x, *x, "unexpected x");
    }
    if let Some(y) = y {
      assert_eq!(info.pos.y, *y, "unexpected y");
    }
    if let Some(w) = width {
      assert_eq!(info.size.unwrap().width, *w, "unexpected width");
    }
    if let Some(h) = height {
      assert_eq!(info.size.unwrap().height, *h, "unexpected height");
    }
    if let Some(rect) = visual_rect {
      assert_eq!(info.visual_box.bounds_rect(), Some(*rect), "unexpected visual rect");
    }
  }
}

pub struct WidgetTester {
  pub widget: GenWidget,
  pub wnd_size: Option<Size>,
  pub flags: Option<WindowFlags>,
  pub env_init: Option<Box<dyn FnOnce()>>,
  pub on_initd: Option<InitdFn>,
  pub comparison: Option<f64>,
}

type InitdFn = Box<dyn FnOnce(&mut TestWindow)>;

impl WidgetTester {
  pub fn new<K: ?Sized>(widget: impl RInto<GenWidget, K>) -> Self {
    Self {
      wnd_size: None,
      widget: widget.r_into(),
      on_initd: None,
      env_init: None,
      comparison: None,
      flags: None,
    }
  }

  pub fn new_with_data<K, D: 'static, W: IntoWidget<'static, K>>(
    data: D, widget_builder: impl Fn(&'static D) -> W + 'static,
  ) -> Self {
    let w = move || widget_builder(unsafe { &*(&data as *const _) }).into_widget();
    Self::new(w)
  }

  pub fn with_env_init(mut self, env_init: impl Fn() + 'static) -> Self {
    self.env_init = Some(Box::new(env_init));
    self
  }

  /// This callback runs after creating the window and drawing the first
  /// frame.
  pub fn on_initd(mut self, on_initd: impl Fn(&mut TestWindow) + 'static) -> Self {
    self.on_initd = Some(Box::new(on_initd));
    self
  }

  pub fn with_wnd_size(mut self, size: Size) -> Self {
    self.wnd_size = Some(size);
    self
  }

  pub fn with_flags(mut self, flags: WindowFlags) -> Self {
    self.flags = Some(self.flags.unwrap_or(WindowFlags::empty()) | flags);
    self
  }

  pub fn with_comparison(mut self, comparison: f64) -> Self {
    self.comparison = Some(comparison);
    self
  }

  pub fn create_wnd(&mut self) -> TestWindow {
    if let Some(env_init) = self.env_init.take() {
      env_init();
    }

    let wnd_size = self.wnd_size.unwrap_or(Size::new(1024., 1024.));
    let mut wnd =
      TestWindow::new(self.widget.clone(), wnd_size, self.flags.unwrap_or(WindowFlags::empty()));

    if let Some(initd) = self.on_initd.take() {
      initd(&mut wnd);
    }

    wnd.draw_frame();
    wnd
  }

  #[track_caller]
  pub fn layout_check(mut self, cases: &[LayoutCase]) {
    let wnd = self.create_wnd();

    cases.iter().for_each(|c| c.check(&wnd));
  }
}

impl Default for LayoutCase {
  fn default() -> Self {
    Self { path: &[0], x: None, y: None, width: None, height: None, visual_rect: None }
  }
}
