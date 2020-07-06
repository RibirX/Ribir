use crate::render::*;
use std::{
  any::{Any, TypeId},
  fmt::Debug,
};
pub mod build_ctx;
pub mod key;
pub mod layout;
mod stateful;
pub mod text;
pub mod widget_tree;
pub mod window;
pub use build_ctx::BuildCtx;
pub use key::{Key, KeyDetect};
pub use layout::row_col_layout::RowColumn;
pub use stateful::{Stateful, StatefulRef};
pub use text::Text;
pub mod events;
use events::pointers::{PointerEvent, PointerEventType, PointerListener};
pub use events::Event;

/// The common behavior for widgets, also support to downcast to special widget.
pub trait Widget: Debug + Any {
  /// classify this widget into one of four type widget, and return the
  /// reference.
  fn classify(&self) -> WidgetClassify;

  /// classify this widget into one of four type widget as mutation reference.
  fn classify_mut(&mut self) -> WidgetClassifyMut;

  /// return the some-value of `InheritWidget` reference if the widget is
  /// inherit from another widget, otherwise None.
  #[inline]
  fn as_inherit(&self) -> Option<&dyn InheritWidget> { None }

  /// like `as_inherit`, but return mutable reference.
  #[inline]
  fn as_inherit_mut(&mut self) -> Option<&mut dyn InheritWidget> { None }

  /// Convert a stateless widget to stateful, use it get a cell ref to modify
  /// the widget.
  fn into_stateful(self, ctx: &BuildCtx) -> Stateful<Self>
  where
    Self: Sized,
  {
    Stateful::new(ctx.tree.clone(), self)
  }

  /// Assign a key to the widget to help framework to track if two widget is a
  /// same widget in two frame.
  #[inline]
  fn with_key<K: Into<Key>>(self, key: K) -> KeyDetect
  where
    Self: Sized,
  {
    KeyDetect::new(key, self.box_it())
  }

  #[inline]
  fn box_it(self) -> BoxWidget
  where
    Self: Sized,
  {
    BoxWidget {
      widget: Box::new(self),
    }
  }

  #[inline]
  fn on_pointer_down<F>(self, handler: F) -> BoxWidget
  where
    Self: Sized,
    F: Fn(&PointerEvent) + 'static,
  {
    listen_pointer_event(self.box_it(), PointerEventType::Down, handler)
  }

  #[inline]
  fn on_pointer_up<F>(self, handler: F) -> BoxWidget
  where
    Self: Sized,
    F: Fn(&PointerEvent) + 'static,
  {
    listen_pointer_event(self.box_it(), PointerEventType::Up, handler)
  }

  #[inline]
  fn on_pointer_move<F>(self, handler: F) -> BoxWidget
  where
    Self: Sized,
    F: Fn(&PointerEvent) + 'static,
  {
    listen_pointer_event(self.box_it(), PointerEventType::Move, handler)
  }

  #[inline]
  fn on_pointer_cancel<F>(self, handler: F) -> BoxWidget
  where
    Self: Sized,
    F: Fn(&PointerEvent) + 'static,
  {
    listen_pointer_event(self.box_it(), PointerEventType::Cancel, handler)
  }
}

/// A widget represented by other widget compose.
pub trait CombinationWidget: Debug {
  /// Describes the part of the user interface represented by this widget.
  fn build(&self) -> BoxWidget;
}

/// a widget has a child.
pub trait SingleChildWidget: RenderWidgetSafety {
  /// called by framework to take child from this widget, and only called once.
  fn take_child(&mut self) -> BoxWidget;
}

/// a widget has multi child
pub trait MultiChildWidget: RenderWidgetSafety {
  /// called by framework to take children from this widget, and only called
  /// once.
  fn take_children(&mut self) -> Vec<BoxWidget>;
}

