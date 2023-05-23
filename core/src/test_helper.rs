use std::sync::atomic::{AtomicU64, Ordering};

use crate::{
  impl_query_self_only,
  prelude::*,
  window::{ShellWindow, WindowId},
};

pub struct Frame {
  pub commands: Vec<PaintCommand>,
  pub viewport: Rect,
  pub surface: Color,
}
pub struct TestWindow(Window);

impl TestWindow {
  /// Create a 1024x1024 window for test
  pub fn new<M: ImplMarker>(root: impl IntoWidget<M>) -> Self {
    Self(Window::new(
      root.into_widget(),
      Box::new(TestShellWindow::new(None)),
      <_>::default(),
    ))
  }

  pub fn new_with_size<M: ImplMarker>(root: impl IntoWidget<M>, size: Size) -> Self {
    Self(Window::new(
      root.into_widget(),
      Box::new(TestShellWindow::new(Some(size))),
      <_>::default(),
    ))
  }

  pub fn new_with_ctx<M: ImplMarker>(
    root: impl IntoWidget<M>,
    size: Size,
    ctx: AppContext,
  ) -> Self {
    Self(Window::new(
      root.into_widget(),
      Box::new(TestShellWindow::new(Some(size))),
      ctx,
    ))
  }

  /// Ues a index path to access widget tree and return the layout info,
  /// [0, 1] means use the second child of the root.
  /// [0, 1, 2] the first node at the root level (must be 0), then down to its
  /// second child, then down to third child.
  pub fn layout_info_by_path<'a>(&'a self, path: &[usize]) -> Option<&'a LayoutInfo> {
    assert_eq!(path[0], 0);
    let tree = &self.0.widget_tree;
    let mut node = tree.root();
    for (level, idx) in path[1..].iter().enumerate() {
      node = node.children(&tree.arena).nth(*idx).unwrap_or_else(|| {
        panic!("node no exist: {:?}", &path[0..level]);
      });
    }
    tree.store.layout_info(node)
  }

  #[inline]
  pub fn widget_count(&self) -> usize { self.widget_tree.count() }

  pub fn take_last_frame(&mut self) -> Option<Frame> {
    self
      .shell_wnd_mut()
      .as_any_mut()
      .downcast_mut::<TestShellWindow>()
      .unwrap()
      .last_frame
      .take()
  }
}

impl std::ops::Deref for TestWindow {
  type Target = Window;

  fn deref(&self) -> &Self::Target { &self.0 }
}

impl std::ops::DerefMut for TestWindow {
  fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}

pub struct TestShellWindow {
  pub size: Size,
  pub cursor: Option<CursorIcon>,
  pub id: WindowId,
  pub last_frame: Option<Frame>,
}

impl ShellWindow for TestShellWindow {
  fn inner_size(&self) -> Size { self.size }

  fn outer_size(&self) -> Size { self.size }

  fn set_size(&mut self, size: Size) { self.size = size; }

  fn set_cursor(&mut self, cursor: CursorIcon) { self.cursor = Some(cursor); }

  fn set_title(&mut self, _: &str) {}

  fn set_icon(&mut self, _: &PixelImage) {}

  fn set_ime_pos(&mut self, _: Point) {}

  fn as_any(&self) -> &dyn Any { self }

  fn as_any_mut(&mut self) -> &mut dyn Any { self }

  fn begin_frame(&mut self) {}

  fn draw_commands(&mut self, viewport: Rect, commands: Vec<PaintCommand>, surface: Color) {
    self.last_frame = Some(Frame { commands, viewport, surface });
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
      cursor: None,
      id: ID.fetch_add(1, Ordering::Relaxed).into(),
      last_frame: None,
    }
  }
}

#[derive(Declare, MultiChild)]
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

impl Query for MockStack {
  impl_query_self_only!();
}

#[derive(Declare, MultiChild)]
pub struct MockMulti;

#[derive(Declare, Clone, SingleChild)]
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

impl Query for MockMulti {
  impl_query_self_only!();
}

impl Query for MockBox {
  impl_query_self_only!();
}
