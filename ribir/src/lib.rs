// pub use ribir_core as core;
pub use ribir_app::prelude::*;
pub use ribir_winit::prelude::*;

pub use ribir_widgets as widgets;
pub mod prelude {
  pub use ribir_core::prelude::*;
  pub use ribir_widgets::prelude::*;
  pub mod app {
    use ribir_core::window::Window;

    #[cfg(feature = "wgpu_gl")]
    pub fn run(root: super::Widget) {
      use ribir_winit::prelude::{WindowBuilder, WinitApplication};

      let app = WinitApplication::new(super::material::purple::light());
      let wnd = WindowBuilder::new(root).build(&app);
      run_with_window(app, wnd);
    }

    #[cfg(feature = "wgpu_gl")]
    pub fn run_with_window(mut app: ribir_winit::prelude::WinitApplication, wnd: Window) {
      println!("WindowId: {:?}", wnd.raw_window.id());
      let wnd_id = app.add_window(wnd);
      println!("WindowId: {wnd_id:?}");
      app.exec(wnd_id);
    }
  }
}

use prelude::*;
#[cfg(feature = "wgpu_gl")]
pub fn wgpu_headless_wnd(root: Widget, ctx: AppContext, size: DeviceSize) -> Window {
  let p_backend = AppContext::wait_future(ribir_gpu::wgpu_backend_headless(
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
