/// Macro help to write an example. This macro accepts a function that returns a
/// widget as the root of the application, you can specify the window size by
/// `wnd_size = { size }`. It will generate codes for you:
///
/// - the `main` function of startup application
///   - use the `wnd_size` you provide or a default size `400x600`
///   - use the package name in `Cargo.toml` as the window title.
/// - generate an image test for the root widget, to ensure every modification
///   is work for your example.
/// - generate an bench test for the root widget, so we can continue to track
///   the performance of the example.
///
/// We may add in the future:
///
/// - report the bundle binary size of this example.
/// - report the startup time of the example.
/// - report the memory and gpu memory used in this example.
/// - report how many frames it can render in one second when vsync-off.

#[macro_export]
macro_rules! example_framework {
  (
    $widget_fn: ident $(,)?
  ) => {
    example_framework!($widget_fn, wnd_size = Size::new(400., 600.));
  };
  (
    $widget_fn: ident,
    wnd_size = $size: expr $(,)?
  ) => {
    #[cfg(test)]
    use ribir::core::test_helper::*;
    #[cfg(test)]
    extern crate test;
    #[cfg(test)]
    use ribir::material as ribir_material;
    #[cfg(test)]
    use test::Bencher;

    widget_bench!($widget_fn, wnd_size = $size);
    widget_image_test!($widget_fn, wnd_size = $size,);

    fn main() {
      unsafe {
        AppCtx::set_app_theme(material::purple::light());
      }
      let name = env!("CARGO_PKG_NAME");
      App::new_window($widget_fn(), Some($size)).set_title(name);
      App::exec();
    }
  };
}
