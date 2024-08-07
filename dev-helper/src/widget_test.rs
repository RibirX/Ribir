/// This macro is equivalent to writing `widget_layout_test!`,
/// `widget_image_test!`, `widget_bench!` three macros at the same time.
#[macro_export]
macro_rules! widget_test_suit {
  (
    $widget_fn: ident,
    wnd_size = $size: expr,
    $({
      path = $path: expr,
      $(x == $x_expect: expr,)?
      $(y == $y_expect: expr,)?
      $(width == $width_expect: expr,)?
      $(height == $height_expect: expr,)?
      $(rect == $rect_expect: expr,)?
      $(size == $size_expect: expr,)?
    })+
    $(comparison = $comparison: expr)?
  ) => {
    widget_layout_test!(
      $widget_fn,
      wnd_size = $size,
      $({
        path = $path,
        $(x == $x_expect,)?
        $(y == $y_expect,)?
        $(width == $width_expect,)?
        $(height == $height_expect,)?
        $(rect == $rect_expect,)?
        $(size == $size_expect,)?
      })+
    );
    widget_image_test!($widget_fn, wnd_size = $size $(,comparison = $comparison)?);
  };

  (
    $widget_fn: ident,
    wnd_size = $size: expr,
    $(x == $x_expect: expr,)?
    $(y == $y_expect: expr,)?
    $(width == $width_expect: expr,)?
    $(height == $height_expect: expr,)?
    $(rect == $rect_expect: expr,)?
    $(size == $size_expect: expr,)?
    $(comparison = $comparison: expr)?
  ) =>{
    widget_test_suit!(
      $widget_fn,
      wnd_size = $size,
      {
        path = [0],
        $(x == $x_expect,)?
        $(y == $y_expect,)?
        $(width == $width_expect,)?
        $(height == $height_expect,)?
        $(rect == $rect_expect,)?
        $(size == $size_expect,)?
      }
      $(comparison = $comparison)?
    );
  };
  (
    $widget_fn: ident,
    $($t:tt)+
  ) => {
    widget_test_suit!($widget_fn, wnd_size = Size::new(1024., 1024.), $($t)+);
  };
}

