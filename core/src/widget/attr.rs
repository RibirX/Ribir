//! Attributes is use to extend ability of a widget. Across attach attribute,
//! widget can be expanded much ability from the attributes. The attach means
//! the widget number will not increase after attributes attached and the origin
//! widget behavior will be kept.

//! Note that widget use the attribute ability across [`AttrsAccess`]!,
//! widget can't hold two same type attribute, so if you implement a custom
//! attribute, you should merge or replace the same type attr if user try to
//! attach more than once.
//!
//! Ribir provide many builtin attributes, and provide method to easy attach,
//! See [`AttachAttr`]!.
//!
//! We can implement a custom widget and the builtin attributes can be directly
//! attached to it.

//! ```
//! # #![feature(negative_impls)]
//! # use ribir::prelude::*;
//! // implement a custom widget.
//! pub struct MyCheckbox;
//!
//! impl CombinationWidget for MyCheckbox {
//!   fn build(&self, ctx: &mut  BuildCtx) -> BoxedWidget {
//!     declare!{
//!       Checkbox {
//!         style: ctx.theme().checkbox.clone(),
//!         ..<_>::default()
//!       }
//!     }
//!   }
//! }
//!
//! let checkbox = MyCheckbox
//!  // can use key attribute
//!  .with_key(1)
//!  // can use pointer listener attribute feedback pointer event input.
//!  .on_pointer_move(|_| {})
//!  // char listener attribute too
//!  .on_char(|_| {});
//!  // and more ....
//! ```
//! # Custom implement attribute.
//!
//! To implement custom attribute, use [`AttrWidget`][AttrWidget] is the easiest
//! way. For example

//! ```
//! # use ribir::{prelude::*, widget::AttrWidget};

//! pub struct HelloAttr;
//!
//! impl HelloAttr {
//!   pub fn hello(&self) {
//!     println!("Hello!");
//!   }
//! }
//!
//! let mut text = Text{ text: "".into(), style:
//! <_>::default()}.into_attr_widget(); text.attrs_mut().insert(HelloAttr);
//! let w: BoxedWidget = text.box_it();
//! w.get_attrs().and_then(Attributes::find::<HelloAttr>).unwrap().hello();
//! ```

//! [attr_impl]: crate::widget::attr::WidgetAttr

use crate::prelude::*;
use std::{
  any::{Any, TypeId},
  collections::HashMap,
  marker::PhantomData,
};
use widget::{focus_listen_on, keyboard_listen_on, pointer_listen_on};

/// `AttachAttr` provide the ability to attach the builtin attrs implemented by
/// Ribir. See the [module-level documentation][mod] for more details.
///
/// [mod]: crate::widget::attr
pub trait AttachAttr {
  type W: Attrs;

  /// Assign a key to the widget to help framework to track if two widget is a
  /// same widget in two frame.
  #[inline]
  fn with_key<K: Into<Key> + 'static>(self, key: K) -> Self::W
  where
    Self: Sized,
  {
    let mut w = self.into_attr_widget();
    w.attrs_mut().insert(key.into());
    w
  }

  /// Assign the type of mouse cursor, show when the mouse pointer is over this
  /// widget.
  #[inline]
  fn with_cursor(self, cursor: CursorIcon) -> Self::W
  where
    Self: Sized,
    Self::W: AttachAttr<W = Self::W>,
  {
    widget::cursor::cursor_attach(cursor, self)
  }

  #[inline]
  fn with_theme(self, data: Theme) -> Self::W
  where
    Self: Sized,
  {
    let mut w = self.into_attr_widget();
    w.attrs_mut().insert(data);
    w
  }

  /// Used to specify the event handler for the pointer down event, which is
  /// fired when the pointing device is initially pressed.
  #[inline]
  fn on_pointer_down<F>(self, handler: F) -> Self::W
  where
    Self: Sized,
    F: FnMut(&PointerEvent) + 'static,
  {
    pointer_listen_on(self, PointerEventType::Down, handler)
  }

  /// Used to specify the event handler for the pointer up event, which is
  /// fired when the all pressed pointing device is released.
  #[inline]
  fn on_pointer_up<F>(self, handler: F) -> Self::W
  where
    Self: Sized,
    F: FnMut(&PointerEvent) + 'static,
  {
    pointer_listen_on(self, PointerEventType::Up, handler)
  }

