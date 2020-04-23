use pathfinder_canvas::{Canvas, CanvasFontContext};
pub use pathfinder_geometry::vector::{Vector2F, Vector2I};
use winit::dpi::PhysicalSize;

/// Prevent export `pathfinder_canvas::CanvasRenderingContext2D` to keep
/// compatibility.
pub struct CanvasRenderingContext2D(
  pathfinder_canvas::CanvasRenderingContext2D,
);

impl CanvasRenderingContext2D {
  #[inline]
  pub fn fill_text(&mut self, string: &str, position: Vector2F) {
    self.0.fill_text(string, position);
  }

  pub(crate) fn into_canvas(self) -> Canvas { self.0.into_canvas() }

  pub(crate) fn new(
    size: PhysicalSize<u32>,
    font_ctx: CanvasFontContext,
  ) -> Self {
    CanvasRenderingContext2D(
      pathfinder_canvas::Canvas::new(
        Vector2I::new(size.width as i32, size.height as i32).to_f32(),
      )
      .get_context_2d(font_ctx.clone()),
    )
  }
}
