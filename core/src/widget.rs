use crate::{prelude::*, render::*};
use std::{
  any::{Any, TypeId},
  fmt::Debug,
};
pub mod build_ctx;
pub mod key;
pub mod layout;
pub use layout::*;
pub mod stateful;
pub mod text;
mod theme;
pub use theme::*;
pub mod widget_tree;
pub mod window;
pub use build_ctx::BuildCtx;
pub use key::{Key, KeyDetect};
pub use stateful::{StateRef, Stateful};
pub use text::Text;
pub mod events;
pub use events::*;
mod phantom;
pub use phantom::PhantomWidget;
pub use smallvec::{smallvec, SmallVec};
mod cursor;
pub use cursor::Cursor;
use rxrust::prelude::*;
pub use winit::window::CursorIcon;
mod margin;
pub use margin::*;
mod padding;
pub use padding::*;
mod box_decoration;
pub use box_decoration::*;
mod attr;
pub use attr::*;

/// The common behavior of widgets, also support to dynamic cast to special
/// widget. In most of cases, needn't implement `Widget` trait directly, and
/// implement `CombinationWidget`, `RenderWidget` instead of
pub trait Widget: Debug + Any {
  /// classify this widget into one of four type widget, and return the
  /// reference.
  fn classify(&self) -> WidgetClassify;

  /// classify this widget into one of four type widget as mutation reference.
  fn classify_mut(&mut self) -> WidgetClassifyMut;

  fn as_any(&self) -> &dyn Any;
  fn as_any_mut(&mut self) -> &mut dyn Any;

  #[inline]
  fn is_combination(&self) -> bool { matches!(self.classify(), WidgetClassify::Combination(_)) }

  #[inline]
  fn is_render(&self) -> bool { !matches!(self.classify(), WidgetClassify::Combination(_)) }

  /// return the some-value of `InheritWidget` reference if the widget is
  /// inherit from another widget, otherwise None.
  #[inline]
  fn as_inherit(&self) -> Option<&dyn InheritWidget> { None }

  /// like `as_inherit`, but return mutable reference.
  #[inline]
  fn as_inherit_mut(&mut self) -> Option<&mut dyn InheritWidget> { None }

  /// return the some-value of `WidgetAttr` reference if the widget attached
  /// attr.
  #[inline]
  fn as_attr(&self) -> Option<&dyn Attribute>
  where
    Self: Sized,
  {
    None
  }

  /// like `as_attr`, but return mutable reference.
  #[inline]
  fn as_attr_mut(&mut self) -> Option<&mut dyn Attribute>
  where
    Self: Sized,
  {
    None
  }

  fn downcast_attr_widget<Attr: Attribute>(&self) -> Option<&Attr>
  where
    Self: Sized,
  {
    let mut attr = self.as_attr();

    while let Some(a) = attr {
      let target_attr = a.as_any().downcast_ref::<Attr>();
      if target_attr.is_some() {
        return target_attr;
      } else {
        attr = a.widget().as_attr();
      }
    }
    None
  }

  fn box_it(self) -> BoxWidget
  where
    Self: Sized,
  {
    BoxWidget {
      widget: Box::new(self),
    }
  }

  /// Convert a stateless widget to stateful, and will split to a stateful
  /// widget, and a `StateRef` which can be use to modify the states of the
  /// widget.
  #[inline]
  fn into_stateful(self, ctx: &mut BuildCtx) -> Stateful<Self>
  where
    Self: Sized,
  {
    Stateful::stateful(self, ctx.tree.as_mut())
  }

  /// Assign the type of mouse cursor, show when the mouse pointer is over this
  /// widget.
  #[inline]
  fn with_cursor(self, cursor: CursorIcon) -> Cursor
  where
    Self: Sized,
  {
    Cursor::new(cursor, self)
  }

  /// Assign whether the `widget` should automatically get focus when the window
  /// loads. Indicates the `widget` can be focused.
  #[inline]
  fn with_auto_focus(self, auto_focus: bool) -> BoxWidget
  where
    Self: Sized,
  {
    FocusListener::from_widget(self.box_it(), Some(auto_focus), None)
  }

  /// Assign where the widget participates in sequential keyboard navigation.
  /// Indicates the `widget` can be focused and
  #[inline]
  fn with_tab_index(self, tab_index: i16) -> BoxWidget
  where
    Self: Sized,
  {
    FocusListener::from_widget(self.box_it(), None, Some(tab_index))
  }

