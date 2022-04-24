#[doc(hidden)]
pub use std::any::{Any, TypeId};
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
pub mod attr;
pub use attr::*;
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

use self::layout_store::BoxClamp;

pub trait Compose {
  /// Describes the part of the user interface represented by this widget.
  /// Called by framework, should never directly call it.
  fn compose(&self, ctx: &mut BuildCtx) -> BoxedWidget;
}

/// RenderWidget is a widget which want to paint something or do a layout to
/// calc itself size and update children positions.
///
/// Render Widget should at least implement one of `Layout` or `Paint`, if all
/// of `as_layout` and `as_paint` return None, the widget will not display.
///
/// If `as_layout` return none, widget size will detected by its single child if
/// it has or as large as possible.
pub trait RenderWidget {
  /// Do the work of computing the layout for this widget, and return the
  /// size it need.
  ///
  /// In implementing this function, You are responsible for calling every
  /// children's perform_layout across the `LayoutCtx`
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size;

  /// Whether the constraints from parent are the only input to detect the
  /// widget size, and child nodes' size not affect its size.
  fn only_sized_by_parent(&self) -> bool;
  /// `paint` is a low level trait to help you draw your widget to paint device
  /// across `PaintingCtx::painter` by itself coordinate system. Not care
  /// about children's paint in this method, framework will call children's
  /// paint individual. And framework guarantee always paint parent before
  /// children.
  fn paint(&self, ctx: &mut PaintingCtx);
}

// todo: deprecated, remove it after optimistic CombinationWidget.
/// A combination widget which want directly implement stateful widget and have
/// no stateless version. Implement `StatefulCombination` only when you need a
/// stateful widget during `build`, otherwise you should implement
/// [`CombinationWidget`]! and a stateful version will auto provide by
/// framework, use [`Stateful::into_stateful`]! to convert.
pub trait StatefulCombination {
  fn build(this: &Stateful<Self>, ctx: &mut BuildCtx) -> BoxedWidget
  where
    Self: Sized;
}

pub struct BoxedWidget(pub(crate) BoxedWidgetInner);

#[marker]
pub(crate) trait Widget {}
impl<W: Compose> Widget for W {}
impl<W: RenderWidget> Widget for W {}
impl<W: StatefulCombination> Widget for W {}

/// A trait to query dynamic type and its inner type on runtime, use this trait
/// to provide type information you want framework know.
pub(crate) trait QueryType {
  /// query self type by type id, and return a reference of `Any` trait to cast
  /// to target type if type match.
  fn query_any(&self, type_id: TypeId) -> Option<&dyn Any>;
  /// query self type by type id, and return a mut reference of `Any` trait to
  /// cast to target type if type match.
  fn query_any_mut(&mut self, type_id: TypeId) -> Option<&mut dyn Any>;
  /// A type can composed by others, this method query all type(include self)
  /// match the type id, and call the callback one by one. The callback accept
  /// an `& dyn Any` of the target type, and return if  want to continue.
  fn query_all_inner_any(&self, type_id: TypeId, callback: &dyn Fn(&dyn Any) -> bool);
  /// A type can composed by others, this method query all type(include self)
  /// match the type id, and call the callback one by one. The callback accept
  /// an `&mut dyn Any` of the target type, and return if want to continue.
  fn query_all_inner_any_mut(
    &mut self,
    type_id: TypeId,
    callback: &mut dyn FnMut(&mut dyn Any) -> bool,
  );
}
pub(crate) trait IntoRender {
  type R: RenderWidget;
  fn into_render(self) -> Self::R;
}

pub(crate) trait IntoCombination {
  type C: Compose;
  fn into_combination(self) -> Self::C;
}

impl<W: RenderWidget> IntoRender for W {
  type R = W;
  #[inline]
  fn into_render(self) -> Self::R { self }
}

impl<W: Compose> IntoCombination for W {
  type C = W;
  #[inline]
  fn into_combination(self) -> Self::C { self }
}

pub(crate) type BoxedSingleChild = Box<SingleChild<Box<dyn RenderNode>>>;
pub(crate) type BoxedMultiChild = MultiChild<Box<dyn RenderNode>>;
pub(crate) trait CombinationNode: Compose + AsAttrs + QueryType {}
pub(crate) trait RenderNode: RenderWidget + AsAttrs + QueryType {}

impl<W: Compose + AsAttrs + QueryType> CombinationNode for W {}

impl<W: RenderWidget + AsAttrs + QueryType> RenderNode for W {}

pub(crate) enum BoxedWidgetInner {
  Combination(Box<dyn CombinationNode>),
  Render(Box<dyn RenderNode>),
  SingleChild(BoxedSingleChild),
  MultiChild(BoxedMultiChild),
}

impl<W: Any> QueryType for W {
  #[inline]
  default fn query_any(&self, type_id: TypeId) -> Option<&dyn Any> {
    (self.type_id() == type_id).then(|| self as &dyn Any)
  }

  #[inline]
  default fn query_any_mut(&mut self, type_id: TypeId) -> Option<&mut (dyn Any + '_)> {
    ((&*self).type_id() == type_id).then(|| self as &mut dyn Any)
  }

  #[inline]
  default fn query_all_inner_any(&self, type_id: TypeId, callback: &dyn Fn(&dyn Any) -> bool) {
    if let Some(a) = self.query_any(type_id) {
      callback(a);
    }
  }

