#[doc(hidden)]
pub use std::{
  any::{Any, TypeId},
  marker::PhantomData,
  ops::Deref,
};
use std::{cell::RefCell, convert::Infallible};

use ribir_algo::Sc;
use rxrust::ops::box_it::CloneableBoxOp;

pub(crate) use crate::widget_tree::*;
use crate::{context::*, prelude::*};
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

/// A boxed function widget.
pub type BoxedWidget = Box<dyn for<'a, 'b> FnOnce(&'a BuildCtx<'b>) -> Widget>;

/// A boxed function widget that can be called multiple times to regenerate
/// widget.
pub struct GenWidget(Box<dyn for<'a, 'b> FnMut(&'a BuildCtx<'b>) -> Widget>);

/// A type can composed by many types, this trait help us to query the type and
/// the inner type by its type id, and call the callback one by one with a `&
/// dyn Any` of the target type. You can control if you want to continue query
/// by return `true` or `false` in the callback.
pub trait Query: Any {
  /// Query the type in a inside first order, and apply the callback to it.
  /// return what the callback return, hint if the query should continue.
  fn query_inside_first(&self, type_id: TypeId, callback: &mut dyn FnMut(&dyn Any) -> bool)
  -> bool;
  /// Query the type in a outside first order, and apply the callback to it,
  /// return what the callback return, hint if the query should continue.
  fn query_outside_first(
    &self, type_id: TypeId, callback: &mut dyn FnMut(&dyn Any) -> bool,
  ) -> bool;
}

impl<'a> dyn Render + 'a {
  #[inline]
  pub fn query_type_inside_first<T: Any>(&self, mut callback: impl FnMut(&T) -> bool) -> bool {
    self
      .query_inside_first(TypeId::of::<T>(), &mut |a| a.downcast_ref().map_or(true, &mut callback))
  }

  #[inline]
  pub fn query_type_outside_first<T: Any>(&self, mut callback: impl FnMut(&T) -> bool) -> bool {
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
  fn build(self, ctx: &BuildCtx) -> Widget;

  /// Convert the widget to named type widget `FnWidget`, this is useful when
  /// you want store a widget and not want to call `build(ctx!())` to
  /// build it into the widget tree.
  ///
  /// # Example
  ///
  /// ```ignore
  /// let w = if xxx {
  ///   fn_widget! { ... }.box_it()
  /// else {
  ///   fn_widget! { ... }.box_it()
  /// };
  /// ```
  fn box_it(self) -> BoxedWidget
  where
    Self: Sized + 'static,
  {
    Box::new(move |ctx| self.build(ctx))
  }
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

  pub(crate) fn new(w: Box<dyn Render>, ctx: &BuildCtx) -> Self {
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

/// only query the inner object, not query self.
macro_rules! impl_proxy_query {
  ($($t:tt)*) => {
    #[inline]
    fn query_inside_first(
      &self,
      type_id: TypeId,
      callback: &mut dyn FnMut(&dyn Any) -> bool,
    ) -> bool {
      self.$($t)*.query_inside_first(type_id, callback)
    }

    #[inline]
    fn query_outside_first(
      &self,
      type_id: TypeId,
      callback: &mut dyn FnMut(&dyn Any) -> bool,
    ) -> bool {
      self.$($t)*.query_outside_first(type_id, callback)
    }
  }
}

/// query self and proxy to the inner object.
macro_rules! impl_proxy_and_self_query {
  ($($t:tt)*) => {
    fn query_inside_first(
      &self,
      type_id: TypeId,
      callback: &mut dyn FnMut(&dyn Any) -> bool,
    ) -> bool {
      if !self.$($t)*.query_inside_first(type_id, callback) {
        return false
      }

      type_id != self.type_id() || callback(self)
    }

    fn query_outside_first(
      &self,
      type_id: TypeId,
      callback: &mut dyn FnMut(&dyn Any) -> bool,
    ) -> bool {
      if type_id == self.type_id() && !callback(self) {
        return false
      }
      self.$($t)*.query_outside_first(type_id, callback)
    }
  }
}

/// query self only.
macro_rules! impl_query_self_only {
  () => {
    #[inline]
    fn query_inside_first(
      &self, type_id: TypeId, callback: &mut dyn FnMut(&dyn Any) -> bool,
    ) -> bool {
      self.query_outside_first(type_id, callback)
    }

    #[inline]
    fn query_outside_first(
      &self, type_id: TypeId, callback: &mut dyn FnMut(&dyn Any) -> bool,
    ) -> bool {
      if type_id == self.type_id() { callback(self) } else { true }
    }
  };
}
pub(crate) use impl_proxy_and_self_query;
pub(crate) use impl_proxy_query;
pub(crate) use impl_query_self_only;

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

impl<C: Compose + 'static> ComposeBuilder for C {
  #[inline]
  fn build(self, ctx: &BuildCtx) -> Widget { Compose::compose(State::value(self)).build(ctx) }
}

impl<R: Render + 'static> RenderBuilder for R {
  #[inline]
  fn build(self, ctx: &BuildCtx) -> Widget { Widget::new(Box::new(self), ctx) }
}

impl<W: ComposeChild<Child = Option<C>> + 'static, C> ComposeChildBuilder for W {
  #[inline]
  fn build(self, ctx: &BuildCtx) -> Widget {
    ComposeChild::compose_child(State::value(self), None).build(ctx)
  }
}

impl<T: Query> Query for Resource<T> {
  impl_proxy_and_self_query!(deref());
}
impl<T: Query> Query for Sc<T> {
  impl_proxy_and_self_query!(deref());
}
impl<T: Query> Query for RefCell<T> {
  impl_proxy_and_self_query!(borrow());
}

impl<T: Query> Query for StateCell<T> {
  impl_proxy_and_self_query!(read());
}

impl_proxy_render!(proxy deref(), Resource<T>, <T>, where  T: Render + 'static);

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

use self::state_cell::StateCell;

impl Drop for Widget {
  fn drop(&mut self) {
    log::warn!("widget allocated but never used: {:?}", self.id);
    self
      .handle
      .with_ctx(|ctx| ctx.tree.borrow_mut().remove_subtree(self.id));
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  macro_rules! impl_wrap_test {
    ($name:ident) => {
      paste::paste! {
        #[test]
        fn [<$name:lower _support_query>]() {
          let warp = $name::new(Void);
          let void_tid = Void.type_id();
          let w_tid = warp.type_id();
          let mut hit = 0;

          let mut hit_fn = |_: &dyn Any| {
            hit += 1;
            true
          };

          warp.query_inside_first(void_tid, &mut hit_fn);
          warp.query_outside_first(void_tid, &mut hit_fn);
          warp.query_inside_first(w_tid, &mut hit_fn);
          warp.query_outside_first(w_tid, &mut hit_fn);
          assert_eq!(hit, 4);
        }
      }
    };
  }
  impl_wrap_test!(Sc);
  impl_wrap_test!(RefCell);
  impl_wrap_test!(Resource);
}