/// This macro generates a layout test for your widget. The first parameter must
/// be a function that returns the widget you want to test, and the macro will
/// use the function name to generate a test with a '_layout' suffix. And
/// then you can specify the window size for testing by `wnd_size = { expression
/// for window size}`, if you have not specified, the window will use
/// `1024x1024` as its default size.
///
/// Then you should provide the expected layout information of a widget index
/// path, See the document of the `assert_layout_result_by_path` macro to learn
/// how to write a expected layout information with a widget index path.
///
/// If you only care about the layout information of the whole widget and ignore
/// the inner information. You can directly use the `{key} == { expression }`
/// expression to describe what you expected. See the document of the
/// `assert_layout_result_by_path` macro to learn the list of the `key`
/// supported.
///
/// Notice: This macro depends on the `TestWindow` in `ribir_core`, you should
/// import `ribir_core::test_helper::*;` first.
///
/// # Examples
///
/// ``` rust
/// use ribir_core::{ prelude::*, test_helper::* };
/// use ribir_dev_helper::*;
///
/// fn my_widget() -> Widget<'static> {
///   fn_widget!{
///     @MockBox {
///       size: Size::new(100., 100.),
///       @MockBox {
///         size: Size::new(50., 50.)
///       }
///     }
///   }
///   .into_widget()
/// }
///
/// // only use to avoid conflict.
/// fn my_widget_a() -> Widget<'static> { my_widget() }
///
/// // Only test the whole widget size.
/// widget_layout_test!(my_widget_a, width == 100., height == 100.,);
///
/// fn my_widget_b() -> Widget<'static> { my_widget() }
/// // Only test the whole widget size but with a window size.
/// widget_layout_test!(
///   my_widget_b,
///   wnd_size = Size::new(10., 10),
///   width == 10.,
///   height == 10.,
/// );
///
/// fn my_widget_c() -> Widget<'static> { my_widget() }
/// // Test two widget layout information.
/// widget_layout_test!(
///   my_widget_c,
///   wnd_size = Size::new(10., 10),
///   { path = [0], width == 100., height == 100., }
///   { path = [0, 0], width == 50., height == 50., }
/// );
/// ```
#[macro_export]
macro_rules! widget_layout_test {

  (
    $widget_fn: ident,
    wnd_size = $size: expr,
    $({
      path = $path: expr,
      $(x == $x_expect: expr,)?
      $(y == $y_expect: expr,)?
      $(width == $width_expect: expr,)?
      $(height == $height_expect: expr,)?
      $(rect == $rect_expect: expr,)?
      $(size == $size_expect: expr,)?
    })+
  ) => {
    paste::paste! {
      #[test]
      fn [<$widget_fn _layout>]() {
        let _scope = unsafe { AppCtx::new_lock_scope() };

        let mut wnd = TestWindow::new_with_size($widget_fn(), $size);
        wnd.draw_frame();

        assert_layout_result_by_path!(
          wnd,
          $({
            path = $path,
            $(x == $x_expect,)?
            $(y == $y_expect,)?
            $(width == $width_expect,)?
            $(height == $height_expect,)?
            $(rect == $rect_expect,)?
            $(size == $size_expect,)?
          })+
        );

      }
    }
  };
  (
    $widget_fn: ident,
    wnd_size = $size: expr,
    $(x == $x_expect: expr,)?
    $(y == $y_expect: expr,)?
    $(width == $width_expect: expr,)?
    $(height == $height_expect: expr,)?
    $(rect == $rect_expect: expr,)?
    $(size == $size_expect: expr,)?
  ) =>{
    widget_layout_test!(
      $widget_fn,
      wnd_size = $size,
      {
        path = [0],
        $(x == $x_expect,)?
        $(y == $y_expect,)?
        $(width == $width_expect,)?
        $(height == $height_expect,)?
        $(rect == $rect_expect,)?
        $(size == $size_expect,)?
      }
    );
  };
  (
    $widget_fn: ident,
    $($t:tt)+
  ) => {
    widget_layout_test!($widget_fn, wnd_size = Size::new(1024., 1024.), $($t)+);
  };
}

/// This macro generates image tests for a widget. The first parameter must be a
/// function that returns the widget you want to test. And the macro will
/// generate tests for the widget with every theme and painter backend. The test
/// and image file name was formatted by `{widget name} _with_{theme
/// name}_by_{painter backend name}`.
///
/// The image file is read from the `test_cases` folder in the workspace root
/// with the test source path.
///
/// You can run the test with `RIBIR_IMG_TEST=overwrite` to overwrite the image
/// file, for example ```
/// RIBIR_IMG_TEST=overwrite cargo test -- smoke
/// ```
#[cfg(not(target_arch = "wasm32"))]
#[macro_export]
macro_rules! widget_image_test {
  ($widget_fn:ident, wnd_size = $size:expr $(,comparison = $comparison:expr)?) => {
    paste::paste! {
      #[test]
      fn [<$widget_fn _with_default_by_wgpu>]() {
        let _scope = unsafe { AppCtx::new_lock_scope() };
        let mut wnd = TestWindow::new_with_size($widget_fn(), $size);
        wnd.draw_frame();
        let Frame { commands, viewport, surface} = wnd.take_last_frame().unwrap();
        let viewport = viewport.to_i32().cast_unit();
        let img = wgpu_render_commands(&commands, viewport, surface);
        let name = format!("{}_with_default_by_wgpu", std::stringify!($widget_fn));
        let file_path = test_case_name!(name, "png");
        ImageTest::new(img, &file_path)
          $(.with_comparison($comparison))?
          .test();
      }

      #[test]
      fn [<$widget_fn _with_material_by_wgpu>]() {
        let _scope = unsafe { AppCtx::new_lock_scope() };
        unsafe { AppCtx::set_app_theme(ribir_material::purple::light()) };

        let mut wnd = TestWindow::new_with_size($widget_fn(), $size);
        wnd.draw_frame();
        let Frame { commands, viewport, surface} = wnd.take_last_frame().unwrap();
        let viewport = viewport.to_i32().cast_unit();
        let img = wgpu_render_commands(&commands, viewport, surface);
        let name = format!("{}_with_material_by_wgpu", std::stringify!($widget_fn));
        let file_path = test_case_name!(name, "png");
        ImageTest::new(img, &file_path)
          $(.with_comparison($comparison))?
          .test();
      }
    }
  };
  ($widget_fn:ident $(,)?) => {
    widget_image_test!($widget_fn, wnd_size = Size::new(128., 128.),);
  };
}

