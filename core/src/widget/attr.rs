//! Attributes is use to extend ability of a widget. Across attach attribute,
//! widget can be expanded much ability from the attributes. The attach means
//! the widget number will not increase after attributes attached and the origin
//! widget behavior will be kept.

//! Note that widget use the attribute ability across [`find_attr`][find],
//! widget can't hold two same type attribute, so if you implement a custom
//! attribute, you should merge or replace the same type attr if user try to
//! attach.
//!
//! Ribir provide many builtin attributes, and provide method to easy attach.
//! For example, we implement a custom widget and the builtin attributes can be
//! attached to it.

//! ```
//! # use ribir::prelude::*;
//! // implement a custom widget.
//! ##[derive(Widget)]
//! pub struct MyCheckbox;
//!
//! impl CombinationWidget for MyCheckbox {
//!   fn build(&self, ctx: &mut  BuildCtx) -> Box<dyn Widget>{
//!     Checkbox::from_theme(ctx.theme()).box_it()
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

//! #[derive(Widget, RenderWidget, CombinationWidget)]
//! pub struct Hello<W: Widget>(#[proxy] AttrWidget<W, HelloAttr>);
//!
//! pub struct HelloAttr;
//!
//! impl HelloAttr {
//!   pub fn hello(&self) {
//!     println!("Hello!");
//!   }
//! }
//!
//! impl<W: Widget> Hello<W> {
//!   pub fn new<A: AttachAttr<W = W>>(w: A) -> Self {
//!     // Take attr from a widget if it's have. We not use the old 'HelloAttr'
//!     // here.
//!     let (_, others, widget) = w.take_attr::<HelloAttr>();
//!     Hello (AttrWidget { widget, major: HelloAttr, others })
//!   }
//! }
//! let widget: Box<dyn Widget> = Hello::new(Text("".to_string())).box_it();
//! widget.find_attr::<HelloAttr>().unwrap().hello()
//! ```
//! [find]: crate::Widget::find_attr
//! [attr_impl] crate::widget::attr::WidgetAttr

use crate::prelude::*;
use rxrust::prelude::*;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::marker::PhantomData;
use widget::{focus_listen_on, keyboard_listen_on, pointer_listen_on};

/// `AttachAttr` provide the ability to attach the builtin attrs implemented by
/// Ribir. See the [module-level documentation][mod] for more details. When
/// derive `#[derive(Widget)]` `AttachAttr` will be also implemented.
///
/// [mod]: crate::widget::attr
pub trait AttachAttr {
  /// The widget the attribute attached to.
  type W: Widget;

  /// Assign a key to the widget to help framework to track if two widget is a
  /// same widget in two frame.
  #[inline]
  fn with_key<K: Into<Key> + 'static>(self, key: K) -> AttrWidget<Self::W>
  where
    Self: Sized,
  {
    self.attach(key)
  }

  /// Assign the type of mouse cursor, show when the mouse pointer is over this
  /// widget.
  #[inline]
  fn with_cursor(self, cursor: CursorIcon) -> AttrWidget<Self::W>
  where
    Self: Sized,
  {
    widget::cursor::cursor_attach(cursor, self)
  }

  #[inline]
  fn with_theme(self, data: Theme) -> AttrWidget<Self::W>
  where
    Self: Sized,
  {
    self.attach(data)
  }

  /// Used to specify the event handler for the pointer down event, which is
  /// fired when the pointing device is initially pressed.
  #[inline]
  fn on_pointer_down<F>(self, handler: F) -> AttrWidget<Self::W>
  where
    Self: Sized,
    F: FnMut(&PointerEvent) + 'static,
  {
    pointer_listen_on(self, PointerEventType::Down, handler)
  }

  /// Used to specify the event handler for the pointer up event, which is
  /// fired when the all pressed pointing device is released.
  #[inline]
  fn on_pointer_up<F>(self, handler: F) -> AttrWidget<Self::W>
  where
    Self: Sized,
    F: FnMut(&PointerEvent) + 'static,
  {
    pointer_listen_on(self, PointerEventType::Up, handler)
  }

