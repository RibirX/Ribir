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

/// Widget that you can attach attribute to.
pub struct AttrWidget<W> {
  pub widget: W,
  pub attrs: Attributes,
}

/// Trait provide quick way to attach attribute to widget. You should not
/// implement this trait, ribir provide the default implementation
pub trait AttachAttr: Sized {
  type Target: AttachAttr<Target = Self::Target>;
  /// Assign a key to the widget to help framework to track if two widget is a
  /// same widget in two frame.
  #[inline]
  fn with_key<K: Into<Key> + 'static>(self, key: K) -> Self::Target { self.insert_attr(key.into()) }

  /// Assign the type of mouse cursor, show when the mouse pointer is over this
  /// widget.
  #[inline]
  fn with_cursor(self, cursor: CursorIcon) -> Self::Target {
    widget::cursor::cursor_attach(cursor, self)
  }

  #[inline]
  fn with_theme(self, data: Theme) -> Self::Target { self.insert_attr(data) }

  /// Used to specify the event handler for the pointer down event, which is
  /// fired when the pointing device is initially pressed.
  #[inline]
  fn on_pointer_down<F>(self, handler: F) -> Self::Target
  where
    F: FnMut(&mut PointerEvent) + 'static,
  {
    self.or_default_with(|pointer: &mut PointerAttr| {
      pointer.listen_on(PointerEventType::Down, handler);
    })
  }

  /// Used to specify the event handler for the pointer up event, which is
  /// fired when the all pressed pointing device is released.
  #[inline]
  fn on_pointer_up<F>(self, handler: F) -> Self::Target
  where
    F: FnMut(&mut PointerEvent) + 'static,
  {
    self.or_default_with(|pointer: &mut PointerAttr| {
      pointer.listen_on(PointerEventType::Up, handler);
    })
  }

  /// Specify the event handler to process pointer move event.
  #[inline]
  fn on_pointer_move<F>(self, handler: F) -> Self::Target
  where
    F: FnMut(&mut PointerEvent) + 'static,
  {
    self.or_default_with(|pointer: &mut PointerAttr| {
      pointer.listen_on(PointerEventType::Move, handler);
    })
  }

  /// Specify the event handler to process pointer tap event.
  #[inline]
  fn on_tap<F>(self, handler: F) -> Self::Target
  where
    F: FnMut(&mut PointerEvent) + 'static,
  {
    self.or_default_with(|pointer: &mut PointerAttr| {
      pointer.listen_on(PointerEventType::Tap, handler);
    })
  }

  /// Specify the event handler to process tap event with `times` tap.
  fn on_tap_times<F>(self, times: u8, mut handler: F) -> Self::Target
  where
    F: FnMut(&mut PointerEvent) + 'static,
  {
    self.or_default_with(|pointer: &mut PointerAttr| {
      pointer
        .tap_times_observable(times)
        .subscribe(move |e| handler(e));
    })
  }

  /// Specify the event handler to process pointer cancel event.
  #[inline]
  fn on_pointer_cancel<F>(self, handler: F) -> Self::Target
  where
    F: FnMut(&mut PointerEvent) + 'static,
  {
    self.or_default_with(|pointer: &mut PointerAttr| {
      pointer.listen_on(PointerEventType::Cancel, handler);
    })
  }

  /// Specify the event handler when pointer enter this widget.
  #[inline]
  fn on_pointer_enter<F>(self, handler: F) -> Self::Target
  where
    F: FnMut(&mut PointerEvent) + 'static,
  {
    self.or_default_with(|pointer: &mut PointerAttr| {
      pointer.listen_on(PointerEventType::Enter, handler);
    })
  }

  /// Specify the event handler when pointer leave this widget.
  #[inline]
  fn on_pointer_leave<F>(self, handler: F) -> Self::Target
  where
    F: FnMut(&mut PointerEvent) + 'static,
  {
    self.or_default_with(|pointer: &mut PointerAttr| {
      pointer.listen_on(PointerEventType::Leave, handler);
    })
  }

  /// Assign whether the `widget` should automatically get focus when the window
  /// loads. Indicates the `widget` can be focused.
  #[inline]
  fn with_auto_focus(self, auto_focus: bool) -> Self::Target {
    self.or_default_with(|focus: &mut FocusAttr| focus.auto_focus = auto_focus)
  }

  /// Assign where the widget participates in sequential keyboard navigation.
  /// Indicates the `widget` can be focused and
  #[inline]
  fn with_tab_index(self, tab_index: i16) -> Self::Target {
    self.or_default_with(|focus: &mut FocusAttr| focus.tab_index = tab_index)
  }

  /// Specify the event handler to process focus event. The focus event is
  /// raised when when the user sets focus on an element.
  #[inline]
  fn on_focus<F>(self, handler: F) -> Self::Target
  where
    F: FnMut(&FocusEvent) + 'static,
  {
    self.or_default_with(|focus: &mut FocusAttr| {
      focus.listen_on(FocusEventType::Focus, handler);
    })
  }

