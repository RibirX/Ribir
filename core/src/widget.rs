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
pub use crate::widget::text::Text;
pub use key::Key;
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
mod scroll_view;
pub use scroll_view::ScrollView;
mod scrollbar;

mod empty;
use self::layout_store::BoxClamp;
pub use empty::Empty;

pub trait Compose {
  // todo: use associated type replace BoxedWidget is friendly?
  /// Describes the part of the user interface represented by this widget.
  /// Called by framework, should never directly call it.
  fn compose(self, ctx: &mut BuildCtx) -> BoxedWidget;
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

/// A compose widget which want directly implement stateful widget and have no
/// stateless version. Implement `StatefulCombination` only when you need a
/// stateful widget during `build`, otherwise you should implement
/// [`Compose`]! and a stateful version will auto provide by
/// framework, use [`Stateful::into_stateful`]! to convert.
pub trait StatefulCompose {
  fn compose(this: Stateful<Self>, ctx: &mut BuildCtx) -> BoxedWidget
  where
    Self: Sized;
}

/// A generic widget wrap for all compose widget result, and keep its type info.
struct ComposedWidget<R, B> {
  composed: R,
  by: B,
}

pub(crate) trait RecursiveCompose {
  fn recursive_compose(self: Box<Self>, ctx: &mut BuildCtx) -> BoxedWidget;
}

impl<C: Compose + 'static> RecursiveCompose for C {
  fn recursive_compose(self: Box<Self>, ctx: &mut BuildCtx) -> BoxedWidget {
    // todo: we need wrap the build context let logic child can query type of ?
    // or keep theme as a individual widget is enough.

    ComposedWidget {
      composed: self.compose(ctx),
      by: PhantomData::<C>,
    }
    .into_boxed(ctx)
  }
}

impl<B: 'static> ComposedWidget<BoxedWidget, B> {
  fn into_boxed(self, ctx: &mut BuildCtx) -> BoxedWidget {
    let by = self.by;
    match self.composed.0 {
      BoxedWidgetInner::Compose(c) => ComposedWidget {
        composed: c.recursive_compose(ctx),
        by,
      }
      .into_boxed(ctx),
      BoxedWidgetInner::Render(r) => ComposedWidget { composed: r, by }.box_it(),
      BoxedWidgetInner::SingleChild(s) => SingleChild {
        widget: ComposedWidget { composed: s.widget, by },
        child: s.child,
      }
      .box_it(),
      BoxedWidgetInner::MultiChild(m) => MultiChild {
        widget: ComposedWidget { composed: m.widget, by },
        children: m.children,
      }
      .box_it(),
    }
  }
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
pub struct BoxedWidget(pub(crate) BoxedWidgetInner);

#[marker]
pub(crate) trait Widget {}
impl<W: Compose> Widget for W {}
impl<W: Render> Widget for W {}

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

pub(crate) type BoxedSingleChild = Box<SingleChild<Box<dyn RenderNode>>>;
pub(crate) type BoxedMultiChild = MultiChild<Box<dyn RenderNode>>;
pub(crate) trait CombinationNode: Compose + QueryType {}
pub(crate) trait RenderNode: Render + QueryType {}

impl<W: Compose + QueryType> CombinationNode for W {}

impl<W: Render + QueryType> RenderNode for W {}

pub(crate) enum BoxedWidgetInner {
  Compose(Box<dyn RecursiveCompose>),
  Render(Box<dyn RenderNode>),
  SingleChild(BoxedSingleChild),
  MultiChild(BoxedMultiChild),
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

// todo: does we can directly  extend the sub tree in compose method, and remove
// box widget?

pub struct RenderMarker;
pub struct ComposeMarker;
pub trait BoxWidget<M> {
  fn box_it(self) -> BoxedWidget;
}

impl<C: Compose + 'static> BoxWidget<ComposeMarker> for C {
  #[inline]
  default fn box_it(self) -> BoxedWidget { BoxedWidget(BoxedWidgetInner::Compose(Box::new(self))) }
}

impl<R: Render + 'static> BoxWidget<RenderMarker> for R {
  #[inline]
  fn box_it(self) -> BoxedWidget { BoxedWidget(BoxedWidgetInner::Render(Box::new(self))) }
}

impl<S: Render + 'static> BoxWidget<RenderMarker> for SingleChild<S> {
  #[inline]
  fn box_it(self) -> BoxedWidget {
    let widget: Box<dyn RenderNode> = Box::new(self.widget);
    let boxed = Box::new(SingleChild { widget, child: self.child });
    BoxedWidget(BoxedWidgetInner::SingleChild(boxed))
  }
}

impl<M: Render + 'static> BoxWidget<RenderMarker> for MultiChild<M> {
  #[inline]
  fn box_it(self) -> BoxedWidget {
    let widget: Box<dyn RenderNode> = Box::new(self.widget);
    let inner = BoxedWidgetInner::MultiChild(MultiChild { widget, children: self.children });
    BoxedWidget(inner)
  }
}

impl BoxWidget<()> for BoxedWidget {
  #[inline]
  fn box_it(self) -> BoxedWidget { self }
}

impl<C: FnOnce(&mut BuildCtx) -> BoxedWidget> Compose for C {
  #[inline]
  fn compose(self, ctx: &mut BuildCtx) -> BoxedWidget { self(ctx) }
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