  /// Specify the event handler to process pointer move event.
  #[inline]
  fn on_pointer_move<F>(self, handler: F) -> AttrWidget<Self::W>
  where
    Self: Sized,
    F: FnMut(&PointerEvent) + 'static,
  {
    pointer_listen_on(self, PointerEventType::Move, handler)
  }

  /// Specify the event handler to process pointer tap event.
  #[inline]
  fn on_tap<F>(self, handler: F) -> AttrWidget<Self::W>
  where
    Self: Sized,
    F: FnMut(&PointerEvent) + 'static,
  {
    pointer_listen_on(self, PointerEventType::Tap, handler)
  }

  /// Specify the event handler to process pointer tap event.
  fn on_tap_times<F>(self, times: u8, mut handler: F) -> AttrWidget<Self::W>
  where
    Self: Sized,
    F: FnMut(&PointerEvent) + 'static,
  {
    let w = self.into_attr_widget();
    w.attrs
      .entry::<PointerAttr>()
      .or_default()
      .tap_times_observable(times)
      .subscribe(move |e| handler(&*e));
    w
  }

  /// Specify the event handler to process pointer cancel event.
  #[inline]
  fn on_pointer_cancel<F>(self, handler: F) -> AttrWidget<Self::W>
  where
    Self: Sized,
    F: FnMut(&PointerEvent) + 'static,
  {
    pointer_listen_on(self, PointerEventType::Cancel, handler)
  }

  /// specify the event handler when pointer enter this widget.
  #[inline]
  fn on_pointer_enter<F>(self, handler: F) -> AttrWidget<Self::W>
  where
    Self: Sized,
    F: FnMut(&PointerEvent) + 'static,
  {
    pointer_listen_on(self, PointerEventType::Enter, handler)
  }

  /// Specify the event handler when pointer leave this widget.
  #[inline]
  fn on_pointer_leave<F>(self, handler: F) -> AttrWidget<Self::W>
  where
    Self: Sized,
    F: FnMut(&PointerEvent) + 'static,
  {
    pointer_listen_on(self, PointerEventType::Leave, handler)
  }

  /// Assign whether the `widget` should automatically get focus when the window
  /// loads. Indicates the `widget` can be focused.
  #[inline]
  fn with_auto_focus(self, auto_focus: bool) -> AttrWidget<Self::W>
  where
    Self: Sized,
  {
    let w = self.into_attr_widget();
    w.attrs.entry::<FocusAttr>().or_default().auto_focus = auto_focus;
    w
  }

  /// Assign where the widget participates in sequential keyboard navigation.
  /// Indicates the `widget` can be focused and
  #[inline]
  fn with_tab_index(self, tab_index: i16) -> AttrWidget<Self::W>
  where
    Self: Sized,
  {
    let w = self.into_attr_widget();
    w.attrs.entry::<FocusAttr>().or_default().tab_index = tab_index;
    w
  }

  /// Specify the event handler to process focus event. The focus event is
  /// raised when when the user sets focus on an element.
  #[inline]
  fn on_focus<F>(self, handler: F) -> AttrWidget<Self::W>
  where
    Self: Sized,
    F: FnMut(&FocusEvent) + 'static,
  {
    focus_listen_on(self, FocusEventType::Focus, handler)
  }

  /// Specify the event handler to process blur event. The blur event is raised
  /// when an widget loses focus.
  #[inline]
  fn on_blur<F>(self, handler: F) -> AttrWidget<Self::W>
  where
    Self: Sized,
    F: FnMut(&FocusEvent) + 'static,
  {
    focus_listen_on(self, FocusEventType::Blur, handler)
  }

  /// Specify the event handler to process focusin event.  The main difference
  /// between this event and blur is that focusin bubbles while blur does not.
  #[inline]
  fn on_focus_in<F>(self, handler: F) -> AttrWidget<Self::W>
  where
    Self: Sized,
    F: FnMut(&FocusEvent) + 'static,
  {
    focus_listen_on(self, FocusEventType::FocusIn, handler)
  }

  /// Specify the event handler to process focusout event. The main difference
  /// between this event and blur is that focusout bubbles while blur does not.
  #[inline]
  fn on_focus_out<F>(self, handler: F) -> AttrWidget<Self::W>
  where
    Self: Sized,
    F: FnMut(&FocusEvent) + 'static,
  {
    focus_listen_on(self, FocusEventType::FocusOut, handler)
  }

