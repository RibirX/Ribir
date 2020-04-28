use crate::*;
pub struct CanvasRenderingContext2D<'a> {
  canvas: &'a mut Canvas,
  buffer: LayerBuffer,
}

pub struct DrawInfo {}

impl<'a> CanvasRenderingContext2D<'a> {
  /// Create a new layer to drawing, and not effect current layer.
  #[inline]
  pub fn new_layer(&self) -> Rendering2DLayer { Rendering2DLayer::new() }

  /// Compose a layer into current drawing.
  pub fn compose_layer(&mut self, other_layer: Rendering2DLayer) -> &mut Self {
    self.compose_layer_buffer(&other_layer.finish())
  }

  /// Compose a layer buffer into current drawing. Layer buffer is the result
  /// after a layer drawing finished.
  #[inline]
  pub fn compose_layer_buffer(&mut self, buffer: &LayerBuffer) -> &mut Self {
    unimplemented!();
    if self.buffer.mergeable(buffer) {
      self.buffer.merge(&buffer)
    }

    self
  }

  /// Commit all composed layer to gpu for painting on screen.
  pub fn commit(&mut self) -> DrawInfo {
    unimplemented!();
  }
}
