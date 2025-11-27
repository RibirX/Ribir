/// This macro is the equivalent of combining `widget_layout_test!` and
/// `widget_image_test!`.
#[macro_export]
macro_rules! widget_test_suit {
  ($name:ident, $widget_tester:expr) => {
    widget_image_tests!($name, $widget_tester);
  };
  ($name:ident, $widget_tester:expr, $($case:expr),+) => {
    widget_layout_test!($name, $widget_tester, $($case),+);
    widget_image_tests!($name, $widget_tester);
  };
}

/// This macro generates a layout test for your widget. It requires the test
/// name as the first parameter and an expression that returns a `WidgetTester`.
/// Then specify the expected `LayoutCase`.
///
/// The macro uses the name to create a test with a '_layout' suffix.
///
/// Note: This macro relies on the `TestWindow` in `ribir_core`, so make sure to
/// import `ribir_core::test_helper::*;` before using it.
#[macro_export]
macro_rules! widget_layout_test {
  ($name:ident, $widget_tester:expr, $($case:expr),+ $(,)?) => {
    paste::paste! {
      #[test]
      fn [<$name _layout>]() {
        let _scope = unsafe { AppCtx::new_lock_scope() };
        $widget_tester.layout_check(&[$($case),+]);
      }
    }
  };
}

/// The macro generates image tests for a widget. It requires the test name as
/// the first parameter and an expression that returns a `WidgetTester`.
///
/// It will produce tests for the widget with every theme and painter backend.
/// The test and image file names are formatted as `{widget name}_with_{theme
/// name}_by_{painter backend name}`.
///
/// The image file is stored in the `test_cases` folder at the workspace's root,
/// relative to the test source path.
///
/// To run the test and overwrite the image file, you can use
/// `RIBIR_IMG_TEST=overwrite`. For instance: ```
/// RIBIR_IMG_TEST=overwrite cargo test --smoke
/// ```
#[cfg(not(target_arch = "wasm32"))]
#[macro_export]
macro_rules! widget_image_tests {
  ($name:ident, $widget_tester:expr $(,)?) => {
    paste::paste! {
      #[test]
      fn [<$name _with_default_by_wgpu>]() {
        let _scope = unsafe { AppCtx::new_lock_scope() };
        svg::svg_registry::clear();
        unsafe { AppCtx::set_app_theme(ribir_slim::purple()) };

        let name = format!("{}_with_default_by_wgpu", std::stringify!($name));
        $crate::assert_widget_eq_image!($widget_tester, name);
      }

      #[test]
      fn [<$name _with_material_by_wgpu>]() {
        let _scope = unsafe { AppCtx::new_lock_scope() };
        svg::svg_registry::clear();
        unsafe { AppCtx::set_app_theme(ribir_material::purple::light()) };

        let name = format!("{}_with_material_by_wgpu", std::stringify!($name));
        $crate::assert_widget_eq_image!($widget_tester, name);
      }
    }
  };
}

#[macro_export]
macro_rules! assert_widget_eq_image {
  ($widget_tester:expr, $name:expr) => {
    let img_path = $crate::test_case_name!($name, "png");

    let mut wnd = $widget_tester.create_wnd();
    wnd.0.draw_frame(None);

    let Frame { commands, viewport, surface } = wnd.take_last_frame().unwrap();
    let viewport = viewport.to_i32().cast_unit();
    let img = $crate::wgpu_render_commands(&commands, viewport, surface);

    let mut img_test = $crate::ImageTest::new(img, &img_path);
    if let Some(c) = $widget_tester.comparison {
      img_test = img_test.with_comparison(c);
    }
    img_test.test();
  };
}
