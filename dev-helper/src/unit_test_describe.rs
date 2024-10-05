/// A unit test macro to help describe the test flow.
///
/// This macro provide ability to pack many unit tests, and print the result
/// like official. Should always use official test harness first, use it only
/// when you need execute test by self.
///
/// This macro depends on crate `colored`.
///
/// # Example
///
/// ```
/// use colored::Colorize;
/// use ribir_dev_helper::unit_test_describe;
///
/// fn test_first() {}
///
/// fn test_second() {}
///
/// fn main() {
///   unit_test_describe! {
///     run_unit_test(test_first);
///     run_unit_test(test_second);
///   }
/// }
/// ```
#[macro_export]
macro_rules! unit_test_describe {
  ($(run_unit_test($name: path);)* ) => {{
    let panic_infos: std::sync::Arc<std::sync::Mutex<Vec<String>>> = Default::default();

    // hook panic to format message
    let c_infos = panic_infos.clone();
    std::panic::set_hook(Box::new(move |info| {
      println!("... {}", "failed".red());
      let info_str = format!("{}", info);
      c_infos.lock().unwrap().push(info_str)
    }));

    let count = unit_test_describe!(count $($name)*);

    println!("running {} test", count);
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


    if let Result::Err(err) = res {
      std::panic::resume_unwind(err);
    }
  }};
  (count) => (0usize);
  (count $x:tt $($xs:tt)* ) => {
    1usize + unit_test_describe!(count $($xs)*)
  };
}