  /// Specify the event handler to process pointer move event.
  #[inline]
  fn on_pointer_move<F>(self, handler: F) -> Self::W
  where
    Self: Sized,
    F: FnMut(&PointerEvent) + 'static,
  {
    pointer_listen_on(self, PointerEventType::Move, handler)
  }

  /// Specify the event handler to process pointer tap event.
  #[inline]
  fn on_tap<F>(self, handler: F) -> Self::W
  where
    Self: Sized,
    F: FnMut(&PointerEvent) + 'static,
  {
    pointer_listen_on(self, PointerEventType::Tap, handler)
  }

  /// Specify the event handler to process tap event with `times` tap.
  fn on_tap_times<F>(self, times: u8, mut handler: F) -> Self::W
  where
    Self: Sized,
    F: FnMut(&PointerEvent) + 'static,
  {
    let mut w = self.into_attr_widget();
    w.attrs_mut()
      .entry::<PointerAttr>()
      .or_default()
      .tap_times_observable(times)
      .subscribe(move |e| handler(&*e));
    w
  }

  /// Specify the event handler to process pointer cancel event.
  #[inline]
  fn on_pointer_cancel<F>(self, handler: F) -> Self::W
  where
    Self: Sized,
    F: FnMut(&PointerEvent) + 'static,
  {
    pointer_listen_on(self, PointerEventType::Cancel, handler)
  }

  /// Specify the event handler when pointer enter this widget.
  #[inline]
  fn on_pointer_enter<F>(self, handler: F) -> Self::W
  where
    Self: Sized,
    F: FnMut(&PointerEvent) + 'static,
  {
    pointer_listen_on(self, PointerEventType::Enter, handler)
  }

  /// Specify the event handler when pointer leave this widget.
  #[inline]
  fn on_pointer_leave<F>(self, handler: F) -> Self::W
  where
    Self: Sized,
    F: FnMut(&PointerEvent) + 'static,
  {
    pointer_listen_on(self, PointerEventType::Leave, handler)
  }

  /// Assign whether the `widget` should automatically get focus when the window
  /// loads. Indicates the `widget` can be focused.
  #[inline]
  fn with_auto_focus(self, auto_focus: bool) -> Self::W
  where
    Self: Sized,
  {
    let mut w = self.into_attr_widget();
    w.attrs_mut().entry::<FocusAttr>().or_default().auto_focus = auto_focus;
    w
  }

  /// Assign where the widget participates in sequential keyboard navigation.
  /// Indicates the `widget` can be focused and
  #[inline]
  fn with_tab_index(self, tab_index: i16) -> Self::W
  where
    Self: Sized,
  {
    let mut w = self.into_attr_widget();
    w.attrs_mut().entry::<FocusAttr>().or_default().tab_index = tab_index;
    w
  }

  /// Specify the event handler to process focus event. The focus event is
  /// raised when when the user sets focus on an element.
  #[inline]
  fn on_focus<F>(self, handler: F) -> Self::W
  where
    Self: Sized,
    F: FnMut(&FocusEvent) + 'static,
  {
    focus_listen_on(self, FocusEventType::Focus, handler)
  }

  /// Specify the event handler to process blur event. The blur event is raised
  /// when an widget loses focus.
  #[inline]
  fn on_blur<F>(self, handler: F) -> Self::W
  where
    Self: Sized,
    F: FnMut(&FocusEvent) + 'static,
  {
    focus_listen_on(self, FocusEventType::Blur, handler)
  }

  /// Specify the event handler to process focusin event.  The main difference
  /// between this event and blur is that focusin bubbles while blur does not.
  #[inline]
  fn on_focus_in<F>(self, handler: F) -> Self::W
  where
    Self: Sized,
    F: FnMut(&FocusEvent) + 'static,
  {
    focus_listen_on(self, FocusEventType::FocusIn, handler)
  }

  /// Specify the event handler to process focusout event. The main difference
  /// between this event and blur is that focusout bubbles while blur does not.
  #[inline]
  fn on_focus_out<F>(self, handler: F) -> Self::W
  where
    Self: Sized,
    F: FnMut(&FocusEvent) + 'static,
  {
    focus_listen_on(self, FocusEventType::FocusOut, handler)
  }

  /// Specify the event handler when keyboard press down.
  #[inline]
  fn on_key_down<F>(self, handler: F) -> Self::W
  where
    Self: Sized,
    F: FnMut(&KeyboardEvent) + 'static,
  {
    keyboard_listen_on(self, KeyboardEventType::KeyDown, handler)
  }

