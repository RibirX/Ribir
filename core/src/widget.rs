pub(crate) use crate::widget_tree::*;
use crate::{
  context::{build_ctx::BuildCtxHandle, *},
  prelude::*,
};
use ribir_algo::{Sc, ShareResource};

#[doc(hidden)]
pub use std::{
  any::{Any, TypeId},
  marker::PhantomData,
  ops::Deref,
};
use std::{convert::Infallible, rc::Rc};
pub trait Compose: Sized {
  /// Describes the part of the user interface represented by this widget.
  /// Called by framework, should never directly call it.
  fn compose(this: State<Self>) -> impl WidgetBuilder;
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
pub struct Widget {
  id: WidgetId,
  handle: BuildCtxHandle,
}

/// A type can composed by many types, this trait help us to query the type and
/// the inner type by its type id, and call the callback one by one with a `&
/// dyn Any` of the target type. You can control if you want to continue query
/// by return `true` or `false` in the callback.
pub trait Query: Any {
  /// Query the type in a inside first order, and apply the callback to it,
  fn query_inside_first(&self, type_id: TypeId, callback: &mut dyn FnMut(&dyn Any) -> bool);
  /// Query the type in a outside first order, and apply the callback to it,
  fn query_outside_first(&self, type_id: TypeId, callback: &mut dyn FnMut(&dyn Any) -> bool);
}

impl<'a> dyn Render + 'a {
  #[inline]
  pub fn query_type_inside_first<T: Any>(&self, mut callback: impl FnMut(&T) -> bool) {
    self.query_inside_first(TypeId::of::<T>(), &mut |a| {
      a.downcast_ref().map_or(true, &mut callback)
    })
  }

  #[inline]
  pub fn query_type_outside_first<T: Any>(&self, mut callback: impl FnMut(&T) -> bool) {
    Query::query_outside_first(self, TypeId::of::<T>(), &mut |a| {
      a.downcast_ref().map_or(true, &mut callback)
    })
  }

  /// Query the most inside type match `T`, and apply the callback to it, return
  /// what the callback return.
  pub fn query_most_inside<T: Any, R>(&self, callback: impl FnOnce(&T) -> R) -> Option<R> {
    let mut callback = Some(callback);
    let mut res = None;
    self.query_type_inside_first(|a| {
      let cb = callback.take().expect("should only call once");
      res = Some(cb(a));
      false
    });
    res
  }

  /// Query the most outside type match `T`, and apply the callback to it,
  /// return what the callback return.
  pub fn query_most_outside<T: Any, R>(&self, callback: impl FnOnce(&T) -> R) -> Option<R> {
    let mut callback = Some(callback);
    let mut res = None;
    self.query_type_outside_first(|a| {
      let cb = callback.take().expect("should only call once");
      res = Some(cb(a));
      false
    });
    res
  }

  /// return if this object contain type `T`
  pub fn contain_type<T: Any>(&self) -> bool {
    let mut hit = false;
    self.query_type_outside_first(|_: &T| {
      hit = true;
      false
    });
    hit
  }

  /// return if this object is type `T`
  pub fn is<T: Any>(&self) -> bool { self.type_id() == TypeId::of::<T>() }
}

/// Trait to build a indirect widget into widget tree with `BuildCtx` in the
/// build phase. You should not implement this trait directly, framework will
/// auto implement this.
///
/// A indirect widget is a widget that is not `Compose`, `Render` and
/// `ComposeChild`,  like function widget and  `Pipe<Widget>`.
pub trait WidgetBuilder {
  fn widget_build(self, ctx: &BuildCtx) -> Widget;
}

/// Trait to build a compose widget into widget tree with `BuildCtx` in the
/// build phase. You should not implement this trait directly, implement
/// `Compose` trait instead.
pub trait ComposeBuilder {
  fn widget_build(self, ctx: &BuildCtx) -> Widget;
}

/// Trait to build a render widget into widget tree with `BuildCtx` in the build
/// phase. You should not implement this trait directly, implement `Render`
/// trait instead.
pub trait RenderBuilder {
  fn widget_build(self, ctx: &BuildCtx) -> Widget;
}

/// Trait to build a `ComposeChild` widget without child into widget tree with
/// `BuildCtx` in the build phase, only work if the child of `ComposeChild` is
/// `Option<>_`  . You should not implement this trait directly,
/// implement `ComposeChild` trait instead.
pub trait ComposeChildBuilder {
  fn widget_build(self, ctx: &BuildCtx) -> Widget;
}

/// Trait only for `Widget`, you should not implement this trait.
pub trait SelfBuilder {
  fn widget_build(self, ctx: &BuildCtx) -> Widget;
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
    self,
    upstream: Subject<'static, ModifyScope, Infallible>,
    ctx: &BuildCtx,
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

