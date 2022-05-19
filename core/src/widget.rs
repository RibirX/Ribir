#[doc(hidden)]
pub use std::{
  any::{Any, TypeId},
  marker::PhantomData,
};
pub mod key;
pub mod layout;
pub use layout::*;
pub mod stateful;
pub mod text;
mod theme;
pub use theme::*;
pub(crate) mod widget_tree;
pub use crate::dynamic_widget::ExprWidget;
pub use crate::widget::text::Text;
pub use key::{Key, KeyWidget};
pub use stateful::*;
mod cursor;
pub use cursor::Cursor;
pub use winit::window::CursorIcon;
mod margin;
pub use margin::*;
mod padding;
pub use padding::*;
mod icon;
pub use icon::*;
mod svg;
pub use svg::*;
mod box_decoration;
pub use box_decoration::*;
mod checkbox;
pub use checkbox::*;
mod scrollable;
pub use scrollable::*;
mod path;
pub use path::*;
mod grid_view;
pub use grid_view::*;
// mod scroll_view;
// pub use scroll_view::ScrollView;
// mod scrollbar;

mod void;
use self::layout_store::BoxClamp;
pub use void::Void;

pub trait Compose {
  /// Describes the part of the user interface represented by this widget.
  /// Called by framework, should never directly call it.
  fn compose(this: Stateful<Self>, ctx: &mut BuildCtx) -> Widget
  where
    Self: Sized;
}

/// RenderWidget is a widget which want to paint something or do a layout to
/// calc itself size and update children positions.
///
/// Render Widget should at least implement one of `Layout` or `Paint`, if all
/// of `as_layout` and `as_paint` return None, the widget will not display.
///
/// If `as_layout` return none, widget size will detected by its single child if
/// it has or as large as possible.
pub trait Render {
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
}

impl<B> Render for ComposedWidget<Box<dyn RenderNode>, B> {
  #[inline]
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    self.composed.perform_layout(clamp, ctx)
  }

  #[inline]
  fn only_sized_by_parent(&self) -> bool { self.composed.only_sized_by_parent() }

  #[inline]
  fn paint(&self, ctx: &mut PaintingCtx) { self.composed.paint(ctx) }
}

impl<W: SingleChild, B> SingleChild for ComposedWidget<W, B> {}

impl<W: MultiChild, B> MultiChild for ComposedWidget<W, B> {}

pub struct Widget(pub(crate) WidgetInner);

#[marker]
pub(crate) trait WidgetMarker {}
impl<W: Compose> WidgetMarker for W {}
impl<W: ComposeSingleChild> WidgetMarker for W {}
impl<W: ComposeMultiChild> WidgetMarker for W {}
impl<W: Render> WidgetMarker for W {}

/// A trait to query dynamic type and its inner type on runtime, use this trait
/// to provide type information you want framework know.
pub(crate) trait QueryType {
  /// query self type by type id, and return a reference of `Any` trait to cast
  /// to target type if type match.
  fn query(&self, type_id: TypeId) -> Option<&dyn Any>;
  /// query self type by type id, and return a mut reference of `Any` trait to
  /// cast to target type if type match.
  fn query_mut(&mut self, type_id: TypeId) -> Option<&mut dyn Any>;
  /// A type can composed by others, this method query all type(include self)
  /// match the type id, and call the callback one by one. The callback accept
  /// an `& dyn Any` of the target type, and return if  want to continue.
  fn query_all<'a>(
    &'a self,
    type_id: TypeId,
    callback: &mut dyn FnMut(&'a dyn Any) -> bool,
    order: QueryOrder,
  );
  /// A type can composed by others, this method query all type(include self)
  /// match the type id, and call the callback one by one. The callback accept
  /// an `&mut dyn Any` of the target type, and return if want to continue.
  fn query_all_mut<'a>(
    &'a mut self,
    type_id: TypeId,
    callback: &mut dyn FnMut(&'a mut dyn Any) -> bool,
    order: QueryOrder,
  );
}