  /// Specify the event handler to process blur event. The blur event is raised
  /// when an widget loses focus.
  #[inline]
  fn on_blur<F>(self, handler: F) -> Self::Target
  where
    F: FnMut(&FocusEvent) + 'static,
  {
    self.or_default_with(|focus: &mut FocusAttr| {
      focus.listen_on(FocusEventType::Blur, handler);
    })
  }

  /// Specify the event handler to process focusin event.  The main difference
  /// between this event and blur is that focusin bubbles while blur does not.
  #[inline]
  fn on_focus_in<F>(self, handler: F) -> Self::Target
  where
    F: FnMut(&FocusEvent) + 'static,
  {
    self.or_default_with(|focus: &mut FocusAttr| {
      focus.listen_on(FocusEventType::FocusIn, handler);
    })
  }

  /// Specify the event handler to process focusout event. The main difference
  /// between this event and blur is that focusout bubbles while blur does not.
  #[inline]
  fn on_focus_out<F>(self, handler: F) -> Self::Target
  where
    F: FnMut(&FocusEvent) + 'static,
  {
    self.or_default_with(|focus: &mut FocusAttr| {
      focus.listen_on(FocusEventType::FocusOut, handler);
    })
  }

  /// Specify the event handler when keyboard press down.
  fn on_key_down<F>(self, handler: F) -> Self::Target
  where
    F: FnMut(&KeyboardEvent) + 'static,
  {
    self
      .or_default::<FocusAttr>()
      .or_default_with(|keyboard: &mut KeyboardAttr| {
        keyboard.listen_on(KeyboardEventType::KeyDown, handler);
      })
  }

  /// Specify the event handler when a key is released.
  fn on_key_up<F>(self, handler: F) -> Self::Target
  where
    F: FnMut(&KeyboardEvent) + 'static,
  {
    self
      .or_default::<FocusAttr>()
      .or_default_with(|keyboard: &mut KeyboardAttr| {
        keyboard.listen_on(KeyboardEventType::KeyUp, handler);
      })
  }

  /// Specify the event handler when received a unicode character.
  fn on_char<F>(self, mut handler: F) -> Self::Target
  where
    F: FnMut(&CharEvent) + 'static,
  {
    // ensure focus attr attached, because a widget can accept char event base
    // on it can be focused.
    self
      .or_default::<FocusAttr>()
      .or_default_with(|char: &mut CharAttr| {
        char.listen_on(move |char_event| handler(char_event));
      })
  }

  /// Specify the event handler when user moving a mouse wheel or similar input
  /// device.
  fn on_wheel<F>(self, mut handler: F) -> Self::Target
  where
    F: FnMut(&WheelEvent) + 'static,
  {
    self.or_default_with(|wheel: &mut WheelAttr| {
      wheel.listen_on(move |wheel_event| handler(&*wheel_event));
    })
  }

  fn insert_attr<A: 'static>(self, attr: A) -> Self::Target;

  fn or_default<A: Default + 'static>(self) -> Self::Target;

  fn or_default_with<F, A: Default + 'static>(self, f: F) -> Self::Target
  where
    F: FnOnce(&mut A);

  fn inspect_or_else<F, D, A: 'static>(self, default: D, f: F) -> Self::Target
  where
    D: FnOnce() -> A,
    F: FnOnce(&mut A);
}

/// Trait provides methods to quick access builtin attributes
pub trait BuiltinAttrs: AsAttrs {
  /// return reference of the cursor specified to this widget if have.
  #[inline]
  fn get_key(&self) -> Option<&Key> { get_attr!(self) }

  /// return reference of the cursor specified to this widget if have.
  fn get_cursor(&self) -> Option<CursorIcon> { get_attr!(self).map(Cursor::icon) }

  /// Try to set cursor icon of a widget, return false if success which the
  /// widget is not implement `Attrs` . Otherwise return true.
  fn try_set_cursor(&mut self, icon: CursorIcon) -> bool {
    self
      .as_attrs_mut()
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
      .as_attrs_mut()
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
      .as_attrs_mut()
      .map(|attrs| attrs.entry::<FocusAttr>().or_default().tab_index = tab_index)
      .is_some()
  }

  /// Return if the widget is auto focused if it is a focusable widget.
  fn get_auto_focus(&self) -> Option<bool> { get_attr!(self).map(|f: &FocusAttr| f.auto_focus) }

  /// Try to set auto focus of widget, return false if success which the widget
  /// is implement `Attrs`. Otherwise return true.
  fn try_set_auto_focus(&mut self, auto_focus: bool) -> bool {
    self
      .as_attrs_mut()
      .map(|attrs| attrs.entry::<FocusAttr>().or_default().auto_focus = auto_focus)
      .is_some()
  }
}

pub trait AsAttrs {
  fn as_attrs(&self) -> Option<&Attributes>;

  fn as_attrs_mut(&mut self) -> Option<&mut Attributes>;

  fn find_attr<A: 'static>(&self) -> Option<&A>
  where
    Self: Sized,
  {
    self.as_attrs().and_then(Attributes::find)
  }