  /// Insets the child of a widget by the given padding.
  #[inline]
  fn with_padding(self, edges: EdgeInsets) -> Padding
  where
    Self: Sized,
  {
    Padding {
      padding: edges,
      child: self.box_it(),
    }
  }

  /// Create space around the widget
  #[inline]
  fn with_margin(self, edges: EdgeInsets) -> Margin
  where
    Self: Sized,
  {
    Margin {
      margin: edges,
      child: self.box_it(),
    }
  }

  /// Sets the background of the widget.
  fn with_background(mut self, background: FillStyle) -> BoxWidget
  where
    Self: Sized,
  {
    if let Some(box_decoration) = Widget::dynamic_cast_mut::<BoxDecoration>(&mut self) {
      box_decoration.background = Some(background);
      self.box_it()
    } else {
      BoxDecoration::new(self.box_it())
        .with_background(background)
        .box_it()
    }
  }

  /// Set the border of the widget
  fn with_border(mut self, border: Border) -> BoxWidget
  where
    Self: Sized,
  {
    if let Some(box_decoration) = Widget::dynamic_cast_mut::<BoxDecoration>(&mut self) {
      box_decoration.border = Some(border);
      self.box_it()
    } else {
      BoxDecoration::new(self.box_it())
        .width_border(border)
        .box_it()
    }
  }

  /// Set the radius of the widget.
  fn with_border_radius(mut self, radius: BorderRadius) -> BoxWidget
  where
    Self: Sized,
  {
    if let Some(box_decoration) = Widget::dynamic_cast_mut::<BoxDecoration>(&mut self) {
      box_decoration.radius = Some(radius);
      self.box_it()
    } else {
      BoxDecoration::new(self.box_it())
        .with_border_radius(radius)
        .box_it()
    }
  }

  /// Used to specify the event handler for the pointer down event, which is
  /// fired when the pointing device is initially pressed.
  #[inline]
  fn on_pointer_down<F>(self, handler: F) -> BoxWidget
  where
    Self: Sized,
    F: FnMut(&PointerEvent) + 'static,
  {
    PointerListener::listen_on(self.box_it(), PointerEventType::Down, handler)
  }

  /// Used to specify the event handler for the pointer up event, which is
  /// fired when the all pressed pointing device is released.
  #[inline]
  fn on_pointer_up<F>(self, handler: F) -> BoxWidget
  where
    Self: Sized,
    F: FnMut(&PointerEvent) + 'static,
  {
    PointerListener::listen_on(self.box_it(), PointerEventType::Up, handler)
  }

  /// Specify the event handler to process pointer move event.
  #[inline]
  fn on_pointer_move<F>(self, handler: F) -> BoxWidget
  where
    Self: Sized,
    F: FnMut(&PointerEvent) + 'static,
  {
    PointerListener::listen_on(self.box_it(), PointerEventType::Move, handler)
  }

  /// Specify the event handler to process pointer tap event.
  #[inline]
  fn on_tap<F>(self, handler: F) -> BoxWidget
  where
    Self: Sized,
    F: FnMut(&PointerEvent) + 'static,
  {
    PointerListener::listen_on(self.box_it(), PointerEventType::Tap, handler)
  }

  /// Specify the event handler to process pointer tap event.
  fn on_tap_times<F>(self, times: u8, mut handler: F) -> BoxWidget
  where
    Self: Sized,
    F: FnMut(&PointerEvent) + 'static,
  {
    let mut pointer = PointerListener::from_widget(self);
    Widget::dynamic_cast_mut::<PointerListener>(&mut pointer)
      .unwrap()
      .tap_times_observable(times)
      .subscribe(move |e| handler(&*e));
    pointer
  }

  /// Specify the event handler to process pointer cancel event.
  #[inline]
  fn on_pointer_cancel<F>(self, handler: F) -> BoxWidget
  where
    Self: Sized,
    F: FnMut(&PointerEvent) + 'static,
  {
    PointerListener::listen_on(self.box_it(), PointerEventType::Cancel, handler)
  }

  /// specify the event handler when pointer enter this widget.
  #[inline]
  fn on_pointer_enter<F>(self, handler: F) -> BoxWidget
  where
    Self: Sized,
    F: FnMut(&PointerEvent) + 'static,
  {
    PointerListener::listen_on(self.box_it(), PointerEventType::Enter, handler)
  }

