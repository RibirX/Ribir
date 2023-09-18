pub(crate) use crate::widget_tree::*;
use crate::{context::*, prelude::*};
use ribir_algo::{Sc, ShareResource};
use rxrust::subscription::{BoxSubscription, SubscriptionGuard};

use std::rc::Rc;
#[doc(hidden)]
pub use std::{
  any::{Any, TypeId},
  marker::PhantomData,
  ops::Deref,
};
pub trait Compose: Sized {
  /// Describes the part of the user interface represented by this widget.
  /// Called by framework, should never directly call it.
  fn compose(this: State<Self>) -> Widget;
}

pub struct HitTest {
  pub hit: bool,
  pub can_hit_child: bool,
}

/// RenderWidget is a widget which want to paint something or do a layout to
/// calc itself size and update children positions.
pub trait Render: Query {
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
pub struct Widget(Box<dyn FnOnce(&BuildCtx) -> WidgetId>);

/// A trait to query dynamic type and its inner type on runtime, use this trait
/// to provide type information you want framework know.
pub trait Query: QueryFiler {
  /// A type can composed by others, this method query all type(include self)
  /// match the type id, and call the callback one by one. The callback accept
  /// an `& dyn Any` of the target type, and return if it want to continue.
  fn query_all(
    &self,
    type_id: TypeId,
    callback: &mut dyn FnMut(&dyn Any) -> bool,
    order: QueryOrder,
  );
}

#[derive(Clone, Copy)]
pub enum QueryOrder {
  InnerFirst,
  OutsideFirst,
}

/// Trait to detect if a type is match the `type_id`.
pub trait QueryFiler {
  /// query self type by type id, and return a reference of `Any` trait to cast
  /// to target type if type match.
  fn query_filter(&self, type_id: TypeId) -> Option<&dyn Any>;
  /// query self type by type id, and return a mut reference of `Any` trait to
  /// cast to target type if type match.
  fn query_filter_mut(&mut self, type_id: TypeId) -> Option<&mut dyn Any>;
}

impl<W: 'static> QueryFiler for W {
  #[inline]
  fn query_filter(&self, type_id: TypeId) -> Option<&dyn Any> {
    (self.type_id() == type_id).then_some(self as &dyn Any)
  }

  #[inline]
  fn query_filter_mut(&mut self, type_id: TypeId) -> Option<&mut dyn Any> {
    ((*self).type_id() == type_id).then_some(self as &mut dyn Any)
  }
}

impl<'a> dyn Render + 'a {
  #[inline]
  pub fn query_all_type<T: Any>(&self, mut callback: impl FnMut(&T) -> bool, order: QueryOrder) {
    self.query_all(
      TypeId::of::<T>(),
      &mut |a| a.downcast_ref().map_or(true, &mut callback),
      order,
    )
  }

  /// Query the first match type in all type by special order, and call
  /// `callback`
  pub fn query_on_first_type<T: Any, R>(
    &self,
    order: QueryOrder,
    callback: impl FnOnce(&T) -> R,
  ) -> Option<R> {
    let mut callback = Some(callback);
    let mut res = None;
    self.query_all_type(
      |a| {
        let cb = callback.take().expect("should only call once");
        res = Some(cb(a));
        false
      },
      order,
    );
    res
  }

  pub fn contain_type<T: Any>(&self) -> bool {
    let mut hit = false;
    self.query_all_type(
      |_: &T| {
        hit = true;
        false
      },
      QueryOrder::OutsideFirst,
    );
    hit
  }

  pub fn is<T: Any>(&self) -> bool { self.query_filter(TypeId::of::<T>()).is_some() }
}

pub struct FnWidget<F>(F);

/// Trait to build a type to a widget, `StrictBuilder` and `WidgetBuilder` is
/// only for type distinction and help us to build complex widget.
///
///
/// These type implement this trait:
///
/// - type implemented `Compose` trait
/// - type implemented `Render` trait
/// - `ComposePair<_,_>`
/// - `SinglePair<_, _>`
/// - `MultiPair<_, _>`
/// - `Pipe<W>` if `W` implement `LooseBuilder`
pub(crate) trait StrictBuilder {
  fn strict_build(self, ctx: &BuildCtx) -> WidgetId;
}