  fn find_attr_mut<A: 'static>(&mut self) -> Option<&mut A>
  where
    Self: Sized,
  {
    self.as_attrs_mut().and_then(Attributes::find_mut)
  }
}

impl<W: IntoRender> IntoRender for AttrWidget<W> {
  type R = AttrWidgetWrap<W::R>;
  #[inline]
  fn into_render(self) -> Self::R {
    AttrWidgetWrap(AttrWidget {
      widget: self.widget.into_render(),
      attrs: self.attrs,
    })
  }
}

impl<W: IntoCombination> IntoCombination for AttrWidget<W> {
  type C = AttrWidgetWrap<W::C>;

  #[inline]
  fn into_combination(self) -> Self::C {
    AttrWidgetWrap(AttrWidget {
      widget: self.widget.into_combination(),
      attrs: self.attrs,
    })
  }
}

impl<W: SingleChildWidget + IntoRender> SingleChildWidget for AttrWidget<W> {}

impl<W: MultiChildWidget + IntoRender> MultiChildWidget for AttrWidget<W> {}

impl<W: Widget> AsAttrs for W {
  fn as_attrs(&self) -> Option<&Attributes> { None }

  fn as_attrs_mut(&mut self) -> Option<&mut Attributes> { None }
}

impl<W> AsAttrs for AttrWidget<W> {
  fn as_attrs(&self) -> Option<&Attributes> { None }

  fn as_attrs_mut(&mut self) -> Option<&mut Attributes> { None }
}

impl<W: AsAttrs> BuiltinAttrs for W {}

/// A wrap for `AttrWidget` to help we can implement all attributes ability for
/// all widget, and avoid trait implement conflict.
pub struct AttrWidgetWrap<W>(pub(crate) AttrWidget<W>);

impl<W> std::ops::Deref for AttrWidget<W> {
  type Target = W;
  #[inline]
  fn deref(&self) -> &Self::Target { &self.widget }
}

impl<W> std::ops::DerefMut for AttrWidget<W> {
  #[inline]
  fn deref_mut(&mut self) -> &mut Self::Target { &mut self.widget }
}

macro_rules! attr_widget_attach_impl {
  () => {
    #[inline]
    fn insert_attr<A: 'static>(mut self, attr: A) -> Self::Target {
      self.attrs.insert(attr);
      self
    }

    #[inline]
    fn or_default<A: Default + 'static>(mut self) -> Self::Target {
      self.attrs.entry::<A>().or_default();
      self
    }

    fn or_default_with<F, A: Default + 'static>(mut self, f: F) -> Self::Target
    where
      F: FnOnce(&mut A),
    {
      let attr = self.attrs.entry().or_default();
      f(attr);
      self
    }

    fn inspect_or_else<F, D, A: 'static>(mut self, default: D, f: F) -> Self::Target
    where
      D: FnOnce() -> A,
      F: FnOnce(&mut A),
    {
      if let Some(attr) = self.attrs.find_mut() {
        f(attr);
      } else {
        self.attrs.insert(default());
      }
      self
    }
  };
}

macro_rules! no_attr_widget_attach_impl {
  () => {
    fn insert_attr<A: 'static>(self, attr: A) -> Self::Target {
      let mut attrs = Attributes::default();
      attrs.insert(attr);
      AttrWidget { widget: self, attrs }
    }

    #[inline]
    fn or_default<A: Default + 'static>(self) -> Self::Target { self.insert_attr(A::default()) }

    fn or_default_with<F, A: Default + 'static>(self, f: F) -> Self::Target
    where
      F: FnOnce(&mut A),
    {
      let mut attr = A::default();
      f(&mut attr);
      self.insert_attr(attr)
    }

    fn inspect_or_else<F, D, A: 'static>(self, default: D, _: F) -> Self::Target
    where
      D: FnOnce() -> A,
      F: FnOnce(&mut A),
    {
      self.insert_attr(default())
    }
  };
}
impl<W> AttachAttr for AttrWidget<W> {
  type Target = AttrWidget<W>;
  attr_widget_attach_impl!();
}

impl<W: Widget> AttachAttr for W {
  type Target = AttrWidget<W>;
  no_attr_widget_attach_impl!();
}

impl<W> AttachAttr for Stateful<W> {
  type Target = Stateful<W>;
  attr_widget_attach_impl!();
}

impl<W: RenderWidget> RenderWidget for AttrWidgetWrap<W> {
  type RO = W::RO;

  #[inline]
  fn create_render_object(&self) -> Self::RO { self.0.create_render_object() }

  #[inline]
  fn update_render_object(&self, object: &mut Self::RO, ctx: &mut UpdateCtx) {
    self.0.update_render_object(object, ctx)
  }
}

impl<W: CombinationWidget> CombinationWidget for AttrWidgetWrap<W> {
  #[inline]
  fn build(&self, ctx: BuildCtx<Self>) -> BoxedWidget { self.0.widget.build(ctx.cast_type()) }
}

macro get_attr($name: ident) {
  $name.as_attrs().and_then(Attributes::find)
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
