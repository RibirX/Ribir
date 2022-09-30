pub mod embed_post;
pub mod key_embed_post;
pub mod recursive_row;

use crate::prelude::*;

// return the flex box rect, and rect of its children.
pub fn widget_and_its_children_box_rect(root: Widget, window_size: Size) -> (Rect, Vec<Rect>) {
  let mut wnd = Window::without_render(root, window_size);
  wnd.draw_frame();

  root_and_children_rect(&mut wnd)
}

pub fn root_and_children_rect(wnd: &Window) -> (Rect, Vec<Rect>) {
  let tree = &wnd.widget_tree;
  let root = tree.root();
  let rect = tree.layout_box_rect(root).unwrap();
  let children_box_rect = root
    .children(tree)
    .map(|c| tree.layout_box_rect(c).unwrap())
    .collect();

  (rect, children_box_rect)
}

#[derive(Default)]
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

pub fn expect_layout_result(wnd_size: Size, w: Widget, items: &[LayoutTestItem]) {
  let mut wnd = Window::without_render(w, wnd_size);
  wnd.draw_frame();
  items.iter().for_each(|LayoutTestItem { path, expect }| {
    let res = layout_info_by_path(&wnd, path);
    if let Some(x) = expect.x {
      assert_eq!(x, res.min_x());
    }
    if let Some(y) = expect.y {
      assert_eq!(y, res.min_y());
    }
    if let Some(width) = expect.width {
      assert_eq!(width, res.width())
    }

    if let Some(height) = expect.height {
      assert_eq!(height, res.height())
    }
  });
}

/// ues a index path to access widget tree and return the layout info,
/// [0, 1] means use the second child of the root.
/// [0, 1, 2] the first node at the root level (must be 0), then down to its
/// second child, then down to third child.
pub fn layout_info_by_path(wnd: &Window, path: &[usize]) -> Rect {
  assert_eq!(path[0], 0);
  let tree = &wnd.widget_tree;
  let mut node = tree.root();
  for (level, idx) in path[1..].into_iter().enumerate() {
    node = node.children(tree).skip(*idx).next().unwrap_or_else(|| {
      panic!("node no exist: {:?}", &path[0..level]);
    });
  }

  tree.layout_box_rect(node).unwrap()
}

impl Window {
  #[inline]
  pub fn widget_count(&self) -> usize { self.widget_tree.count() }
}

#[allow(unused)]
macro count {
  () => (0usize),
  ( $x:tt $($xs:tt)* ) => (1usize + count!($($xs)*))
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
/// use ribir::test::unit_test_describe;
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

pub macro unit_test_describe($(run_unit_test($name: ident);)* ) {{
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