  /// Specify the event handler when pointer leave this widget.
  #[inline]
  fn on_pointer_leave<F>(self, handler: F) -> BoxWidget
  where
    Self: Sized,
    F: FnMut(&PointerEvent) + 'static,
  {
    PointerListener::listen_on(self.box_it(), PointerEventType::Leave, handler)
  }

  /// Specify the event handler to process focus event. The focus event is
  /// raised when when the user sets focus on an element.
  #[inline]
  fn on_focus<F>(self, handler: F) -> BoxWidget
  where
    Self: Sized,
    F: FnMut(&FocusEvent) + 'static,
  {
    FocusListener::listen_on(self.box_it(), FocusEventType::Focus, handler)
  }

  /// Specify the event handler to process blur event. The blur event is raised
  /// when an widget loses focus.
  #[inline]
  fn on_blur<F>(self, handler: F) -> BoxWidget
  where
    Self: Sized,
    F: FnMut(&FocusEvent) + 'static,
  {
    FocusListener::listen_on(self.box_it(), FocusEventType::Blur, handler)
  }

  /// Specify the event handler to process focusin event.  The main difference
  /// between this event and blur is that focusin bubbles while blur does not.
  #[inline]
  fn on_focus_in<F>(self, handler: F) -> BoxWidget
  where
    Self: Sized,
    F: FnMut(&FocusEvent) + 'static,
  {
    FocusListener::listen_on(self.box_it(), FocusEventType::FocusIn, handler)
  }

  /// Specify the event handler to process focusout event. The main difference
  /// between this event and blur is that focusout bubbles while blur does not.
  #[inline]
  fn on_focus_out<F>(self, handler: F) -> BoxWidget
  where
    Self: Sized,
    F: FnMut(&FocusEvent) + 'static,
  {
    FocusListener::listen_on(self.box_it(), FocusEventType::FocusOut, handler)
  }

  /// Specify the event handler when keyboard press down.
  #[inline]
  fn on_key_down<F>(self, handler: F) -> BoxWidget
  where
    Self: Sized,
    F: FnMut(&KeyboardEvent) + 'static,
  {
    KeyboardListener::listen_on(self.box_it(), KeyboardEventType::KeyDown, handler)
  }

  /// Specify the event handler when a key is released.
  #[inline]
  fn on_key_up<F>(self, handler: F) -> BoxWidget
  where
    Self: Sized,
    F: FnMut(&KeyboardEvent) + 'static,
  {
    KeyboardListener::listen_on(self.box_it(), KeyboardEventType::KeyUp, handler)
  }

  /// Specify the event handler when received a unicode character.
  #[inline]
  fn on_char<F>(self, mut handler: F) -> BoxWidget
  where
    Self: Sized,
    F: FnMut(&CharEvent) + 'static,
  {
    let widget = CharListener::from_widget(self.box_it());
    Widget::dynamic_cast_ref::<CharListener>(&widget)
      .unwrap()
      .event_observable()
      .subscribe(move |char_event| handler(&*char_event));
    widget
  }
}

/// A widget represented by other widget compose.
pub trait CombinationWidget: Debug {
  /// Describes the part of the user interface represented by this widget.
  /// Called by framework, should never directly call it.
  fn build(&self, ctx: &mut BuildCtx) -> BoxWidget;
}

/// RenderWidget provide configuration for render object which provide actual
/// rendering or computing layout for the application.
pub trait RenderWidget: Debug + Sized {
  /// The render object type will created.
  type RO: RenderObject<Owner = Self> + Send + Sync + 'static;

  /// Creates an instance of the RenderObject that this RenderWidget
  /// represents, using the configuration described by this RenderWidget
  fn create_render_object(&self) -> Self::RO;

  /// Called by framework to take children from this widget, return some-value
  /// to if it has child, else return None. This method will only be called
  /// once. Should never directly call it.
  fn take_children(&mut self) -> Option<SmallVec<[BoxWidget; 1]>>;
}

/// RenderWidgetSafety is a object safety trait of RenderWidget, never directly
/// implement this trait, just implement [`RenderWidget`](RenderWidget).
pub trait RenderWidgetSafety: Debug {
  fn create_render_object(&self) -> Box<dyn RenderObjectSafety + Send + Sync>;
  fn take_children(&mut self) -> Option<SmallVec<[BoxWidget; 1]>>;
}

