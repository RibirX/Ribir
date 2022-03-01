pub mod embed_post;
pub mod key_embed_post;
pub mod recursive_row;

use crate::prelude::*;

// return the flex box rect, and rect of its children.
pub fn widget_and_its_children_box_rect(root: BoxedWidget, window_size: Size) -> (Rect, Vec<Rect>) {
  let mut wnd = Window::without_render(root, window_size);
  wnd.render_ready();

  root_and_children_rect(&mut wnd)
}

pub fn root_and_children_rect(wnd: &Window) -> (Rect, Vec<Rect>) {
  let ctx = wnd.context();
  let tree = &ctx.widget_tree;
  let layout = &ctx.layout_store;
  let r_root = tree.root().render_widget(tree).unwrap();
  let rect = layout.layout_box_rect(r_root).unwrap();
  let children_box_rect = r_root
    .children(tree)
    .map(|c| {
      let rid = c.render_widget(tree).unwrap();
      layout.layout_box_rect(rid).unwrap()
    })
    .collect();

  (rect, children_box_rect)
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
