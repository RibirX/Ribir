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
  fn compose(this: impl StateWriter<Value = Self>) -> impl WidgetBuilder;
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
pub const COMPOSE_CHILD: usize = 3;
pub const FN: usize = 4;

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
    Compose::compose(State::value(self)).build(ctx)
  }
}

impl<R: Render + 'static> IntoWidgetStrict<RENDER> for R {
  #[inline]
  fn into_widget_strict(self, ctx: &BuildCtx) -> Widget {
    Widget::new(Box::new(PureRender(self)), ctx)
  }
}

impl<W: ComposeChild<Child = Option<C>> + 'static, C> IntoWidgetStrict<COMPOSE_CHILD> for W {
  #[inline]
  fn into_widget_strict(self, ctx: &BuildCtx) -> Widget {
    ComposeChild::compose_child(State::value(self), None).build(ctx)
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

/// Trait to build a indirect widget into widget tree with `BuildCtx` in the
/// build phase. You should not implement this trait directly, framework will
/// auto implement this.
///
/// A indirect widget is a widget that is not `Compose`, `Render` and
/// `ComposeChild`,  like function widget and  `Pipe<Widget>`.
pub trait WidgetBuilder {
  fn build(self, ctx: &BuildCtx) -> Widget;
}

/// Trait to build a compose widget into widget tree with `BuildCtx` in the
/// build phase. You should not implement this trait directly, implement
/// `Compose` trait instead.
pub trait ComposeBuilder {
  fn build(self, ctx: &BuildCtx) -> Widget;
}

/// Trait to build a render widget into widget tree with `BuildCtx` in the build
/// phase. You should not implement this trait directly, implement `Render`
/// trait instead.
pub trait RenderBuilder {
  fn build(self, ctx: &BuildCtx) -> Widget;
}

/// Trait to build a `ComposeChild` widget without child into widget tree with
/// `BuildCtx` in the build phase, only work if the child of `ComposeChild` is
/// `Option<>_`  . You should not implement this trait directly,
/// implement `ComposeChild` trait instead.
pub trait ComposeChildBuilder {
  fn build(self, ctx: &BuildCtx) -> Widget;
}

/// Trait only for `Widget`, you should not implement this trait.
pub trait SelfBuilder {
  fn build(self, ctx: &BuildCtx) -> Widget;
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

impl SelfBuilder for Widget {
  #[inline(always)]
  fn build(self, _: &BuildCtx) -> Widget { self }
}

impl<F> WidgetBuilder for F
where
  F: FnOnce(&BuildCtx) -> Widget,
{
  #[inline]
  fn build(self, ctx: &BuildCtx) -> Widget { self(ctx) }
}

impl WidgetBuilder for GenWidget {
  #[inline]
  fn build(mut self, ctx: &BuildCtx) -> Widget { self.gen_widget(ctx) }
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

impl<C: Compose + 'static> ComposeBuilder for C {
  #[inline]
  fn build(self, ctx: &BuildCtx) -> Widget { Compose::compose(State::value(self)).build(ctx) }
}

impl<R: Render + 'static> RenderBuilder for R {
  fn build(self, ctx: &BuildCtx) -> Widget { Widget::new(Box::new(PureRender(self)), ctx) }
}

impl<W: ComposeChild<Child = Option<C>> + 'static, C> ComposeChildBuilder for W {
  #[inline]
  fn build(self, ctx: &BuildCtx) -> Widget {
    ComposeChild::compose_child(State::value(self), None).build(ctx)
  }
}

pub(crate) fn hit_test_impl(ctx: &HitTestCtx, pos: Point) -> bool {
  ctx
    .box_rect()
    .map_or(false, |rect| rect.contains(pos))
}

macro_rules! _replace {
  (@replace($n: path) [$($e:tt)*] {#} $($rest:tt)*) => {
    $crate::widget::_replace!(@replace($n) [$($e)* $n] $($rest)*);
  };
  (@replace($n: path) [$($e:tt)*] $first: tt $($rest:tt)*) => {
    $crate::widget::_replace!(@replace($n) [$($e)* $first] $($rest)*);
  };
  (@replace($i: path) [$($e:tt)*]) => { $($e)* };
  (@replace($n: path) $first: tt $($rest:tt)*) => {
    $crate::widget::_replace!(@replace($n) [$first] $($rest)*);
  };
}

macro_rules! multi_build_replace_impl {
  ($($rest:tt)*) => {
    $crate::widget::repeat_and_replace!([
      $crate::widget::ComposeBuilder,
      $crate::widget::RenderBuilder,
      $crate::widget::ComposeChildBuilder,
      $crate::widget::WidgetBuilder
    ] $($rest)*);
  };
}

macro_rules! multi_build_replace_impl_include_self {
  ($($rest:tt)*) => {
    $crate::widget::multi_build_replace_impl!($($rest)*);
    $crate::widget::_replace!(@replace($crate::widget::SelfBuilder) $($rest)*);
  };
  ({} $($rest:tt)*) => {}
}

macro_rules! repeat_and_replace {
  ([$first: path $(,$n: path)*] $($rest:tt)*) => {
    $crate::widget::_replace!(@replace($first) $($rest)*);
    $crate::widget::repeat_and_replace!([$($n),*] $($rest)*);
  };
  ([] $($rest:tt)*) => {
  };
}

pub(crate) use _replace;
pub(crate) use multi_build_replace_impl;
pub(crate) use multi_build_replace_impl_include_self;
pub(crate) use repeat_and_replace;

impl Drop for Widget {
  fn drop(&mut self) {
    log::warn!("widget allocated but never used: {:?}", self.id);
    self
      .handle
      .with_ctx(|ctx| ctx.tree.borrow_mut().remove_subtree(self.id));
  }
}
