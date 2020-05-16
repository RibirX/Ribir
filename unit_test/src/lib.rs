#![feature(decl_macro)]

use canvas::*;
#[allow(unused_imports)]
use colored::*;
use futures::executor::block_on;
#[allow(unused_imports)]
use std::sync::{Arc, Mutex};

#[allow(dead_code)]
pub fn write_frame_to<S: Surface>(mut frame: NewTextureFrame<S>, path: &str) {
  let abs_path = format!("{}/{}", env!("CARGO_MANIFEST_DIR"), path);
  let _ = block_on(frame.save_as_png(&abs_path));
}

/// check if the frame is equal to the image at `path`, the path relative the
/// package root;
pub macro assert_frame_eq($frame: expr, $path: expr $(,)?) {
  let abs_path = format!("{}/{}", env!("CARGO_MANIFEST_DIR"), $path);
  let file_data = std::fs::read(abs_path.clone()).unwrap();

  let mut frame_data = vec![];
  let cursor = std::io::Cursor::new(&mut frame_data);
  block_on($frame.to_png(cursor)).unwrap();

  if file_data != frame_data {
    panic!(
      "{}",
      format!(
        "Frame is not same with `{}`,\nMaybe you want use `write_frame_to` to save frame as png to compare.",
        abs_path
      )
    );
  }
}

#[allow(unused_macros)]
macro count {
  () => (0usize),
  ( $x:tt $($xs:tt)* ) => (1usize + count!($($xs)*))
}

/// A unit test help macro to describe the test flow. This macro provide ability
/// to pack many unit tests, and print the result like official. Should always
/// use official test harness first, use it only when you need execute test by
/// self.
///
/// # Example
/// fn test_first() {}
///
/// fn test_second {}
///
/// fn main() {
///   unit_test_describe!{
///     run_unit_test(test_first);
///     run_unit_test(test_second);
///   }
/// }
pub macro unit_test_describe($(run_unit_test($name: ident);)* ) {{
  println!("");

  let panic_infos: Arc<Mutex<Vec<String>>> = Default::default();

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

#[cfg(test)]

mod tests {
  use super::*;

  #[test]
  #[should_panic = "test 2 panic!"]
  fn smoke() {
    fn test1() {
      panic!("test 1 panic!");
    }

    fn test2() {
      panic!("test 2 panic!");
    }

    unit_test_describe! {
      run_unit_test(test1);
      run_unit_test(test2);
    }
  }
}
