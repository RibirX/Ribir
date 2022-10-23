pub(crate) use crate::{composed_widget::ComposedWidget, stateful::*, widget_tree::*};
use crate::{context::*, dynamic_widget::ExprWidget};
use algo::ShareResource;
use painter::*;

#[doc(hidden)]
pub use std::{
  any::{Any, TypeId},
  marker::PhantomData,
  ops::Deref,
};
use std::{cell::RefCell, rc::Rc};
pub trait Compose {
  /// Describes the part of the user interface represented by this widget.
  /// Called by framework, should never directly call it.
  fn compose(this: StateWidget<Self>) -> Widget
  where
    Self: Sized;
}

pub struct HitTest {
  pub(crate) hit: bool,
  pub(crate) can_hit_child: bool,
}

/// RenderWidget is a widget which want to paint something or do a layout to
/// calc itself size and update children positions.
///
/// Render Widget should at least implement one of `Layout` or `Paint`, if all
/// of `as_layout` and `as_paint` return None, the widget will not display.
///
/// If `as_layout` return none, widget size will detected by its single child if
/// it has or as large as possible.
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
  fn hit_test(&self, ctx: &TreeCtx, pos: Point) -> HitTest {
    let is_hit = hit_test_impl(ctx, pos);
    HitTest { hit: is_hit, can_hit_child: is_hit }
  }
}

pub(crate) fn hit_test_impl(ctx: &TreeCtx, pos: Point) -> bool {
  let id = ctx.id();
  ctx
    .widget_tree()
    .layout_box_rect(id)
    .map_or(false, |rect| rect.contains(pos))
}

/// Enum to store both stateless and stateful widget.
pub enum StateWidget<W> {
  Stateless(W),
  Stateful(Stateful<W>),
}

pub struct Widget {
  pub(crate) node: Option<WidgetNode>,
  pub(crate) children: Children,
}

pub(crate) enum WidgetNode {
  Compose(Box<dyn for<'r> FnOnce(&'r mut BuildCtx) -> Widget>),
  Render(Box<dyn Render>),
  Dynamic(ExprWidget<Box<dyn for<'r> FnMut(&'r mut BuildCtx) -> Vec<Widget>>>),
}

pub(crate) enum Children {
  None,
  Single(Box<Widget>),
  Multi(Vec<Widget>),
}

/// A trait to query dynamic type and its inner type on runtime, use this trait
/// to provide type information you want framework know.
pub trait Query {
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
    (self.type_id() == type_id).then(|| self as &dyn Any)
  }

  #[inline]
  fn query_filter_mut(&mut self, type_id: TypeId) -> Option<&mut dyn Any> {
    ((&*self).type_id() == type_id).then(|| self as &mut dyn Any)
  }
}

impl<'a> dyn Render + 'a {
  #[inline]
  pub fn query_all_type<T: Any>(&self, mut callback: impl FnMut(&T) -> bool, order: QueryOrder) {
    self.query_all(
      TypeId::of::<T>(),
      &mut |a: &dyn Any| a.downcast_ref().map_or(true, |t| callback(t)),
      order,
    )
  }

  /// Query the first match type in all type by special order, and call
  /// `callback`
  pub fn query_on_first_type<T: Any>(&self, order: QueryOrder, callback: impl FnOnce(&T)) {
    let mut callback = Some(callback);
    self.query_all_type(
      move |a| {
        let cb = callback.take().expect("should only call once");
        cb(a);
        false
      },
      order,
    );
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
}

pub trait IntoWidget<M: ?Sized> {
  fn into_widget(self) -> Widget;
}

impl Widget {
  #[inline]
  pub fn into_widget(self) -> Widget { self }
}

impl<C: Compose + Into<StateWidget<C>> + 'static> IntoWidget<dyn Compose> for C {
  #[inline]
  fn into_widget(self) -> Widget {
    ComposedWidget::<Widget, C>::new(Compose::compose(self.into())).into_widget()
  }
}

impl<R: Render + 'static> IntoWidget<dyn Render> for R {
  #[inline]
  fn into_widget(self) -> Widget {
    Widget {
      node: Some(WidgetNode::Render(Box::new(self))),
      children: Children::None,
    }
  }
}