/// Trait to build a type to a widget and allow to create new widget in build
/// phase.
///
/// - Types implemented `StrictBuilder` will auto implement `WidgetBuilder`
/// - `Pipe<Option<W>>` directly implement this trait.
pub trait WidgetBuilder {
  fn build(self, ctx: &BuildCtx) -> WidgetId;
}

impl<T: Into<Widget>> WidgetBuilder for T {
  #[inline]
  fn build(self, ctx: &BuildCtx) -> WidgetId { self.into().build(ctx) }
}

impl StrictBuilder for WidgetId {
  fn strict_build(self, _: &BuildCtx) -> WidgetId { self }
}

impl<F> FnWidget<F> {
  #[inline]
  pub fn new<R>(f: F) -> Self
  where
    F: FnOnce(&BuildCtx) -> R,
  {
    FnWidget(f)
  }

  #[inline]
  pub fn into_inner(self) -> F { self.0 }
}

impl<F, R> StrictBuilder for FnWidget<F>
where
  F: FnOnce(&BuildCtx) -> R + 'static,
  R: WidgetBuilder,
{
  fn strict_build(self, ctx: &BuildCtx) -> WidgetId { (self.0)(ctx).build(ctx) }
}

#[macro_export]
macro_rules! impl_proxy_query {
  (reverse [$first: expr $(, $rest: expr)*] $($reversed: expr)*) => {
    impl_proxy_query!(reverse [$($rest),*] $first $($reversed)*);
  };
  (reverse [] $($reversed: expr)*) => { $($reversed)* };
  (
    paths [
      $(
        $($name: tt $(($($args: ident),*))?).*
      ),+
    ],
    $ty: ty $(, <$($($lf: lifetime)? $($p: ident)?), *>)? $(,where $($w:tt)*)?
  ) => {
    impl $(<$($($lf)? $($p)?),*>)? Query for $ty $(where $($w)*)? {
      #[inline]
      fn query_all(
        &self,
        type_id: TypeId,
        callback: &mut dyn FnMut(&dyn Any) -> bool,
        order: QueryOrder,
      ) {
        let mut query_more = true;
        match order {
          QueryOrder::InnerFirst => {
            impl_proxy_query!(reverse
              [$(
                if query_more {
                  self.$($name $(($($args),*))?).*
                    .query_all(
                      type_id,
                      &mut |any| {
                        query_more = callback(any);
                        query_more
                      },
                      order,
                    );
                }
              ),+]
            );
            if let Some(a) = self.query_filter(type_id) {
              callback(a);
            }
          }
          QueryOrder::OutsideFirst => {
            if let Some(a) = self.query_filter(type_id) {
              query_more = callback(a);
            }
            if query_more {
              $(
                if query_more {
                  self.$($name $(($($args),*))?).*
                    .query_all(
                      type_id,
                      &mut |any| {
                        query_more = callback(any);
                        query_more
                      },
                      order,
                    );
                }
              )+
            }
          }
        }
      }
    }
  };
}

#[macro_export]
macro_rules! impl_query_self_only {
  ($name: ty $(, <$($($lf: lifetime)? $($p: ident)?), *>)? $(,where $($w:tt)*)?) => {
    impl $(<$($($lf)? $($p)?),*>)? Query for $name $(where $($w)*)? {
      #[inline]
      fn query_all(
        &self,
        type_id: TypeId,
        callback: &mut dyn FnMut(&dyn Any) -> bool,
        _: QueryOrder,
      ) {
        if let Some(a) = self.query_filter(type_id) {
          callback(a);
        }
      }
    }
  };
}

