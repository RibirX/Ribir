use super::device::{AbstractDevice, Device};
use pathfinder_canvas::CanvasFontContext;
use pathfinder_color::ColorF;
pub use pathfinder_geometry::vector::{Vector2F, Vector2I};
use pathfinder_renderer::{
  concurrent::rayon::RayonExecutor,
  concurrent::scene_proxy::SceneProxy,
  gpu::{
    options::{DestFramebuffer, RendererOptions},
    renderer::Renderer,
  },
  options::BuildOptions,
};
use pathfinder_resources::fs::FilesystemResourceLoader;
use winit::{dpi::PhysicalSize, window::Window as NativeWindow};

pub struct Canvas {
  device: Device,
  renderer: Renderer<<Device as AbstractDevice>::D>,
  font_ctx: CanvasFontContext,
}

impl Canvas {
  pub(crate) fn new(size: PhysicalSize<u32>) -> Self {
    let device = Device::new();
    let size = Vector2I::new(size.width as i32, size.height as i32);
    // Create a Pathfinder renderer.
    let renderer = Renderer::new(
      device.native_device(),
      &FilesystemResourceLoader::locate(),
      DestFramebuffer::full_window(size),
      RendererOptions {
        background_color: Some(ColorF::white()),
      },
    );

    Self {
      device,
      renderer,
      font_ctx: CanvasFontContext::from_system_source(),
    }
  }

  #[inline]
  pub(crate) fn attach(&self, window: &NativeWindow) {
    self.device.attach(window);
  }

  pub(crate) fn get_context_2d(
    &self,
    size: PhysicalSize<u32>,
  ) -> CanvasRenderingContext2D {
    CanvasRenderingContext2D(
      pathfinder_canvas::Canvas::new(
        Vector2I::new(size.width as i32, size.height as i32).to_f32(),
      )
      .get_context_2d(self.font_ctx.clone()),
    )
  }

  pub(crate) fn commit_frame(
    &mut self,
    rendering_context: CanvasRenderingContext2D,
  ) {
    let scene = SceneProxy::from_scene(
      rendering_context.0.into_canvas().into_scene(),
      RayonExecutor,
    );
    scene.build_and_render(&mut self.renderer, BuildOptions::default());
    self.renderer.device.present_drawable();
  }
}

/// Prevent export `pathfinder_canvas::CanvasRenderingContext2D` to keep
/// capability.
pub struct CanvasRenderingContext2D(
  pathfinder_canvas::CanvasRenderingContext2D,
);

impl CanvasRenderingContext2D {
  #[inline]
  pub fn fill_text(&mut self, string: &str, position: Vector2F) {
    self.0.fill_text(string, position);
  }
}
