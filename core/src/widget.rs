#[doc(hidden)]
pub use std::{
  any::{Any, TypeId},
  marker::PhantomData,
  ops::Deref,
};

use widget_id::{new_node, RenderQueryable};

pub(crate) use crate::widget_tree::*;
use crate::{context::*, prelude::*, render_helper::PureRender};
pub trait Compose: Sized {
  /// Describes the part of the user interface represented by this widget.
  /// Called by framework, should never directly call it.
  fn compose(this: impl StateWriter<Value = Self>) -> Widget<'static>;
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
pub struct Widget<'w>(InnerWidget<'w>);
pub(crate) enum InnerWidget<'w> {
  Leaf(Box<dyn RenderQueryable>),
  Lazy(LazyWidget<'w>),
  LazyBuild(Box<dyn FnOnce(&BuildCtx) -> WidgetId + 'w>),
  SubTree { node: Box<Widget<'w>>, children: Vec<Widget<'w>> },
}
/// This serves as a wrapper for `Box<dyn FnOnce(&BuildCtx) -> Widget<'w> +
/// 'w>`, but does not utilize the 'w in the return type to prevent the
/// `LazyWidget` from becoming **invariant**. This approach allows `Widget<'w>`
/// to remain **covariant** with the lifetime `'w`.

/// This approach should be acceptable since `LazyWidget` is private and not
/// accessed externally. Additionally, the lifetime will shorten once we consume
/// it to obtain the `Widget<'w>`.
pub(crate) struct LazyWidget<'w>(Box<dyn FnOnce(&BuildCtx) -> Widget<'static> + 'w>);

impl<'w> LazyWidget<'w> {
  pub(crate) fn new(f: impl FnOnce(&BuildCtx) -> Widget<'w> + 'w) -> Self {
    let f: Box<dyn FnOnce(&BuildCtx) -> Widget<'w> + 'w> = Box::new(f);
    // Safety: the lifetime will shorten once we consume it to obtain the
    // `Widget<'w>`.
    let f: Box<dyn FnOnce(&BuildCtx) -> Widget<'static> + 'w> = unsafe { std::mem::transmute(f) };
    Self(f)
  }

  fn consume(self, ctx: &BuildCtx) -> Widget<'w> { (self.0)(ctx) }
}

/// A boxed function widget that can be called multiple times to regenerate
/// widget.
pub struct GenWidget(Box<dyn FnMut(&BuildCtx) -> Widget<'static>>);

// The widget type marker.
pub const COMPOSE: usize = 1;
pub const RENDER: usize = 2;
pub const FN: usize = 3;

/// Defines a trait for converting any widget into a `Widget` type. Direct
/// implementation of this trait is not recommended as it is automatically
/// implemented by the framework.
///
/// Instead, focus on implementing `Compose`, `Render`, or `ComposeChild`.
pub trait IntoWidget<'w, const M: usize>: 'w {
  fn into_widget(self) -> Widget<'w>;
}

/// A trait used by the framework to implement `IntoWidget`. Unlike
/// `IntoWidget`, this trait is not implemented for `Widget` itself. This design
/// choice allows the framework to use either `IntoWidget` or `IntoWidgetStrict`
/// as a generic bound, preventing implementation conflicts.
pub(crate) trait IntoWidgetStrict<'w, const M: usize>: 'w {
  fn into_widget_strict(self) -> Widget<'w>;
}

impl<'w> IntoWidget<'w, FN> for Widget<'w> {
  #[inline(always)]
  fn into_widget(self) -> Widget<'w> { self }
}

impl<'w, const M: usize, T: IntoWidgetStrict<'w, M>> IntoWidget<'w, M> for T {
  #[inline(always)]
  fn into_widget(self) -> Widget<'w> { self.into_widget_strict() }
}

impl<C: Compose + 'static> IntoWidgetStrict<'static, COMPOSE> for C {
  #[inline]
  fn into_widget_strict(self) -> Widget<'static> {
    Compose::compose(State::value(self)).into_widget()
  }
}

impl<R: Render + 'static> IntoWidgetStrict<'static, RENDER> for R {
  fn into_widget_strict(self) -> Widget<'static> {
    InnerWidget::Leaf(Box::new(PureRender(self))).into()
  }
}

impl<W: ComposeChild<'static, Child = Option<C>>, C> Compose for W {
  fn compose(this: impl StateWriter<Value = Self>) -> Widget<'static> {
    ComposeChild::compose_child(this, None)
  }
}

impl<'w, F> IntoWidgetStrict<'w, FN> for F
where
  F: FnOnce(&BuildCtx) -> Widget<'w> + 'w,
{
  fn into_widget_strict(self) -> Widget<'w> {
    let lazy = LazyWidget::new(self);
    InnerWidget::Lazy(lazy).into()
  }
}

impl IntoWidgetStrict<'static, FN> for GenWidget {
  #[inline]
  fn into_widget_strict(self) -> Widget<'static> { self.0.into_widget_strict() }
}

impl<'a> Widget<'a> {
  pub(crate) fn build(self, ctx: &BuildCtx) -> WidgetId {
    // fixme: restore the state of `&BuildCtx`, wait for provider.
    let mut subtrees = vec![];
    let root = self.inner_build(&mut subtrees, ctx);
    while let Some((p, child)) = subtrees.pop() {
      let c = child.inner_build(&mut subtrees, ctx);
      p.append(c, &mut ctx.tree.borrow_mut());
    }
    root
  }

  fn inner_build(self, subtrees: &mut Vec<(WidgetId, Widget<'a>)>, ctx: &BuildCtx) -> WidgetId {
    match self.0 {
      InnerWidget::Leaf(r) => new_node(&mut ctx.tree.borrow_mut().arena, r),
      InnerWidget::LazyBuild(f) => f(ctx),
      InnerWidget::Lazy(l) => l.consume(ctx).inner_build(subtrees, ctx),
      InnerWidget::SubTree { node, children } => {
        let p = node.inner_build(subtrees, ctx);
        let leaf = p.single_leaf(&ctx.tree.borrow());
        for c in children.into_iter().rev() {
          subtrees.push((leaf, c))
        }
        p
      }
    }
  }
}
impl GenWidget {
  #[inline]
  pub fn new(f: impl FnMut(&BuildCtx) -> Widget<'static> + 'static) -> Self { Self(Box::new(f)) }

  #[inline]
  pub fn gen_widget(&mut self, ctx: &BuildCtx) -> Widget<'static> { (self.0)(ctx) }
}

impl<F: FnMut(&BuildCtx) -> Widget<'static> + 'static> From<F> for GenWidget {
  #[inline]
  fn from(f: F) -> Self { Self::new(f) }
}

pub(crate) fn hit_test_impl(ctx: &HitTestCtx, pos: Point) -> bool {
  ctx
    .box_rect()
    .map_or(false, |rect| rect.contains(pos))
}

impl<'w> From<InnerWidget<'w>> for Widget<'w> {
  fn from(value: InnerWidget<'w>) -> Self { Widget(value) }
}