#[allow(clippy::test_attr_in_doctest)]
/// Macro is used to check if the layout information of a widget is as expected.
/// At first, it accepts a `TestWindow` that contains the widgets you want to
/// test. Then use a pair of braces to describe the layout information of a
/// widget, in the braces inner, you should specify the widget by an index path,
/// then use `{key} == {expression}` to describe what you want to test the
/// widget.
///
/// - For the index path:
///  - [0, 1] means use the second child of the root, the root level must be 0.
///  - [0, 1, 2] means the first node at the root level (must be 0), then down
///    to its  second child, then down to the third child.
/// - For the `{key} == {expression}`, you can use:
///  - `x` with an expression returns an `f32` to check the widget `x` position.
///  - `y` with an expression returns an `f32` to check the widget `y` position.
///  - `width` with an expression returns an `f32` to check the width of the
///    widget.
///  - `height` with an expression returns an `f32` to check the height of the
///    widget.
///  - `size` with an expression returns a `Size` to check the size of the
///    widget.
///  - `rect` with an expression returns a `Rect` to check the whole box of the
///    widget.
///
///# Examples
///
///``` rust
/// use ribir_core::{ prelude::*, test_helper::* };
/// use ribir_dev_helper::*;
///
/// #[test]
/// fn assert_layout_result_by_path_example(){
///  reset_test_env!();
///  let w = widget!{
///    MockBox {
///      size: Size::new(100., 100.),
///      MockBox {
///        size: Size::new(50., 50.)
///      }
///    }
///  };
///
///  let mut wnd = TestWindow::new(w);
///  wnd.draw_frame();
///
///  assert_layout_result_by_path!(
///    wnd,
///    { path = [0], width == 100., height == 100. }
///    { path = [0, 0], width == 50., height == 50. }
///  );  
/// }
/// ```
#[macro_export]
macro_rules! assert_layout_result_by_path {
  (
    $test_wnd: ident,
    $({
      path = $path: expr,
      $(x == $x_expect: expr,)?
      $(y == $y_expect: expr,)?
      $(width == $width_expect: expr,)?
      $(height == $height_expect: expr,)?
      $(rect == $rect_expect: expr,)?
      $(size == $size_expect: expr,)?
    })+
    ) => {
      $(
        let info = $test_wnd.layout_info_by_path(&$path).unwrap();
        $(assert_eq!(info.pos.x, $x_expect, "unexpected x");)?
        $(assert_eq!(info.pos.y, $y_expect, "unexpected y");)?
        $(assert_eq!(info.size.unwrap().width, $width_expect, "unexpected width");)?
        $(assert_eq!(info.size.unwrap().height, $height_expect, "unexpected height");)?
        $(assert_eq!(info.size.unwrap(), $size_expect, "unexpected size");)?
        $(
          let size = info.size.unwrap();
          assert_eq!(Rect::new(info.pos, size), $rect_expect, "unexpected rect");
        )?
      )+
  };
}
