use crate::widget::{RenderFul, WidgetBuilder};

use super::*;

/// Trait specify what child a widget can have, and the target type is the
/// result of widget compose its child.
pub trait SingleWithChild<C> {
  type Target;
  fn with_child(self, child: C, ctx: &BuildCtx) -> Self::Target;
}

/// A node of widget with not compose its child.
pub struct WidgetPair<W, C> {
  pub widget: W,
  pub child: C,
}

impl<W: SingleChild> SingleChild for Option<W> {}

impl<W: SingleChild, C> SingleWithChild<C> for W {
  type Target = WidgetPair<W, C>;

  #[inline]
  fn with_child(self, child: C, _: &BuildCtx) -> Self::Target { WidgetPair { widget: self, child } }
}

impl<W, C1: SingleChild, C2> SingleWithChild<C2> for WidgetPair<W, C1> {
  type Target = WidgetPair<W, WidgetPair<C1, C2>>;

  fn with_child(self, c: C2, ctx: &BuildCtx) -> Self::Target {
    let WidgetPair { widget, child } = self;
    WidgetPair {
      widget,
      child: child.with_child(c, ctx),
    }
  }
}

trait IntoSingleParent {
  fn into_single_parent(self) -> Box<dyn Render>;
}

trait WidgetChild {
  fn child_build(self, ctx: &BuildCtx) -> WidgetId;
}

impl IntoSingleParent for Box<dyn RenderSingleChild> {
  fn into_single_parent(self) -> Box<dyn Render> { self.into_render() }
}

impl<W: RenderSingleChild + 'static> IntoSingleParent for W {
  fn into_single_parent(self) -> Box<dyn Render> { Box::new(self) }
}

impl<W: RenderSingleChild + 'static> IntoSingleParent for Stateful<W> {
  #[inline]
  fn into_single_parent(self) -> Box<dyn Render> { Box::new(RenderFul(self)) }
}

impl<D> IntoSingleParent for Stateful<DynWidget<D>>
where
  D: RenderSingleChild + WidgetBuilder + 'static,
{
  #[inline]
  fn into_single_parent(self) -> Box<dyn Render> { Box::new(DynRender::single(self)) }
}

impl<D> IntoSingleParent for Stateful<DynWidget<Option<D>>>
where
  D: RenderSingleChild + WidgetBuilder + 'static,
{
  #[inline]
  fn into_single_parent(self) -> Box<dyn Render> { Box::new(DynRender::option(self)) }
}

impl WidgetChild for Widget {
  #[inline]
  fn child_build(self, ctx: &BuildCtx) -> WidgetId { self.build(ctx) }
}

impl<W: WidgetBuilder> WidgetChild for W {
  #[inline]
  fn child_build(self, ctx: &BuildCtx) -> WidgetId { self.build(ctx) }
}

impl<W, C> WidgetBuilder for WidgetPair<W, C>
where
  W: IntoSingleParent,
  C: WidgetChild,
{
  fn build(self, ctx: &BuildCtx) -> WidgetId {
    let Self { widget, child } = self;
    let p = ctx.alloc_widget(widget.into_single_parent());
    let child = child.child_build(ctx);
    ctx.append_child(p, child);
    p
  }
}

impl<W, C> WidgetBuilder for WidgetPair<Option<W>, C>
where
  W: IntoSingleParent,
  C: WidgetChild,
{
  fn build(self, ctx: &BuildCtx) -> WidgetId {
    let Self { widget, child } = self;
    if let Some(widget) = widget {
      WidgetPair { widget, child }.build(ctx)
    } else {
      child.child_build(ctx)
    }
  }
}

impl<W, C> WidgetBuilder for WidgetPair<W, Option<C>>
where
  W: IntoSingleParent,
  C: WidgetChild,
{
  fn build(self, ctx: &BuildCtx) -> WidgetId {
    let Self { widget, child } = self;
    if let Some(child) = child {
      WidgetPair { widget, child }.build(ctx)
    } else {
      let node = widget.into_single_parent();
      ctx.alloc_widget(node)
    }
  }
}

#[cfg(test)]
mod tests {
  use crate::test_helper::MockBox;

  use super::*;

  #[test]
  fn pair_with_child() {
    let mock_box = MockBox { size: ZERO_SIZE };
    let _ = FnWidget::new(|ctx| {
      mock_box
        .clone()
        .with_child(mock_box.clone(), ctx)
        .with_child(mock_box, ctx)
        .build(ctx)
    });
  }
}