#[derive(Clone, Copy)]
pub(crate) enum QueryOrder {
  InnerFirst,
  OutsideFirst,
}
pub(crate) trait CombinationNode: Compose + QueryType {}
pub(crate) trait RenderNode: Render + QueryType {}

impl<W: Compose + QueryType> CombinationNode for W {}

impl<W: Render + QueryType> RenderNode for W {}

/// A generic widget wrap for all compose widget result, and keep its type info.
pub(crate) struct ComposedWidget<R, B> {
  composed: R,
  by: B,
}

impl<B: 'static> ComposedWidget<Widget, B> {
  fn into_widget(self) -> Widget {
    let by = self.by;
    match self.composed.0 {
      WidgetInner::Compose(c) => {
        { |ctx: &mut BuildCtx| ComposedWidget { composed: c(ctx), by }.into_widget() }.into_widget()
      }
      WidgetInner::Render(r) => ComposedWidget { composed: r, by }.into_widget(),
      WidgetInner::SingleChild(s) => {
        let widget: Box<dyn RenderNode> = Box::new(ComposedWidget { composed: s.widget, by });
        let single = Box::new(SingleChildWidget { widget, child: s.child });
        Widget(WidgetInner::SingleChild(single))
      }
      WidgetInner::MultiChild(m) => {
        let widget: Box<dyn RenderNode> = Box::new(ComposedWidget { composed: m.widget, by });
        let multi = MultiChildWidget { widget, children: m.children };
        Widget(WidgetInner::MultiChild(multi))
      }
      WidgetInner::Expr(_) => unreachable!(),
    }
  }
}

pub(crate) type BoxedSingleChild = Box<SingleChildWidget<Box<dyn RenderNode>>>;
pub(crate) type BoxedMultiChild = MultiChildWidget<Box<dyn RenderNode>>;

pub(crate) enum WidgetInner {
  Compose(Box<dyn FnOnce(&mut BuildCtx) -> Widget>),
  Render(Box<dyn RenderNode>),
  SingleChild(BoxedSingleChild),
  MultiChild(BoxedMultiChild),
  Expr(ExprWidget<Box<dyn FnMut() -> Box<dyn Iterator<Item = Widget>>>>),
}

impl<W: Any> QueryType for W {
  #[inline]
  default fn query(&self, type_id: TypeId) -> Option<&dyn Any> {
    (self.type_id() == type_id).then(|| self as &dyn Any)
  }

  #[inline]
  default fn query_mut(&mut self, type_id: TypeId) -> Option<&mut (dyn Any)> {
    ((&*self).type_id() == type_id).then(|| self as &mut dyn Any)
  }

  #[inline]
  default fn query_all<'a>(
    &'a self,
    type_id: TypeId,
    callback: &mut dyn FnMut(&'a dyn Any) -> bool,
    _: QueryOrder,
  ) {
    if let Some(a) = self.query(type_id) {
      callback(a);
    }
  }

  #[inline]
  default fn query_all_mut<'a>(
    &'a mut self,
    type_id: TypeId,
    callback: &mut dyn FnMut(&'a mut dyn Any) -> bool,
    _: QueryOrder,
  ) {
    if let Some(a) = self.query_mut(type_id) {
      callback(a);
    }
  }
}

impl<'a> dyn RenderNode + 'a {
  #[inline]
  pub fn query_all_type<'b, T: Any>(
    &'b self,
    mut callback: impl FnMut(&'b T) -> bool,
    order: QueryOrder,
  ) {
    let q = self as &dyn QueryType;
    q.query_all(
      TypeId::of::<T>(),
      &mut |a: &dyn Any| a.downcast_ref().map_or(true, |t| callback(t)),
      order,
    )
  }