pub enum WidgetClassify<'a> {
  Combination(&'a dyn CombinationWidget),
  Render(&'a dyn RenderWidgetSafety),
}

pub enum WidgetClassifyMut<'a> {
  Combination(&'a mut dyn CombinationWidget),
  Render(&'a mut dyn RenderWidgetSafety),
}

/// Use inherit method to implement a `Widget`, this is use to extend ability of
/// a widget but not increase the widget number. Notice it's difference to class
/// inherit, it's instance inherit. If the base widget already inherit a same
/// type widget, the new widget should merge into the same type base widget. If
/// the base widget is a `StatefulWidget`, the new widget should inherit
/// between `StatefulWidget` and its base widget, new widget inherit the base
/// widget of `StatefulWidget` and `StatefulWidget` inherit the new widget.
/// `StatefulWidget` is so special is because it's a preallocate widget in
/// widget tree, so if we not do this, the widget inherit from `StatefulWidget`
/// will be lost, so widget inherit `StatefulWidget` will be convert to be
/// inherited by it. Base on the before two point, the inherit order are not
/// guaranteed.
pub trait InheritWidget: Widget {
  fn base_widget(&self) -> &dyn Widget;
  fn base_widget_mut(&mut self) -> &mut dyn Widget;
}

pub struct BoxWidget {
  pub(crate) widget: Box<dyn Widget>,
}

impl std::fmt::Debug for BoxWidget {
  #[inline]
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { self.widget.fmt(f) }
}

impl InheritWidget for BoxWidget {
  #[inline]
  fn base_widget(&self) -> &dyn Widget { self.widget.borrow() }
  #[inline]
  fn base_widget_mut(&mut self) -> &mut dyn Widget { self.widget.borrow_mut() }
}

impl Widget for BoxWidget {
  #[inline]
  fn classify(&self) -> WidgetClassify { self.base_widget().classify() }

  #[inline]
  fn classify_mut(&mut self) -> WidgetClassifyMut { self.base_widget_mut().classify_mut() }

  #[inline]
  fn as_inherit(&self) -> Option<&dyn InheritWidget> { Some(self) }

  #[inline]
  fn as_inherit_mut(&mut self) -> Option<&mut dyn InheritWidget> { Some(self) }

  #[inline]
  fn as_any(&self) -> &dyn Any { self }

  #[inline]
  fn as_any_mut(&mut self) -> &mut dyn Any { self }

  fn box_it(self) -> BoxWidget
  where
    Self: Sized,
  {
    self
  }
}

impl From<Box<dyn Widget>> for BoxWidget {
  #[inline]
  fn from(widget: Box<dyn Widget>) -> Self { Self { widget } }
}

/// A function help `InheritWidget` inherit `base` widget.
///
/// ## params
/// *base*: the base widget want inherit from.
/// *ctor_by_base*: construct widget with the base widget should really inherit
/// *merge*: use to merge the widget into the base widget `T`, if the type `T`
/// is already be inherited in the  base widgets.
pub fn inherit<T: InheritWidget, C, M>(
  mut base: BoxWidget,
  mut ctor_by_base: C,
  mut merge: M,
) -> BoxWidget
where
  M: FnMut(&mut T),
  C: FnMut(BoxWidget) -> T,
{
  if let Some(already) = Widget::dynamic_cast_mut::<T>(&mut base) {
    merge(already);
    base
  } else if let Some(stateful) = Widget::dynamic_cast_mut::<Stateful<BoxWidget>>(&mut base) {
    stateful.replace_base_with(|base| ctor_by_base(base).box_it());
    base
  } else {
    ctor_by_base(base).box_it()
  }
}

impl dyn Widget {
  /// Returns some mutable reference to the boxed value if it or its **base
  /// widget** is of type T, or None if it isn't.
  pub fn dynamic_cast_mut<T: 'static>(&mut self) -> Option<&mut T> {
    if Any::type_id(self) == TypeId::of::<T>() {
      let ptr = self as *mut dyn Widget as *mut T;
      // SAFETY: just checked whether we are pointing to the correct type, and we can
      // rely on that check for memory safety because we have implemented Any for
      // all types; no other impls can exist as they would conflict with our impl.
      unsafe { Some(&mut *ptr) }
    } else {
      self
        .as_inherit_mut()
        .and_then(|inherit| inherit.base_widget_mut().dynamic_cast_mut())
    }
  }

  /// Returns some reference to the boxed value if it or its **base widget** is
  /// of type T, or None if it isn't.
  pub fn dynamic_cast_ref<T: 'static>(&self) -> Option<&T> {
    if self.type_id() == TypeId::of::<T>() {
      let ptr = self as *const dyn Widget as *const T;
      // SAFETY: just checked whether we are pointing to the correct type, and we can
      // rely on that check for memory safety because we have implemented Any for
      // all types; no other impls can exist as they would conflict with our impl.
      unsafe { Some(&*ptr) }
    } else {
      self
        .as_inherit()
        .and_then(|inherit| inherit.base_widget().dynamic_cast_ref())
    }
  }
}