impl<F: FnOnce(&mut BuildCtx) -> Widget + 'static> IntoWidget<F> for F {
  #[inline]
  fn into_widget(self) -> Widget {
    Widget {
      node: Some(WidgetNode::Compose(Box::new(self))),
      children: Children::None,
    }
  }
}

#[macro_export]
macro_rules! impl_proxy_query {
  ($($field: tt)*) => {
    #[inline]
    fn query_all(
      &self,
      type_id: TypeId,
      callback: &mut dyn FnMut(&dyn Any) -> bool,
      order: QueryOrder,
    ) {
      self.$($field)*.query_all(type_id, callback, order)
    }
  };
}

#[macro_export]
macro_rules! impl_query_self_only {
  () => {
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
  };
}

impl<T: Render> Render for algo::ShareResource<T> {
  #[inline]
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    T::perform_layout(self, clamp, ctx)
  }

  #[inline]
  fn paint(&self, ctx: &mut PaintingCtx) { T::paint(self, ctx) }

  fn only_sized_by_parent(&self) -> bool { T::only_sized_by_parent(self) }
}

impl<T: Query> Query for ShareResource<T> {
  fn query_all(
    &self,
    type_id: TypeId,
    callback: &mut dyn FnMut(&dyn Any) -> bool,
    order: QueryOrder,
  ) {
    (&**self).query_all(type_id, callback, order)
  }
}

impl<W> From<W> for StateWidget<W> {
  #[inline]
  fn from(w: W) -> Self { StateWidget::Stateless(w) }
}

impl<W> From<Stateful<W>> for StateWidget<W> {
  #[inline]
  fn from(w: Stateful<W>) -> Self { StateWidget::Stateful(w) }
}

impl<W: IntoStateful> StateWidget<W> {
  pub fn into_stateful(self) -> Stateful<W> {
    match self {
      StateWidget::Stateless(w) => w.into_stateful(),
      StateWidget::Stateful(w) => w,
    }
  }
}

impl Children {
  pub(crate) fn is_none(&self) -> bool { matches!(self, Children::None) }

  pub(crate) fn for_each(self, mut cb: impl FnMut(Widget)) {
    match self {
      Children::None => {}
      Children::Single(w) => cb(*w),
      Children::Multi(m) => m.into_iter().for_each(cb),
    }
  }

  pub(crate) fn len(&self) -> usize {
    match self {
      Children::None => 0,
      Children::Single(_) => 1,
      Children::Multi(m) => m.len(),
    }
  }
}

#[macro_export]
macro_rules! impl_proxy_render {
  ($($proxy: tt)*) => {
      #[inline]
      fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
        self.$($proxy)*.perform_layout(clamp, ctx)
      }

      #[inline]
      fn paint(&self, ctx: &mut PaintingCtx) { self.$($proxy)*.paint(ctx) }

      #[inline]
      fn only_sized_by_parent(&self) -> bool { self.$($proxy)*.only_sized_by_parent() }
  };
}

impl<W: Render> Render for RefCell<W> {
  impl_proxy_render!(borrow());
}

impl<W: Query> Query for RefCell<W> {
  impl_proxy_query!(borrow());
}

impl<W: Render + 'static> Render for Rc<W> {
  impl_proxy_render!(deref());
}

impl<W: Query + 'static> Query for Rc<W> {
  fn query_all(
    &self,
    type_id: TypeId,
    callback: &mut dyn FnMut(&dyn Any) -> bool,
    order: QueryOrder,
  ) {
    let mut query_more = true;
    match order {
      QueryOrder::InnerFirst => {
        self.deref().query_all(
          type_id,
          &mut |any| {
            query_more = callback(any);
            query_more
          },
          order,
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
          self.deref().query_all(type_id, callback, order);
        }
      }
    }
  }
}

impl Render for Box<dyn Render> {
  impl_proxy_render!(deref());
}

impl Query for Box<dyn Render> {
  impl_proxy_query!(deref());
}