  pub(crate) fn new(w: Box<dyn Render>, ctx: &BuildCtx) -> Self {
    Self::from_id(ctx.alloc_widget(w), ctx)
  }

  pub(crate) fn from_id(id: WidgetId, ctx: &BuildCtx) -> Self { Self { id, handle: ctx.handle() } }
}

impl SelfBuilder for Widget {
  #[inline(always)]
  fn widget_build(self, _: &BuildCtx) -> Widget { self }
}

impl<F> WidgetBuilder for F
where
  F: FnOnce(&BuildCtx) -> Widget,
{
  #[inline]
  fn widget_build(self, ctx: &BuildCtx) -> Widget { self(ctx) }
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

      fn query_inside_first(&self, type_id: TypeId, callback: &mut dyn FnMut(&dyn Any) -> bool) {
        let mut query_more = true;
        impl_proxy_query!(reverse
          [$(
            if query_more {
              self.$($name $(($($args),*))?).*
                .query_inside_first(
                  type_id,
                  &mut |any| {
                    query_more = callback(any);
                    query_more
                  },
                );
            }
          ),+]
        );
        if type_id == self.type_id() {
          callback(self);
        }
      }
      fn query_outside_first(&self, type_id: TypeId, callback: &mut dyn FnMut(&dyn Any) -> bool) {
        if type_id == self.type_id() {
          callback(self);
        }
        let mut query_more = true;
        if query_more {
          $(
            if query_more {
              self.$($name $(($($args),*))?).*
                .query_outside_first(
                  type_id,
                  &mut |any| {
                    query_more = callback(any);
                    query_more
                  },
                );
            }
          )+
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
      fn query_inside_first(&self, type_id: TypeId, callback: &mut dyn FnMut(&dyn Any) -> bool) {
        self.query_outside_first(type_id, callback)
      }

      #[inline]
      fn query_outside_first(&self, type_id: TypeId, callback: &mut dyn FnMut(&dyn Any) -> bool) {
        if type_id == self.type_id() {
          callback(self);
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

impl<C: Compose> ComposeBuilder for C {
  #[inline]
  fn widget_build(self, ctx: &BuildCtx) -> Widget {
    Compose::compose(State::value(self)).widget_build(ctx)
  }
}
impl<R: Render + 'static> RenderBuilder for R {
  #[inline]
  fn widget_build(self, ctx: &BuildCtx) -> Widget { Widget::new(Box::new(self), ctx) }
}

impl<W: ComposeChild<Child = Option<C>>, C> ComposeChildBuilder for W {
  #[inline]
  fn widget_build(self, ctx: &BuildCtx) -> Widget {
    ComposeChild::compose_child(State::value(self), None).widget_build(ctx)
  }
}

impl_proxy_query!(paths [deref()], ShareResource<T>, <T>, where  T: Render + 'static);
impl_proxy_render!(proxy deref(), ShareResource<T>, <T>, where  T: Render + 'static);
impl_proxy_query!(paths [deref()], Rc<W>, <W>, where W: Query + 'static);
impl_proxy_render!(proxy deref(), Rc<W>, <W>, where W: Render + 'static);
impl_proxy_query!(paths [deref()], Sc<W>, <W>, where W: Query + 'static);

pub(crate) fn hit_test_impl(ctx: &HitTestCtx, pos: Point) -> bool {
  ctx.box_rect().map_or(false, |rect| rect.contains(pos))
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