  /// Specify the event handler when a key is released.
  #[inline]
  fn on_key_up<F>(self, handler: F) -> Self::W
  where
    Self: Sized,
    F: FnMut(&KeyboardEvent) + 'static,
  {
    keyboard_listen_on(self, KeyboardEventType::KeyUp, handler)
  }

  /// Specify the event handler when received a unicode character.
  fn on_char<F>(self, mut handler: F) -> Self::W
  where
    Self: Sized,
    F: FnMut(&CharEvent) + 'static,
  {
    let mut w = self.into_attr_widget();

    // ensure focus attr attached, because a widget can accept char event base
    // on it can be focused.
    w.attrs_mut().entry::<FocusAttr>().or_default();
    w.attrs_mut()
      .entry::<CharAttr>()
      .or_default()
      .event_observable()
      .subscribe(move |char_event| handler(&*char_event));

    w
  }

  /// Specify the event handler when user moving a mouse wheel or similar input
  /// device.
  fn on_wheel<F>(self, mut handler: F) -> Self::W
  where
    Self: Sized,
    F: FnMut(&WheelEvent) + 'static,
  {
    let mut w = self.into_attr_widget();
    w.attrs_mut()
      .entry::<WheelAttr>()
      .or_default()
      .event_observable()
      .subscribe(move |wheel_event| handler(&*wheel_event));

    w
  }

  fn into_attr_widget(self) -> Self::W;
}

macro get_attr($name: ident) {
  $name.get_attrs().and_then(Attributes::find)
}

pub trait AttrsAccess {
  /// return reference of the cursor specified to this widget if have.
  #[inline]
  fn get_key(&self) -> Option<&Key> { get_attr!(self) }

  /// return reference of the cursor specified to this widget if have.
  fn get_cursor(&self) -> Option<CursorIcon> { get_attr!(self).map(Cursor::icon) }

  /// Try to set cursor icon of a widget, return false if success which the
  /// widget is not implement `Attrs` . Otherwise return true.
  fn try_set_cursor(&mut self, icon: CursorIcon) -> bool {
    self
      .get_attrs_mut()
      .map(|attrs| {
        if let Some(cursor) = attrs.find_mut::<Cursor>() {
          cursor.set_icon(icon);
        } else {
          attrs.insert(Cursor::default()).map(|c| c.icon());
        }
      })
      .is_some()
  }

  /// Return reference of the theme attach to this widget if have. This function
  /// not find theme in ancestors, if you want to find the theme effect this
  /// widget, usually you should use the
  /// [`BuildCtx::theme`](crate::widget::BuildCtx).
  #[inline]
  fn get_theme(&self) -> Option<&Theme> { get_attr!(self) }

  /// Try to set theme to the subtree of the widget, return false if success
  /// which the widget is not implement `Attrs`. Otherwise return true.
  fn try_set_theme(&mut self, theme: Theme) -> bool {
    self
      .get_attrs_mut()
      .map(|attrs| attrs.insert(theme))
      .is_some()
  }

  /// Return the sequential keyboard navigation of widget if it is a focusable
  /// widget.
  fn get_tab_index(&self) -> Option<i16> { get_attr!(self).map(|f: &FocusAttr| f.tab_index) }

  /// Try to set the sequential keyboard navigation of widget, return false if
  /// the widget is implement `Attrs`. Otherwise return true.
  fn try_set_tab_index(&mut self, tab_index: i16) -> bool {
    self
      .get_attrs_mut()
      .map(|attrs| attrs.entry::<FocusAttr>().or_default().tab_index = tab_index)
      .is_some()
  }

  /// Return if the widget is auto focused if it is a focusable widget.
  fn get_auto_focus(&self) -> Option<bool> { get_attr!(self).map(|f: &FocusAttr| f.auto_focus) }

  /// Try to set auto focus of widget, return false if success which the widget
  /// is implement `Attrs`. Otherwise return true.
  fn try_set_auto_focus(&mut self, auto_focus: bool) -> bool {
    self
      .get_attrs_mut()
      .map(|attrs| attrs.entry::<FocusAttr>().or_default().auto_focus = auto_focus)
      .is_some()
  }

  fn get_attrs(&self) -> Option<&Attributes>;

  fn get_attrs_mut(&mut self) -> Option<&mut Attributes>;
}

pub trait Attrs: AttachAttr {
  fn attrs(&self) -> &Attributes;

  fn attrs_mut(&mut self) -> &mut Attributes;
}

