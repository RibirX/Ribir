pub use ribir_core as core;
pub use ribir_widgets as widgets;
pub mod prelude {
  pub use ribir_core::prelude::*;
  pub use ribir_widgets::prelude::*;
  pub mod app {

    #[cfg(feature = "wgpu_gl")]
    pub fn run(root: super::Widget) {
      let mut app = super::Application::default();
      let wnd = app.new_window(|native_wnd, ctx| {
        let size = native_wnd.inner_size();
        let p_backend = super::AppContext::wait_future(gpu::wgpu_backend_with_wnd(
          &native_wnd,
          super::DeviceSize::new(size.width, size.height),
          None,
          None,
          ctx.shaper.clone(),
        ));
        super::Window::new(native_wnd, p_backend, root.into_widget(), ctx)
      });
      app.exec(wnd);
    }
  }
}

use prelude::*;
#[cfg(feature = "wgpu_gl")]
pub fn wgpu_headless_wnd(root: Widget, ctx: AppContext, size: DeviceSize) -> Window {
  let p_backend = AppContext::wait_future(gpu::wgpu_backend_headless(
    size,
    None,
    None,
    ctx.shaper.clone(),
  ));
  Window::new(
    ribir_core::window::MockRawWindow {
      size: Size::from_untyped(size.to_f32().to_untyped()),
      ..Default::default()
    },
    p_backend,
    root,
    ctx,
  )
}
