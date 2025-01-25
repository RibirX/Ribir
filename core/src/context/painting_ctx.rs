use ribir_geom::Transform;

use super::WidgetCtxImpl;
use crate::{
  prelude::{Painter, ProviderCtx, WidgetId},
  widget::WidgetTree,
};

pub struct PaintingCtx<'a> {
  id: WidgetId,
  tree: &'a WidgetTree,
  painter: &'a mut Painter,
  provider_ctx: ProviderCtx,
  /// The matrix transforms a content painter into the box widget painter.
  /// Widgets like `Background`, `Border`, etc., need to use the box painter to
  /// draw decorations and ignore the padding transform, starting from the
  /// widget box's origin. For example, in `Padding<Background<Text>>`, the
  /// text requires translation, but the background should not.
  /// The `Background` should utilize a `box_painter` that applies this matrix.
  box_offset: Transform,
}

impl<'a> WidgetCtxImpl for PaintingCtx<'a> {
  #[inline]
  fn id(&self) -> WidgetId { self.id }

  #[inline]
  fn tree(&self) -> &WidgetTree { self.tree }
}

impl<'a> PaintingCtx<'a> {
  pub(crate) fn new(id: WidgetId, tree: &'a WidgetTree, painter: &'a mut Painter) -> Self {
    let provider_ctx = if let Some(p) = id.parent(tree) {
      ProviderCtx::collect_from(p, tree)
    } else {
      ProviderCtx::default()
    };

    let box_offset = Transform::identity();
    Self { id, tree, painter, provider_ctx, box_offset }
  }

  /// Called by the framework when the painting widget is finished.
  #[inline]
  pub(crate) fn finish(&mut self) { self.provider_ctx.pop_providers_for(self.id); }

  #[inline]
  pub(crate) fn switch_to(&mut self, id: WidgetId) {
    self.box_offset = Transform::identity();
    self.id = id;
  }

  /// Apply a transformation apply only for the content but should not effect
  /// the box widget painter.
  pub fn content_only_transform_apply(&mut self, matrix: &Transform) {
    let box_matrix = self.box_offset.then(self.painter.transform());
    self.painter.apply_transform(matrix);
    self.box_offset = box_matrix.then(
      &self
        .painter
        .transform()
        .inverse()
        .unwrap_or_default(),
    );
  }

  /// Return the 2d painter to draw 2d things.
  #[inline]
  pub fn painter(&mut self) -> &mut Painter { self.painter }

  /// Provide a box painter that can draw decorations from the widget box's
  /// origin point without being affected by padding transformations. Any
  /// transform made to this painter will be restore when it is dropped.
  pub fn box_painter(&mut self) -> BoxPainter {
    let content = *self.painter.transform();
    let box_matrix = self.box_offset.then(&content);
    let painter = self.painter.set_transform(box_matrix);

    BoxPainter { content, painter }
  }

  #[inline]
  pub fn provider_ctx_and_painter(&mut self) -> (&mut ProviderCtx, &mut Painter) {
    (&mut self.provider_ctx, self.painter)
  }

  pub fn provider_ctx_and_box_painter(&mut self) -> (&mut ProviderCtx, BoxPainter) {
    // Safety: The Provider context and box painter operate independently.
    let provider_ctx = &mut (unsafe { &mut *(self as *mut Self) }).provider_ctx;
    let painter = self.box_painter();
    (provider_ctx, painter)
  }
}

impl<'w> AsRef<ProviderCtx> for PaintingCtx<'w> {
  #[inline]
  fn as_ref(&self) -> &ProviderCtx { &self.provider_ctx }
}

impl<'w> AsMut<ProviderCtx> for PaintingCtx<'w> {
  #[inline]
  fn as_mut(&mut self) -> &mut ProviderCtx { &mut self.provider_ctx }
}

pub struct BoxPainter<'a> {
  content: Transform,
  painter: &'a mut Painter,
}

impl std::ops::Deref for BoxPainter<'_> {
  type Target = Painter;

  #[inline]
  fn deref(&self) -> &Self::Target { self.painter }
}

impl std::ops::DerefMut for BoxPainter<'_> {
  #[inline]
  fn deref_mut(&mut self) -> &mut Self::Target { self.painter }
}

impl Drop for BoxPainter<'_> {
  fn drop(&mut self) { self.painter.set_transform(self.content); }
}
