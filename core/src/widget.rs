use std::convert::Infallible;
#[doc(hidden)]
pub use std::{
  any::{Any, TypeId},
  marker::PhantomData,
  ops::Deref,
};

use rxrust::ops::box_it::CloneableBoxOp;
use widget_id::RenderQueryable;

pub(crate) use crate::widget_tree::*;
use crate::{context::*, prelude::*, render_helper::PureRender};
pub trait Compose: Sized {
  /// Describes the part of the user interface represented by this widget.
  /// Called by framework, should never directly call it.
  fn compose(this: impl StateWriter<Value = Self>) -> impl IntoWidgetStrict<FN>;
}

pub struct HitTest {
  pub hit: bool,
  pub can_hit_child: bool,
}

/// RenderWidget is a widget which want to paint something or do a layout to
/// calc itself size and update children positions.
pub trait Render: 'static {
  /// Do the work of computing the layout for this widget, and return the
  /// size it need.
  ///
  /// In implementing this function, You are responsible for calling every
  /// children's perform_layout across the `LayoutCtx`
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size;

  /// `paint` is a low level trait to help you draw your widget to paint device
  /// across `PaintingCtx::painter` by itself coordinate system. Not care
  /// about children's paint in this method, framework will call children's
  /// paint individual. And framework guarantee always paint parent before
  /// children.
  fn paint(&self, ctx: &mut PaintingCtx);

  /// Whether the constraints from parent are the only input to detect the
  /// widget size, and child nodes' size not affect its size.
  fn only_sized_by_parent(&self) -> bool { false }

  /// Determines the set of render widgets located at the given position.
  fn hit_test(&self, ctx: &HitTestCtx, pos: Point) -> HitTest {
    let is_hit = hit_test_impl(ctx, pos);
    HitTest { hit: is_hit, can_hit_child: is_hit }
  }

  fn get_transform(&self) -> Option<Transform> { None }
}

/// The common type of all widget can convert to.
pub struct Widget {
  id: WidgetId,
  handle: BuildCtxHandle,
}

/// A boxed function widget that can be called multiple times to regenerate
/// widget.
pub struct GenWidget(Box<dyn for<'a, 'b> FnMut(&'a BuildCtx<'b>) -> Widget>);

// The widget type marker.
pub const COMPOSE: usize = 1;
pub const RENDER: usize = 2;
pub const FN: usize = 3;

/// Defines a trait for converting any widget into a `Widget` type. Direct
/// implementation of this trait is not recommended as it is automatically
/// implemented by the framework.
///
/// Instead, focus on implementing `Compose`, `Render`, or `ComposeChild`.
pub trait IntoWidget<const M: usize> {
  fn into_widget(self, ctx: &BuildCtx) -> Widget;
}

/// A trait used by the framework to implement `IntoWidget`. Unlike
/// `IntoWidget`, this trait is not implemented for `Widget` itself. This design
/// choice allows the framework to use either `IntoWidget` or `IntoWidgetStrict`
/// as a generic bound, preventing implementation conflicts.
// fixme: should be pub(crate)
pub trait IntoWidgetStrict<const M: usize> {
  fn into_widget_strict(self, ctx: &BuildCtx) -> Widget;
}

impl IntoWidget<FN> for Widget {
  #[inline(always)]
  fn into_widget(self, _: &BuildCtx) -> Widget { self }
}

impl<const M: usize, T: IntoWidgetStrict<M>> IntoWidget<M> for T {
  #[inline(always)]
  fn into_widget(self, ctx: &BuildCtx) -> Widget { self.into_widget_strict(ctx) }
}

impl<C: Compose + 'static> IntoWidgetStrict<COMPOSE> for C {
  #[inline]
  fn into_widget_strict(self, ctx: &BuildCtx) -> Widget {
    Compose::compose(State::value(self)).into_widget(ctx)
  }
}

impl<R: Render + 'static> IntoWidgetStrict<RENDER> for R {
  #[inline]
  fn into_widget_strict(self, ctx: &BuildCtx) -> Widget {
    Widget::new(Box::new(PureRender(self)), ctx)
  }
}

impl<W: ComposeChild<Child = Option<C>>, C> Compose for W {
  fn compose(this: impl StateWriter<Value = Self>) -> impl IntoWidgetStrict<FN> {
    fn_widget! {
      ComposeChild::compose_child(this, None)
    }
  }
}

impl<F> IntoWidgetStrict<FN> for F
where
  F: FnOnce(&BuildCtx) -> Widget,
{
  #[inline]
  fn into_widget_strict(self, ctx: &BuildCtx) -> Widget { self(ctx) }
}

impl IntoWidgetStrict<FN> for GenWidget {
  #[inline]
  fn into_widget_strict(mut self, ctx: &BuildCtx) -> Widget { self.gen_widget(ctx) }
}

impl Widget {
  /// Consume the widget, and return its id. This means this widget already be
  /// append into its parent.
  pub(crate) fn consume(self) -> WidgetId {
    let id = self.id;
    std::mem::forget(self);
    id
  }

  /// Subscribe the modifies `upstream` to mark the widget dirty when the
  /// `upstream` emit a modify event that contains `ModifyScope::FRAMEWORK`.
  pub(crate) fn dirty_subscribe(
    self, upstream: CloneableBoxOp<'static, ModifyScope, Infallible>, ctx: &BuildCtx,
  ) -> Self {
    let dirty_set = ctx.tree.borrow().dirty_set.clone();
    let id = self.id();
    let h = upstream
      .filter(|b| b.contains(ModifyScope::FRAMEWORK))
      .subscribe(move |_| {
        dirty_set.borrow_mut().insert(id);
      })
      .unsubscribe_when_dropped();

    self.attach_anonymous_data(h, ctx)
  }

  pub(crate) fn id(&self) -> WidgetId { self.id }

  pub(crate) fn new(w: Box<dyn RenderQueryable>, ctx: &BuildCtx) -> Self {
    Self::from_id(ctx.alloc_widget(w), ctx)
  }

  pub(crate) fn from_id(id: WidgetId, ctx: &BuildCtx) -> Self { Self { id, handle: ctx.handle() } }
}

impl GenWidget {
  #[inline]
  pub fn new(f: impl FnMut(&BuildCtx) -> Widget + 'static) -> Self { Self(Box::new(f)) }

  #[inline]
  pub fn gen_widget(&mut self, ctx: &BuildCtx) -> Widget { (self.0)(ctx) }
}

impl<F: FnMut(&BuildCtx) -> Widget + 'static> From<F> for GenWidget {
  #[inline]
  fn from(f: F) -> Self { Self::new(f) }
}

pub(crate) fn hit_test_impl(ctx: &HitTestCtx, pos: Point) -> bool {
  ctx
    .box_rect()
    .map_or(false, |rect| rect.contains(pos))
}

impl Drop for Widget {
  fn drop(&mut self) {
    log::warn!("widget allocated but never used: {:?}", self.id);
    self
      .handle
      .with_ctx(|ctx| ctx.tree.borrow_mut().remove_subtree(self.id));
  }
}