use std::borrow::{Borrow, BorrowMut};

pub macro inherit_widget(
  $ty: ty,
  $base_widget: tt
  $(, <$($generics: tt),*>)?
  $(, where $($wty:ty : $bound: tt),*)?
) {
  impl<$($($generics ,)*)?> InheritWidget for $ty
  where
    $($($wty: $bound), *)?
  {
    #[inline]
    fn base_widget(&self) -> &dyn Widget { self.$base_widget.borrow() }
    #[inline]
    fn base_widget_mut(&mut self) -> &mut dyn Widget { self.$base_widget.borrow_mut() }
  }

  impl_widget_for_inherit_widget!($ty $(, <$($generics),*>)? $(, where $($wty : $bound),*)?);
}

/// Auto implement `Widget` for `CombinationWidget`,  We should also implement
/// `Widget` for `RenderWidget`, but can not do it before rust specialization
/// finished. So just CombinationWidget implemented it, this is user use most,
/// and others provide a macro to do it.
impl<T: CombinationWidget + 'static> Widget for T {
  #[inline]
  fn classify(&self) -> WidgetClassify { WidgetClassify::Combination(self) }

  #[inline]
  fn classify_mut(&mut self) -> WidgetClassifyMut { WidgetClassifyMut::Combination(self) }

  #[inline]
  fn as_any(&self) -> &dyn Any { self }

  #[inline]
  fn as_any_mut(&mut self) -> &mut dyn Any { self }
}

impl<T: CombinationWidget> !RenderWidget for T {}
impl<T: RenderWidget> !CombinationWidget for T {}

pub macro render_widget_base_impl($ty: ty) {
  impl Widget for $ty {
    #[inline]
    fn classify(&self) -> WidgetClassify { WidgetClassify::Render(self) }

    #[inline]
    fn classify_mut(&mut self) -> WidgetClassifyMut { WidgetClassifyMut::Render(self) }

    #[inline]
    fn as_any(&self) -> &dyn Any { self }

    #[inline]
    fn as_any_mut(&mut self) -> &mut dyn Any { self }
  }
}

pub macro impl_widget_for_inherit_widget(
  $ty: ty
  $(, <$($generics: tt),*>)?
  $(, where $($wty:ty : $bound: tt),*)?
) {
  impl<$($($generics ,)*)?> Widget for $ty
  where
    $($($wty: $bound), *)?
  {
    #[inline]
    fn classify(&self) -> WidgetClassify { self.base_widget().classify() }

    #[inline]
    fn classify_mut(&mut self) -> WidgetClassifyMut { self.base_widget_mut().classify_mut() }

    #[inline]
    fn as_inherit(&self) -> Option<&dyn InheritWidget> { Some(self) }

    #[inline]
    fn as_inherit_mut(&mut self) -> Option<&mut dyn InheritWidget> { Some(self) }

    #[inline]
    fn as_any(&self) -> &dyn Any { self }

    #[inline]
    fn as_any_mut(&mut self) -> &mut dyn Any { self }
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

    assert!(Widget::dynamic_cast_ref::<PointerListener>(&widget).is_some());
    assert!(Widget::dynamic_cast_mut::<PointerListener>(&mut widget).is_some());
    assert!(Widget::dynamic_cast_ref::<Text>(&widget).is_some());
    assert!(Widget::dynamic_cast_mut::<Text>(&mut widget).is_some());
  }
}
