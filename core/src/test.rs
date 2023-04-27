use crate::{impl_query_self_only, prelude::*, widget::WidgetTree};

#[derive(Default, Clone, Copy)]
pub struct ExpectRect {
  pub x: Option<f32>,
  pub y: Option<f32>,
  pub width: Option<f32>,
  pub height: Option<f32>,
}
pub struct LayoutTestItem<'a> {
  pub path: &'a [usize],
  pub expect: ExpectRect,
}

pub fn expect_layout_result_with_theme<M: ImplMarker, W: IntoWidget<M>>(
  w: W,
  wnd_size: Option<Size>,
  theme: Theme,
  items: &[LayoutTestItem],
) {
  let ctx = AppContext {
    app_theme: std::rc::Rc::new(theme),
    ..<_>::default()
  };
  let mut wnd = Window::mock_window(w, wnd_size.unwrap_or_else(|| Size::new(1024., 1024.)), ctx);
  wnd.draw_frame();
  items.iter().for_each(|LayoutTestItem { path, expect }| {
    assert_layout_result(&wnd, path, expect);
  });
}

pub fn expect_layout_result<M: ImplMarker, W: IntoWidget<M>>(
  w: W,
  wnd_size: Option<Size>,
  items: &[LayoutTestItem],
) {
  let mut wnd = Window::default_mock(w, wnd_size);
  wnd.draw_frame();
  items.iter().for_each(|LayoutTestItem { path, expect }| {
    assert_layout_result(&wnd, path, expect);
  });
}
pub fn assert_layout_result(wnd: &Window, path: &[usize], expect: &ExpectRect) {
  let rect = layout_rect_by_path(wnd, path);
  if let Some(x) = expect.x {
    assert_eq!(rect.min_x(), x, "path: {path:?}");
  }
  if let Some(y) = expect.y {
    assert_eq!(rect.min_y(), y, "path: {path:?}");
  }
  if let Some(width) = expect.width {
    assert_eq!(rect.width(), width, "path: {path:?}")
  }

  if let Some(height) = expect.height {
    assert_eq!(rect.height(), height, "path: {path:?}")
  }
}

// stripped the framework's auxiliary widget, return the WidgetId of the user's
// real content widget
fn content_widget_id(tree: &WidgetTree) -> WidgetId {
  let root = tree.root();
  root.first_child(&tree.arena).unwrap()
}

/// ues a index path to access widget tree and return the layout info,
/// [0, 1] means use the second child of the root.
/// [0, 1, 2] the first node at the root level (must be 0), then down to its
/// second child, then down to third child.
pub fn layout_rect_by_path(wnd: &Window, path: &[usize]) -> Rect {
  let info = layout_info_by_path(wnd, content_widget_id(&wnd.widget_tree), path).unwrap();
  Rect::new(info.pos, info.size.unwrap())
}

pub fn layout_size_by_path(wnd: &Window, path: &[usize]) -> Size {
  let info = layout_info_by_path(wnd, content_widget_id(&wnd.widget_tree), path).unwrap();
  info.size.unwrap()
}

pub fn layout_position_by_path(wnd: &Window, path: &[usize]) -> Point {
  let info = layout_info_by_path(wnd, content_widget_id(&wnd.widget_tree), path).unwrap();
  info.pos
}

pub fn layout_info_by_path<'a>(
  wnd: &'a Window,
  root: WidgetId,
  path: &[usize],
) -> Option<&'a LayoutInfo> {
  assert_eq!(path[0], 0);
  let tree: &widget::WidgetTree = &wnd.widget_tree;
  let mut node = root;

  for (level, idx) in path[1..].iter().enumerate() {
    node = node.children(&tree.arena).nth(*idx).unwrap_or_else(|| {
      panic!("node no exist: {:?}", &path[0..level]);
    });
  }

  tree.store.layout_info(node)
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
impl Query for MockMulti {
  impl_query_self_only!();
}
impl Window {
  #[inline]
  pub fn widget_count(&self) -> usize {
    let wid = content_widget_id(&self.widget_tree);
    self.widget_tree.count(wid)
  }
}

#[allow(unused)]
macro count {
  () => (0usize),
  ( $x:tt $($xs:tt)* ) => (1usize + count!($($xs)*))
}

impl ExpectRect {
  pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
    Self {
      x: Some(x),
      y: Some(y),
      width: Some(width),
      height: Some(height),
    }
  }

  pub fn from_size(size: Size) -> Self {
    Self {
      width: Some(size.width),
      height: Some(size.height),
      ..Default::default()
    }
  }

  pub fn from_point(pos: Point) -> Self {
    Self {
      x: Some(pos.x),
      y: Some(pos.y),
      ..Default::default()
    }
  }

  pub fn from_rect(rect: Rect) -> Self {
    Self {
      x: Some(rect.min_x()),
      y: Some(rect.min_y()),
      width: Some(rect.width()),
      height: Some(rect.height()),
    }
  }
}

/// A unit test help macro to describe the test flow. This macro provide ability
/// to pack many unit tests, and print the result like official. Should always
/// use official test harness first, use it only when you need execute test by
/// self.
///
/// This macro depends on crate `colored`.
///
/// # Example
///
/// ```
/// use ribir_core::test::unit_test_describe;
///
/// fn test_first() {}
///
/// fn test_second() {}
///
/// fn main() {
///   use colored::Colorize;
///
///   unit_test_describe!{
///     run_unit_test(test_first);
///     run_unit_test(test_second);
///   }
/// }
/// ```

pub macro unit_test_describe($(run_unit_test($name: path);)* ) {{
  let panic_infos: std::sync::Arc<std::sync::Mutex<Vec<String>>> = Default::default();

  // hook panic to format message
  let c_infos = panic_infos.clone();
  std::panic::set_hook(Box::new(move |info| {
    println!("... {}", "failed".red());
    let info_str = format!("{}", info);
    c_infos.lock().unwrap().push(info_str)
  }));

  let count = count!($($name)*);

  println!("running {} tests", count);
  let mut res  = Result::Ok(());
  // catch panic and continue execute unit tests.
  $(
   res = std::panic::catch_unwind(|| {
      // run the unit tests
      print!("test {}::{} ", module_path!(), stringify!($name));
      $name();
      println!("... {}", "ok".green());
    }).and(res);
  )*

  // remove panic hook
  let _ = std::panic::take_hook();

  // unit tests result message.
  let infos = panic_infos.lock().unwrap();
  let failed = infos.len();
  let pass = count - failed;
  let result = if res.is_err() {
    "failed".red()
  } else {
    "ok".green()
  };
  println!("");
  println!("test results: {}. {} passed; {} failed;\n", result, pass, failed);

  if !infos.is_empty() {
    println!("--------- {} failed stdout ---------", module_path!());
    infos.iter().for_each(|info| println!("{}", info))
  }

  println!("");

  if let Result::Err(err) = res {
    std::panic::resume_unwind(err);
  }
}}
