/// This macro generates a layout test for your widget. The first parameter must
/// be a function name that returns the widget you want to test, and the macro
/// will use the function name to generate a test with a '_layout' suffix. And
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
/// fn my_widget() -> Widget {
///   widget!{
///     MockBox {
///       size: Size::new(100., 100.),
///       MockBox {
///         size: Size::new(50., 50.)
///       }
///     }
///   }
/// }
///
/// // only use to avoid conflict.
/// fn my_widget_a() -> Widget { my_widget() }
///
/// // Only test the whole widget size.
/// widget_layout_test!(my_widget_a, width == 100., height == 100.,);
///
/// fn my_widget_b() -> Widget { my_widget() }
/// // Only test the whole widget size but with a window size.
/// widget_layout_test!(
///   my_widget_b,
///   wnd_size = Size::new(10., 10),
///   width == 10.,
///   height == 10.,
/// );
///
/// fn my_widget_c() -> Widget { my_widget() }
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
    $($t:tt)+
  ) => {
    widget_layout_test!(
      window = TestWindow::new_with_size($widget_fn(), $size),
      $widget_fn,
      $($t)+
    );
  };
  (
    $widget_fn: ident,
    $($t:tt)+
  ) => {
    widget_layout_test!(window = TestWindow::new($widget_fn()), $widget_fn, $($t)+);
  };
  (
    window = $wnd: expr,
    $widget_fn: ident,
    $(x == $x_expect: expr,)?
    $(y == $y_expect: expr,)?
    $(width == $width_expect: expr,)?
    $(height == $height_expect: expr,)?
    $(rect == $rect_expect: expr,)?
    $(size == $size_expect: expr,)?
  ) =>{
    widget_layout_test!(
      window = $wnd,
      $widget_fn,
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
    window = $wnd: expr,
    $widget_fn: ident,
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
      fn [<layout_ $widget_fn>]() {
        let mut wnd = $wnd;
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
}

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
        $(assert_eq!($x_expect, info.pos.x, "unexpected x");)?
        $(assert_eq!($y_expect, info.pos.y, "unexpected y");)?
        $(assert_eq!($width_expect, info.size.unwrap().width, "unexpected width");)?
        $(assert_eq!($height_expect, info.size.unwrap().height, "unexpected height");)?
        $(assert_eq!($size_expect, info.size.unwrap(), "unexpected size");)?
        $(
          let size = info.size.unwrap();
          assert_eq!(Rect::new(info.pos, size), $rect_expect, "unexpected rect");
        )?
      )+
  };
}