pub enum WidgetClassify<'a> {
  Combination(&'a dyn CombinationWidget),
  Render(&'a dyn RenderWidgetSafety),
  SingleChild(&'a dyn SingleChildWidget),
  MultiChild(&'a dyn MultiChildWidget),
}

pub enum WidgetClassifyMut<'a> {
  Combination(&'a mut dyn CombinationWidget),
  Render(&'a mut dyn RenderWidgetSafety),
  SingleChild(&'a mut dyn SingleChildWidget),
  MultiChild(&'a mut dyn MultiChildWidget),
}

impl<'a> WidgetClassify<'a> {
  #[inline]
  pub fn is_combination(&self) -> bool { matches!(self, WidgetClassify::Combination(_)) }

  #[inline]
  pub fn is_render(&self) -> bool { !matches!(self, WidgetClassify::Combination(_)) }

  #[inline]
  pub fn is_single_child(&self) -> bool { matches!(self, WidgetClassify::SingleChild(_)) }

  #[inline]
  pub fn is_multi_child(&self) -> bool { matches!(self, WidgetClassify::MultiChild(_)) }
}

impl<'a> WidgetClassifyMut<'a> {
  #[inline]
  pub fn is_combination(&self) -> bool { matches!(self, WidgetClassifyMut::Combination(_)) }

  #[inline]
  pub fn is_render(&self) -> bool { !matches!(self, WidgetClassifyMut::Combination(_)) }

  #[inline]
  pub fn is_single_child(&self) -> bool { matches!(self, WidgetClassifyMut::SingleChild(_)) }

  #[inline]
  pub fn is_multi_child(&self) -> bool { matches!(self, WidgetClassifyMut::MultiChild(_)) }
}

/// Use inherit method to implement a `Widget`, this is use to extend ability of
/// a widget but not increase the widget number. Notice it's difference to class
/// inherit, it's instance inherit.
pub trait InheritWidget: Widget {
  fn base_widget(&self) -> &dyn Widget;
  fn base_widget_mut(&mut self) -> &mut dyn Widget;
}

pub struct BoxWidget {
  widget: Box<dyn Widget>,
}

impl std::fmt::Debug for BoxWidget {
  #[inline]
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { self.widget.fmt(f) }
}

inherit_widget!(BoxWidget, widget);

impl dyn Widget {
  pub fn downcast_mut<T: 'static>(&mut self) -> Option<&mut T> {
    if (&*self).type_id() == TypeId::of::<T>() {
      let ptr = self as *mut dyn Widget as *mut T;
      // SAFETY: just checked whether we are pointing to the correct type, and we can
      // rely on that check for memory safety because we have implemented Any for
      // all types; no other impls can exist as they would conflict with our impl.
      unsafe { Some(&mut *ptr) }
    } else {
      self
        .as_inherit_mut()
        .and_then(|inherit| Widget::downcast_mut(inherit.base_widget_mut()))
    }
  }

  pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
    if self.type_id() == TypeId::of::<T>() {
      let ptr = self as *const dyn Widget as *const T;
      // SAFETY: just checked whether we are pointing to the correct type, and we can
      // rely on that check for memory safety because we have implemented Any for
      // all types; no other impls can exist as they would conflict with our impl.
      unsafe { Some(&*ptr) }
    } else {
      self
        .as_inherit()
        .and_then(|inherit| inherit.base_widget().downcast_ref())
    }
  }
}

use std::borrow::{Borrow, BorrowMut};

pub macro inherit_widget($ty: ty, $base_widget: ident) {
  impl InheritWidget for $ty {
    #[inline]
    fn base_widget(&self) -> &dyn Widget { self.$base_widget.borrow() }
    #[inline]
    fn base_widget_mut(&mut self) -> &mut dyn Widget { self.$base_widget.borrow_mut() }
  }

  impl Widget for $ty {
    #[inline]
    fn classify(&self) -> WidgetClassify { self.base_widget().classify() }

    #[inline]
    fn classify_mut(&mut self) -> WidgetClassifyMut { self.base_widget_mut().classify_mut() }

    #[inline]
    fn as_inherit(&self) -> Option<&dyn InheritWidget> { Some(self) }

    #[inline]
    fn as_inherit_mut(&mut self) -> Option<&mut dyn InheritWidget> { Some(self) }
  }
}

/// We should also implement Widget for RenderWidgetSafety, SingleChildWidget
/// and MultiChildWidget, but can not do it before rust specialization finished.
/// So just CombinationWidget implemented it, this is user use most, and others
/// provide a macro to do it.
impl<T: CombinationWidget + 'static> Widget for T {
  #[inline]
  fn classify(&self) -> WidgetClassify { WidgetClassify::Combination(self) }

  #[inline]
  fn classify_mut(&mut self) -> WidgetClassifyMut { WidgetClassifyMut::Combination(self) }
}

impl<T: CombinationWidget> !RenderWidget for T {}
impl<T: RenderWidget> !CombinationWidget for T {}
impl<T: MultiChildWidget> !SingleChildWidget for T {}
impl<T: SingleChildWidget> !MultiChildWidget for T {}

pub macro render_widget_base_impl() {
  #[inline]
  fn classify(&self) -> WidgetClassify { WidgetClassify::Render(self) }

  #[inline]
  fn classify_mut(&mut self) -> WidgetClassifyMut { WidgetClassifyMut::Render(self) }
}

pub macro single_child_widget_base_impl() {
  #[inline]
  fn classify(&self) -> WidgetClassify { WidgetClassify::SingleChild(self) }

  #[inline]
  fn classify_mut(&mut self) -> WidgetClassifyMut { WidgetClassifyMut::SingleChild(self) }
}

pub macro multi_child_widget_base_impl() {
  #[inline]
  fn classify(&self) -> WidgetClassify { WidgetClassify::MultiChild(self) }

  #[inline]
  fn classify_mut(&mut self) -> WidgetClassifyMut { WidgetClassifyMut::MultiChild(self) }
}

fn listen_pointer_event<H: Fn(&PointerEvent) + 'static>(
  mut w: BoxWidget,
  event_type: PointerEventType,
  handler: H,
) -> BoxWidget {
  if let Some(listener) = Widget::downcast_mut::<PointerListener>(&mut w) {
    listener.listen_on(event_type, handler);
    w.box_it()
  } else {
    let mut pointer = PointerListener::new(w);
    pointer.listen_on(event_type, handler);
    pointer.box_it()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn dynamic_cast() {
    let mut widget = Text("hello".to_string())
      .with_key(0)
      .on_pointer_down(|_| {});

    assert!(Widget::downcast_ref::<KeyDetect>(&widget).is_some());
    assert!(Widget::downcast_mut::<KeyDetect>(&mut widget).is_some());
    assert!(Widget::downcast_ref::<PointerListener>(&widget).is_some());
    assert!(Widget::downcast_mut::<PointerListener>(&mut widget).is_some());
    assert!(Widget::downcast_ref::<Text>(&widget).is_some());
    assert!(Widget::downcast_mut::<Text>(&mut widget).is_some());
  }
}