pub auto trait NoAttrs {}
impl<W> !NoAttrs for AttrWidget<W> {}

impl<W: NoAttrs> AttachAttr for W {
  type W = AttrWidget<W>;
  #[inline]
  fn into_attr_widget(self) -> Self::W { AttrWidget { widget: self, attrs: <_>::default() } }
}

impl<W> AttachAttr for AttrWidget<W> {
  type W = AttrWidget<W>;
  #[inline]
  fn into_attr_widget(self) -> Self::W { self }
}

impl<W: NoAttrs> AttrsAccess for W {
  #[inline]
  fn get_attrs(&self) -> Option<&Attributes> { None }

  #[inline]
  fn get_attrs_mut(&mut self) -> Option<&mut Attributes> { None }
}

impl<W> AttrsAccess for AttrWidget<W> {
  #[inline]
  fn get_attrs(&self) -> Option<&Attributes> { Some(self.attrs()) }

  #[inline]
  fn get_attrs_mut(&mut self) -> Option<&mut Attributes> { Some(self.attrs_mut()) }
}

impl<W> Attrs for AttrWidget<W> {
  #[inline]
  fn attrs(&self) -> &Attributes { &self.attrs }

  #[inline]
  fn attrs_mut(&mut self) -> &mut Attributes { &mut self.attrs }
}

#[derive(CombinationWidget, RenderWidget, SingleChildWidget, MultiChildWidget)]
pub struct AttrWidget<W> {
  #[proxy]
  pub widget: W,
  pub attrs: Attributes,
}

impl<W: IntoStateful> IntoStateful for AttrWidget<W>
where
  W::S: Attrs,
{
  type S = W::S;

  fn into_stateful(self) -> Self::S {
    let Self { widget, attrs } = self;

    let mut stateful = widget.into_stateful();
    stateful.attrs_mut().0.extend(attrs.0);

    stateful
  }
}

impl<W> std::ops::Deref for AttrWidget<W> {
  type Target = W;
  #[inline]
  fn deref(&self) -> &Self::Target { &self.widget }
}

impl<W> std::ops::DerefMut for AttrWidget<W> {
  #[inline]
  fn deref_mut(&mut self) -> &mut Self::Target { &mut self.widget }
}

#[derive(Default)]
pub struct Attributes(HashMap<TypeId, Box<dyn Any>>);

impl Attributes {
  pub fn insert<A: Any>(&mut self, attr: A) -> Option<Box<A>> {
    self
      .0
      .insert(TypeId::of::<A>(), Box::new(attr))
      .map(|a| a.downcast().unwrap())
  }

  pub fn find<A: Any>(&self) -> Option<&A> {
    self
      .0
      .get(&TypeId::of::<A>())
      .map(|attr| attr.downcast_ref().unwrap())
  }

  pub fn find_mut<A: Any>(&mut self) -> Option<&mut A> {
    self.0.get_mut(&TypeId::of::<A>()).map(attr_downcast_mut)
  }

  #[inline]
  pub fn entry<A: Any>(&mut self) -> Entry<A> {
    Entry {
      entry: self.0.entry(TypeId::of::<A>()),
      type_mark: PhantomData,
    }
  }
}

pub struct Entry<'a, A: Any> {
  entry: std::collections::hash_map::Entry<'a, TypeId, Box<dyn Any>>,
  type_mark: PhantomData<*const A>,
}

impl<'a, A: Any> Entry<'a, A> {
  #[inline]
  pub fn or_default(self) -> &'a mut A
  where
    A: Default,
  {
    self.or_insert_with(A::default)
  }

  pub fn or_insert_with<F: FnOnce() -> A>(self, default: F) -> &'a mut A {
    attr_downcast_mut(self.entry.or_insert_with(|| Box::new(default())))
  }
}

///  Safety: a utility function to downcast attribute, use it only if you know
/// the type backed in the `Box<dyn Any>`
#[inline]
fn attr_downcast_mut<A: Any>(attr: &mut Box<dyn Any>) -> &mut A { attr.downcast_mut().unwrap() }

#[test]
fn fix_into_stateful_keep_attrs() {
  let s = SizedBox { size: Size::zero() }.with_key(1).into_stateful();
  assert_eq!(get_attr!(s), Some(&Key::Ki4(1)));
  assert!(
    s.get_attrs()
      .and_then(Attributes::find::<StateAttr>)
      .is_some()
  );
}
