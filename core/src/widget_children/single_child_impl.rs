use super::*;
use crate::widget::WidgetBuilder;

/// Trait specify what child a widget can have, and the target type is the
/// result of widget compose its child.
pub trait SingleWithChild<C> {
  type Target;
  fn with_child(self, child: C, ctx: &BuildCtx) -> Self::Target;
}

/// A node of widget with not compose its child.
pub struct SinglePair<W, C> {
  pub widget: W,
  pub child: C,
}

impl<W: SingleChild> SingleChild for Option<W> {}

impl<W: SingleChild, C> SingleWithChild<C> for W {
  type Target = SinglePair<W, C>;

  #[inline]
  fn with_child(self, child: C, _: &BuildCtx) -> Self::Target { SinglePair { widget: self, child } }
}

impl<W, C1: SingleChild, C2> SingleWithChild<C2> for SinglePair<W, C1> {
  type Target = SinglePair<W, SinglePair<C1, C2>>;

  fn with_child(self, c: C2, ctx: &BuildCtx) -> Self::Target {
    let SinglePair { widget, child } = self;
    SinglePair {
      widget,
      child: child.with_child(c, ctx),
    }
  }
}

trait SingleParent {
  fn into_single_parent(self, ctx: &mut BuildCtx) -> WidgetId;
}

trait WidgetChild {
  fn child_build(self, ctx: &BuildCtx) -> WidgetId;
}

impl<W: RenderParent + SingleChild + 'static> SingleParent for W {
  #[inline]
  fn into_single_parent(self, ctx: &mut BuildCtx) -> WidgetId { self.into_render_parent(ctx) }
}

impl<W: RenderParent + SingleChild> SingleParent for Pipe<W> {
  #[inline]
  fn into_single_parent(self, ctx: &mut BuildCtx) -> WidgetId { self.into_only_parent(ctx) }
}

impl<D> SingleParent for Stateful<DynWidget<D>>
where
  D: Render + SingleChild + WidgetBuilder + 'static,
{
  #[inline]
  fn into_single_parent(self, ctx: &mut BuildCtx) -> WidgetId {
    Box::new(DynRender::single(self)).build(ctx)
  }
}

impl<D> SingleParent for Stateful<DynWidget<Option<D>>>
where
  D: Render + SingleChild + WidgetBuilder + 'static,
{
  #[inline]
  fn into_single_parent(self, ctx: &mut BuildCtx) -> WidgetId {
    Box::new(DynRender::option(self)).build(ctx)
  }
}

impl WidgetChild for Widget {
  #[inline]
  fn child_build(self, ctx: &BuildCtx) -> WidgetId { self.build(ctx) }
}

impl<W: WidgetBuilder> WidgetChild for W {
  #[inline]
  fn child_build(self, ctx: &BuildCtx) -> WidgetId { self.build(ctx) }
}

impl<W, C> WidgetBuilder for SinglePair<W, C>
where
  W: SingleParent,
  C: WidgetChild,
{
  fn build(self, ctx: &BuildCtx) -> WidgetId {
    let Self { widget, child } = self;
    let p = widget.into_single_parent(ctx.force_as_mut());
    let child = child.child_build(ctx);
    ctx.force_as_mut().append_child(p, child);
    p
  }
}

impl<W, C> WidgetBuilder for SinglePair<Option<W>, C>
where
  W: SingleParent,
  C: WidgetChild,
{
  fn build(self, ctx: &BuildCtx) -> WidgetId {
    let Self { widget, child } = self;
    if let Some(widget) = widget {
      SinglePair { widget, child }.build(ctx)
    } else {
      child.child_build(ctx)
    }
  }
}

impl<W, C> WidgetBuilder for SinglePair<W, Option<C>>
where
  W: SingleParent,
  C: WidgetChild,
{
  fn build(self, ctx: &BuildCtx) -> WidgetId {
    let Self { widget, child } = self;
    if let Some(child) = child {
      SinglePair { widget, child }.build(ctx)
    } else {
      widget.into_single_parent(ctx.force_as_mut())
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