  #[inline]
  default fn query_all_inner_any_mut(
    &mut self,
    type_id: TypeId,
    callback: &mut dyn FnMut(&mut dyn Any) -> bool,
  ) {
    if let Some(a) = self.query_any_mut(type_id) {
      callback(a);
    }
  }
}

impl dyn QueryType {
  #[inline]
  pub fn query_type<T: Any>(&self) -> Option<&T> {
    self
      .query_any(TypeId::of::<T>())
      .and_then(|a| a.downcast_ref())
  }

  #[inline]
  pub fn query_type_mut<T: Any>(&mut self) -> Option<&mut T> {
    self
      .query_any_mut(TypeId::of::<T>())
      .and_then(|a| a.downcast_mut())
  }

  #[inline]
  pub fn query_all_inner_type<T: Any>(&self, callback: impl Fn(&T) -> bool) {
    let callback = |a: &dyn Any| a.downcast_ref().map_or(true, |t| callback(t));
    self.query_all_inner_any(TypeId::of::<T>(), &callback)
  }

  #[inline]
  fn query_all_inner_type_mut<T: Any>(&mut self, mut callback: impl FnMut(&mut T) -> bool) {
    let mut callback = |a: &mut dyn Any| a.downcast_mut().map_or(true, |t| callback(t));
    self.query_all_inner_any_mut(TypeId::of::<T>(), &mut callback)
  }
}
// Widget & BoxWidget default implementation
pub struct CombinationMarker;
pub struct StatefulCombinationMarker;
pub struct RenderMarker;

pub trait BoxWidget<Marker> {
  fn box_it(self) -> BoxedWidget;
}

impl<T: IntoCombination + 'static> BoxWidget<CombinationMarker> for T {
  #[inline]
  fn box_it(self) -> BoxedWidget {
    BoxedWidget(BoxedWidgetInner::Combination(Box::new(
      self.into_combination(),
    )))
  }
}

impl<T: IntoRender + 'static> BoxWidget<RenderMarker> for T {
  #[inline]
  fn box_it(self) -> BoxedWidget {
    BoxedWidget(BoxedWidgetInner::Render(Box::new(self.into_render())))
  }
}

impl<S: IntoRender + 'static> BoxWidget<RenderMarker> for SingleChild<S> {
  fn box_it(self) -> BoxedWidget {
    let widget: Box<dyn RenderNode> = Box::new(self.widget.into_render());
    let boxed = Box::new(SingleChild { widget, child: self.child });
    BoxedWidget(BoxedWidgetInner::SingleChild(boxed))
  }
}

impl<M: IntoRender + 'static> BoxWidget<RenderMarker> for MultiChild<M> {
  fn box_it(self) -> BoxedWidget {
    let widget: Box<dyn RenderNode> = Box::new(self.widget.into_render());
    let inner = BoxedWidgetInner::MultiChild(MultiChild { widget, children: self.children });
    BoxedWidget(inner)
  }
}

struct StatefulCombinationWrap<W>(Stateful<W>);

impl<W: StatefulCombination> Compose for StatefulCombinationWrap<W> {
  fn compose(&self, ctx: &mut BuildCtx) -> BoxedWidget
  where
    Self: Sized,
  {
    self.0.mark_during_build(true);
    let c = StatefulCombination::build(&self.0, ctx);
    self.0.mark_during_build(false);
    c
  }
}

impl<W> AsAttrs for StatefulCombinationWrap<W>
where
  Self: Widget,
{
  #[inline]
  fn as_attrs(&self) -> Option<&Attributes> { self.0.as_attrs() }

  #[inline]
  fn as_attrs_mut(&mut self) -> Option<&mut Attributes> { self.0.as_attrs_mut() }
}

impl<W: StatefulCombination + 'static> BoxWidget<StatefulCombinationMarker> for Stateful<W> {
  #[inline]
  fn box_it(self) -> BoxedWidget { StatefulCombinationWrap(self).box_it() }
}

impl<W: StatefulCombination + 'static> BoxWidget<StatefulCombinationMarker> for W {
  #[inline]
  fn box_it(self) -> BoxedWidget { StatefulCombinationWrap(self.into_stateful()).box_it() }
}

impl BoxWidget<StatefulCombinationMarker> for BoxedWidget {
  #[inline]
  fn box_it(self) -> BoxedWidget { self }
}

impl AsAttrs for BoxedWidget {
  fn as_attrs(&self) -> Option<&Attributes> {
    match &self.0 {
      BoxedWidgetInner::Combination(c) => c.as_attrs(),
      BoxedWidgetInner::Render(r) => r.as_attrs(),
      BoxedWidgetInner::SingleChild(s) => s.widget.as_attrs(),
      BoxedWidgetInner::MultiChild(m) => m.widget.as_attrs(),
    }
  }

  fn as_attrs_mut(&mut self) -> Option<&mut Attributes> {
    match &mut self.0 {
      BoxedWidgetInner::Combination(c) => c.as_attrs_mut(),
      BoxedWidgetInner::Render(r) => r.as_attrs_mut(),
      BoxedWidgetInner::SingleChild(s) => s.widget.as_attrs_mut(),
      BoxedWidgetInner::MultiChild(m) => m.widget.as_attrs_mut(),
    }
  }
}

impl<W: Fn(&mut BuildCtx) -> BoxedWidget> Compose for W {
  #[inline]
  fn compose(&self, ctx: &mut BuildCtx) -> BoxedWidget { self(ctx) }
}