  #[inline]
  pub fn query_all_type_mut<'b, T: Any>(
    &'b mut self,
    mut callback: impl FnMut(&'b mut T) -> bool,
    order: QueryOrder,
  ) {
    let q = self as &mut dyn QueryType;
    q.query_all_mut(
      TypeId::of::<T>(),
      &mut |a: &mut dyn Any| a.downcast_mut().map_or(true, |t| callback(t)),
      order,
    )
  }

  /// Query the first match type in all type by special order, and return a
  /// reference of it.
  pub fn query_first_type<T: Any>(&self, order: QueryOrder) -> Option<&T> {
    let mut target = None;
    self.query_all_type(
      |a| {
        target = Some(a);
        false
      },
      order,
    );
    target
  }

  /// Query the first match type in all type by special order. and return a mut
  /// reference of it.

  pub fn query_first_type_mut<T: Any>(&mut self, order: QueryOrder) -> Option<&mut T> {
    let mut target = None;
    self.query_all_type_mut(
      |a| {
        target = Some(a);
        false
      },
      order,
    );
    target
  }
}

pub trait IntoWidget<M: ?Sized> {
  fn into_widget(self) -> Widget;
}

impl IntoWidget<Widget> for Widget {
  #[inline]
  fn into_widget(self) -> Widget { self }
}

impl<C: Compose + 'static> IntoWidget<dyn Compose> for C {
  fn into_widget(self) -> Widget {
    Widget(WidgetInner::Compose(Box::new(|ctx| {
      ComposedWidget {
        composed: Compose::compose(self.into_stateful(), ctx),
        by: PhantomData::<C>,
      }
      .into_widget()
    })))
  }
}

impl<R: Render + 'static> IntoWidget<dyn Render> for R {
  #[inline]
  fn into_widget(self) -> Widget { Widget(WidgetInner::Render(Box::new(self))) }
}

impl<F: FnOnce(&mut BuildCtx) -> Widget + 'static> IntoWidget<F> for F {
  #[inline]
  fn into_widget(self) -> Widget { Widget(WidgetInner::Compose(Box::new(self))) }
}

#[macro_export]
macro_rules! impl_query_type {
  ($info: ident, $inner_widget: ident) => {
    fn query_all<'a>(
      &'a self,
      type_id: std::any::TypeId,
      callback: &mut dyn FnMut(&'a dyn Any) -> bool,
      order: QueryOrder,
    ) {
      let info = &self.$info;
      let widget = &self.$inner_widget;
      let mut continue_query = true;
      match order {
        QueryOrder::InnerFirst => {
          widget.query_all(
            type_id,
            &mut |t| {
              continue_query = callback(t);
              continue_query
            },
            order,
          );
          if continue_query {
            info.query_all(type_id, callback, order);
          }
        }
        QueryOrder::OutsideFirst => {
          info.query_all(type_id, callback, order);
          if continue_query {
            widget.query_all(
              type_id,
              &mut |t| {
                continue_query = callback(t);
                continue_query
              },
              order,
            );
          }
        }
      }
    }

    fn query_all_mut<'a>(
      &'a mut self,
      type_id: std::any::TypeId,
      callback: &mut dyn FnMut(&'a mut dyn Any) -> bool,
      order: QueryOrder,
    ) {
      let info = &mut self.$info;
      let widget = &mut self.$inner_widget;
      let mut continue_query = true;
      match order {
        QueryOrder::InnerFirst => {
          widget.query_all_mut(
            type_id,
            &mut |t| {
              continue_query = callback(t);
              continue_query
            },
            order,
          );
          if continue_query {
            info.query_all_mut(type_id, callback, order);
          }
        }
        QueryOrder::OutsideFirst => {
          info.query_all_mut(type_id, callback, order);
          if continue_query {
            widget.query_all_mut(
              type_id,
              &mut |t| {
                continue_query = callback(t);
                continue_query
              },
              order,
            );
          }
        }
      }
    }
  };
}