  /// Specify the event handler when keyboard press down.
  #[inline]
  fn on_key_down<F>(self, handler: F) -> AttrWidget<Self::W>
  where
    Self: Sized,
    F: FnMut(&KeyboardEvent) + 'static,
  {
    keyboard_listen_on(self, KeyboardEventType::KeyDown, handler)
  }

  /// Specify the event handler when a key is released.
  #[inline]
  fn on_key_up<F>(self, handler: F) -> AttrWidget<Self::W>
  where
    Self: Sized,
    F: FnMut(&KeyboardEvent) + 'static,
  {
    keyboard_listen_on(self, KeyboardEventType::KeyUp, handler)
  }

  /// Specify the event handler when received a unicode character.
  fn on_char<F>(self, mut handler: F) -> AttrWidget<Self::W>
  where
    Self: Sized,
    F: FnMut(&CharEvent) + 'static,
  {
    let w = self.into_attr_widget();

    // ensure focus attr attached, because a widget can accept char event base
    // on it can be focused.
    w.attrs.entry::<FocusAttr>().or_default();

    w.attrs
      .entry::<CharAttr>()
      .or_default()
      .event_observable()
      .subscribe(move |char_event| handler(&*char_event));
    w
  }

  /// Specify the event handler when user moving a mouse wheel or similar input
  /// device.
  fn on_wheel<F>(self, mut handler: F) -> AttrWidget<Self::W>
  where
    Self: Sized,
    F: FnMut(&WheelEvent) + 'static,
  {
    let w = self.into_attr_widget();

    w.attrs
      .entry::<WheelAttr>()
      .or_default()
      .event_observable()
      .subscribe(move |wheel_event| handler(&*wheel_event));
    w
  }

  /// Attach the attr to the widget, if it's already attached an `A` type attr
  /// replace it.
  #[inline]
  fn attach<A: Any>(self, attr: A) -> AttrWidget<Self::W>
  where
    Self: Sized,
  {
    let w = self.into_attr_widget();
    w.attrs.insert(attr);
    w
  }

  /// convert widget to `AttrWidget` which support to attach attributes in it.
  fn into_attr_widget(self) -> AttrWidget<Self::W>;
}

pub struct AttrWidget<W> {
  pub widget: W,
  pub attrs: Attributes,
}

impl<W: Widget> Widget for AttrWidget<W> {}

impl<W: CombinationWidget> CombinationWidget for AttrWidget<W> {
  #[inline]
  fn build(&self, ctx: &mut BuildCtx) -> BoxedWidget { self.widget.build(ctx) }

  #[inline]
  fn get_attrs(&self) -> Option<&Attributes> { Some(&self.attrs) }
}

impl<W: CloneStates> CloneStates for AttrWidget<W> {
  type States = W::States;
  #[inline]
  fn clone_states(&self) -> Self::States { self.widget.clone_states() }
}

impl<W: RenderWidget> RenderWidget for AttrWidget<W> {
  type RO = W::RO;
  #[inline]
  fn create_render_object(&self) -> Self::RO { self.widget.create_render_object() }
}

impl<W: Widget> AttachAttr for AttrWidget<W> {
  type W = W;

  #[inline]
  fn into_attr_widget(self) -> AttrWidget<Self::W> { self }
}

impl<W: SingleChildWidget> SingleChildWidget for AttrWidget<W> {}
impl<W: MultiChildWidget> MultiChildWidget for AttrWidget<W> {}

impl<W: IntoStateful + Widget> IntoStateful for AttrWidget<W> {
  type S = AttrWidget<W::S>;

  fn into_stateful(self) -> Self::S {
    let Self { widget, attrs } = self;

    let widget = widget.into_stateful();
    AttrWidget { widget, attrs }
  }
}

impl<W: Stateful> Stateful for AttrWidget<W> {
  type RawWidget = W::RawWidget;
  #[inline]
  fn ref_cell(&self) -> StateRefCell<Self::RawWidget> { self.widget.ref_cell() }
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

  pub fn get_mut<A: Any>(&mut self) -> Option<&mut A> {
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