#[macro_export]
macro_rules! impl_proxy_render {
  (
    proxy $($mem: tt $(($($args: ident),*))?).*,
    $name: ty $(,<$($($lf: lifetime)? $($p: ident)?), *>)?
    $(,where $($w:tt)*)?
  ) => {
    impl $(<$($($lf)? $($p)?),*>)? Render for $name $(where $($w)*)? {
      #[inline]
      fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
        self.$($mem $(($($args),*))?).*.perform_layout(clamp, ctx)
      }

      #[inline]
      fn paint(&self, ctx: &mut PaintingCtx) {
        self.$($mem $(($($args),*))?).*.paint(ctx)
      }

      #[inline]
      fn only_sized_by_parent(&self) -> bool {
        self.$($mem $(($($args),*))?).*.only_sized_by_parent()
      }

      #[inline]
      fn hit_test(&self, ctx: &HitTestCtx, pos: Point) -> HitTest {
        self.$($mem $(($($args),*))?).*.hit_test(ctx, pos)
      }

      #[inline]
      fn get_transform(&self) -> Option<Transform> {
        self.$($mem $(($($args),*))?).*.get_transform()
      }
    }
  };
}

impl<C: Compose> StrictBuilder for C {
  #[inline]
  fn strict_build(self, ctx: &BuildCtx) -> WidgetId { State::value(self).strict_build(ctx) }
}

impl<C: Compose> StrictBuilder for Stateful<C> {
  #[inline]
  fn strict_build(self, ctx: &BuildCtx) -> WidgetId { State::stateful(self).strict_build(ctx) }
}

impl<R: Render + 'static> Compose for R {
  fn compose(this: State<Self>) -> Widget {
    FnWidget::new(move |ctx| ctx.alloc_widget(this.into())).into()
  }
}

impl<W: StrictBuilder + 'static> From<W> for Widget {
  #[inline]
  fn from(value: W) -> Self { Self(Box::new(|ctx| value.strict_build(ctx))) }
}

impl<F, R> From<F> for FnWidget<F>
where
  F: FnOnce(&BuildCtx) -> R,
  R: Into<Widget>,
{
  #[inline]
  fn from(value: F) -> Self { Self(value) }
}

impl Widget {
  #[inline]
  pub fn build(self, ctx: &BuildCtx) -> WidgetId { (self.0)(ctx) }
}

impl<R: Render + 'static> From<R> for Box<dyn Render> {
  #[inline]
  fn from(value: R) -> Self { Box::new(value) }
}

impl<R: Render + 'static> From<Stateful<R>> for Box<dyn Render> {
  #[inline]
  fn from(value: Stateful<R>) -> Self { Box::new(RenderFul(value)) }
}

impl_proxy_query!(paths [deref()], ShareResource<T>, <T>, where  T: Render + 'static);
impl_proxy_render!(proxy deref(), ShareResource<T>, <T>, where  T: Render + 'static);
impl_proxy_query!(paths [deref()], Rc<W>, <W>, where W: Query + 'static);
impl_proxy_render!(proxy deref(), Rc<W>, <W>, where W: Render + 'static);
impl_proxy_query!(paths [deref()], Sc<W>, <W>, where W: Query + 'static);
impl_proxy_render!(proxy deref(), Sc<W>, <W>, where W: Render + 'static);

impl_query_self_only!(Vec<SubscriptionGuard<BoxSubscription<'static>>>);

/// Directly return `v`, this function does nothing, but it's useful to help you
/// declare a widget expression in `widget!` macro.
#[inline]
pub const fn from<W>(v: W) -> W { v }

/// Return OptionWidget::Some widget if the `b` is true, the return value wrap
/// from the return value of `f` method called.
#[inline]
pub fn then<W>(b: bool, f: impl FnOnce() -> W) -> Option<W> { b.then(f) }

/// calls the closure on `value` and returns
#[inline]
pub fn map<T, W>(value: T, f: impl FnOnce(T) -> W) -> W { f(value) }

pub(crate) fn hit_test_impl(ctx: &HitTestCtx, pos: Point) -> bool {
  ctx.box_rect().map_or(false, |rect| rect.contains(pos))
}
