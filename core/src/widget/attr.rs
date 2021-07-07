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
use std::any::Any;

use std::collections::LinkedList;

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
  fn with_key<K: Into<Key> + 'static>(self, key: K) -> KeyDetect<Self::W>
  where
    Self: Sized,
  {
    KeyDetect::new(self, key)
  }

  /// Assign the type of mouse cursor, show when the mouse pointer is over this
  /// widget.
  #[inline]
  fn with_cursor(self, cursor: CursorIcon) -> Cursor<Self::W>
  where
    Self: Sized,
  {
    Cursor::new(cursor, self)
  }

  #[inline]
  fn with_theme(self, data: ThemeData) -> Theme<Self::W>
  where
    Self: Sized,
  {
    Theme::new(self, data)
  }

  /// Used to specify the event handler for the pointer down event, which is
  /// fired when the pointing device is initially pressed.
  fn on_pointer_down<F>(self, handler: F) -> PointerListener<Self::W>
  where
    Self: Sized,
    F: FnMut(&PointerEvent) + 'static,
  {
    let mut listener = PointerListener::from_widget(self);
    listener.listen_on(PointerEventType::Down, handler);
    listener
  }

  /// Used to specify the event handler for the pointer up event, which is
  /// fired when the all pressed pointing device is released.
  fn on_pointer_up<F>(self, handler: F) -> PointerListener<Self::W>
  where
    Self: Sized,
    F: FnMut(&PointerEvent) + 'static,
  {
    let mut listener = PointerListener::from_widget(self);
    listener.listen_on(PointerEventType::Up, handler);
    listener
  }

  /// Specify the event handler to process pointer move event.
  fn on_pointer_move<F>(self, handler: F) -> PointerListener<Self::W>
  where
    Self: Sized,
    F: FnMut(&PointerEvent) + 'static,
  {
    let mut listener = PointerListener::from_widget(self);
    listener.listen_on(PointerEventType::Move, handler);
    listener
  }

  /// Specify the event handler to process pointer tap event.
  fn on_tap<F>(self, handler: F) -> PointerListener<Self::W>
  where
    Self: Sized,
    F: FnMut(&PointerEvent) + 'static,
  {
    let mut listener = PointerListener::from_widget(self);
    listener.listen_on(PointerEventType::Tap, handler);
    listener
  }

  /// Specify the event handler to process pointer tap event.
  fn on_tap_times<F>(self, times: u8, mut handler: F) -> PointerListener<Self::W>
  where
    Self: Sized,
    F: FnMut(&PointerEvent) + 'static,
  {
    let pointer = PointerListener::from_widget(self);
    pointer
      .tap_times_observable(times)
      .subscribe(move |e| handler(&*e));
    pointer
  }

  /// Specify the event handler to process pointer cancel event.
  fn on_pointer_cancel<F>(self, handler: F) -> PointerListener<Self::W>
  where
    Self: Sized,
    F: FnMut(&PointerEvent) + 'static,
  {
    let mut listener = PointerListener::from_widget(self);
    listener.listen_on(PointerEventType::Cancel, handler);
    listener
  }

  /// specify the event handler when pointer enter this widget.
  fn on_pointer_enter<F>(self, handler: F) -> PointerListener<Self::W>
  where
    Self: Sized,
    F: FnMut(&PointerEvent) + 'static,
  {
    let mut listener = PointerListener::from_widget(self);
    listener.listen_on(PointerEventType::Enter, handler);
    listener
  }

  /// Specify the event handler when pointer leave this widget.
  fn on_pointer_leave<F>(self, handler: F) -> PointerListener<Self::W>
  where
    Self: Sized,
    F: FnMut(&PointerEvent) + 'static,
  {
    let mut listener = PointerListener::from_widget(self);
    listener.listen_on(PointerEventType::Leave, handler);
    listener
  }

  /// Assign whether the `widget` should automatically get focus when the window
  /// loads. Indicates the `widget` can be focused.
  #[inline]
  fn with_auto_focus(self, auto_focus: bool) -> FocusListener<Self::W>
  where
    Self: Sized,
  {
    FocusListener::from_widget(self, Some(auto_focus), None)
  }

  /// Assign where the widget participates in sequential keyboard navigation.
  /// Indicates the `widget` can be focused and
  #[inline]
  fn with_tab_index(self, tab_index: i16) -> FocusListener<Self::W>
  where
    Self: Sized,
  {
    FocusListener::from_widget(self, None, Some(tab_index))
  }

  /// Specify the event handler to process focus event. The focus event is
  /// raised when when the user sets focus on an element.
  fn on_focus<F>(self, handler: F) -> FocusListener<Self::W>
  where
    Self: Sized,
    F: FnMut(&FocusEvent) + 'static,
  {
    let focus = FocusListener::from_widget(self, None, None);
    focus.listen_on(FocusEventType::Focus, handler);
    focus
  }

  /// Specify the event handler to process blur event. The blur event is raised
  /// when an widget loses focus.
  fn on_blur<F>(self, handler: F) -> FocusListener<Self::W>
  where
    Self: Sized,
    F: FnMut(&FocusEvent) + 'static,
  {
    let focus = FocusListener::from_widget(self, None, None);
    focus.listen_on(FocusEventType::Blur, handler);
    focus
  }

  /// Specify the event handler to process focusin event.  The main difference
  /// between this event and blur is that focusin bubbles while blur does not.
  #[inline]
  fn on_focus_in<F>(self, handler: F) -> FocusListener<Self::W>
  where
    Self: Sized,
    F: FnMut(&FocusEvent) + 'static,
  {
    let focus = FocusListener::from_widget(self, None, None);
    focus.listen_on(FocusEventType::FocusIn, handler);
    focus
  }

  /// Specify the event handler to process focusout event. The main difference
  /// between this event and blur is that focusout bubbles while blur does not.
  #[inline]
  fn on_focus_out<F>(self, handler: F) -> FocusListener<Self::W>
  where
    Self: Sized,
    F: FnMut(&FocusEvent) + 'static,
  {
    let focus = FocusListener::from_widget(self, None, None);
    focus.listen_on(FocusEventType::FocusOut, handler);
    focus
  }

  /// Specify the event handler when keyboard press down.
  #[inline]
  fn on_key_down<F>(self, handler: F) -> KeyboardListener<Self::W>
  where
    Self: Sized,
    F: FnMut(&KeyboardEvent) + 'static,
  {
    let keyboard = KeyboardListener::from_widget(self);
    keyboard.listen_on(KeyboardEventType::KeyDown, handler);
    keyboard
  }

  /// Specify the event handler when a key is released.
  #[inline]
  fn on_key_up<F>(self, handler: F) -> KeyboardListener<Self::W>
  where
    Self: Sized,
    F: FnMut(&KeyboardEvent) + 'static,
  {
    let keyboard = KeyboardListener::from_widget(self);
    keyboard.listen_on(KeyboardEventType::KeyUp, handler);
    keyboard
  }

  /// Specify the event handler when received a unicode character.
  fn on_char<F>(self, mut handler: F) -> CharListener<Self::W>
  where
    Self: Sized,
    F: FnMut(&CharEvent) + 'static,
  {
    let widget = CharListener::from_widget(self);
    widget
      .event_observable()
      .subscribe(move |char_event| handler(&*char_event));
    widget
  }

  /// Specify the event handler when user moving a mouse wheel or similar input
  /// device.
  fn on_wheel<F>(self, mut handler: F) -> WheelListener<Self::W>
  where
    Self: Sized,
    F: FnMut(&WheelEvent) + 'static,
  {
    let widget = WheelListener::from_widget(self);
    widget
      .event_observable()
      .subscribe(move |wheel_event| handler(&*wheel_event));
    widget
  }

  /// Detach attributes from the widget, and return the specified attr `A` if
  /// have, other attributes and the origin widget attributes attached to.
  fn take_attr<A: Any>(self) -> (Option<A>, Option<Attrs>, Self::W);
}

#[derive(CombinationWidget, RenderWidget)]
pub struct AttrWidget<W, A: Any> {
  #[proxy]
  pub widget: W,
  pub major: A,
  pub others: Option<Attrs>,
}

impl<W: Widget, A: Any> Widget for AttrWidget<W, A> {
  #[inline]
  fn attrs_ref(&self) -> Option<AttrsRef> {
    Some(AttrsRef {
      major: &self.major,
      other_attrs: self.others.as_ref().map(|a| &a.0),
    })
  }

  #[inline]
  fn attrs_mut(&mut self) -> Option<AttrsMut> {
    Some(AttrsMut {
      major: &mut self.major,
      other_attrs: self.others.as_mut().map(|a| &mut a.0),
    })
  }
}

impl<W: Widget, A: Any> AttachAttr for AttrWidget<W, A> {
  type W = W;

  fn take_attr<M: Any>(self) -> (Option<M>, Option<Attrs>, Self::W) {
    let Self { widget, mut major, mut others } = self;
    let new_major = if let Some(m) = (&mut major as &mut dyn Any).downcast_mut::<M>() {
      let mut tmp = std::mem::MaybeUninit::<M>::uninit();
      unsafe {
        tmp.as_mut_ptr().copy_from(m as *const M, 1);
      }
      std::mem::forget(major);
      Some(unsafe { tmp.assume_init() })
    } else {
      let new_major = others.as_mut().and_then(|others| others.remove_attr::<M>());
      others
        .get_or_insert_with(Attrs::default)
        .front_push_attr(major);
      new_major
    };

    (new_major, others, widget)
  }
}

pub struct AttrsRef<'a> {
  major: &'a dyn Any,
  other_attrs: Option<&'a LinkedList<Box<dyn Any>>>,
}

pub struct AttrsMut<'a> {
  major: &'a mut dyn Any,
  other_attrs: Option<&'a mut LinkedList<Box<dyn Any>>>,
}

impl<'a> AttrsRef<'a> {
  pub fn find_attr<A: 'static>(self) -> Option<&'a A> {
    self.major.downcast_ref::<A>().or_else(|| {
      self
        .other_attrs
        .and_then(|attrs| attrs.iter().find_map(|attr| attr.downcast_ref::<A>()))
    })
  }
}

impl<'a> AttrsMut<'a> {
  pub fn find_attr_mut<A: 'static>(self) -> Option<&'a mut A> {
    let Self { major, other_attrs } = self;
    major.downcast_mut::<A>().or_else(move || {
      other_attrs.and_then(|attrs| attrs.iter_mut().find_map(|attr| attr.downcast_mut::<A>()))
    })
  }
}

impl<W: IntoStateful + Widget, A: Any> IntoStateful for AttrWidget<W, A> {
  type S = AttrWidget<W::S, A>;

  fn into_stateful(self) -> Self::S {
    let Self { widget, major, others } = self;

    let widget = widget.into_stateful();
    AttrWidget { widget, major, others }
  }
}

impl<W: Stateful, A: Any> Stateful for AttrWidget<W, A> {
  type RawWidget = W::RawWidget;
  #[inline]
  fn ref_cell(&self) -> StateRefCell<Self::RawWidget> { self.widget.ref_cell() }
}

impl<W, A: Any> std::ops::Deref for AttrWidget<W, A> {
  type Target = W;
  #[inline]
  fn deref(&self) -> &Self::Target { &self.widget }
}

impl<W, A: Any> std::ops::DerefMut for AttrWidget<W, A> {
  #[inline]
  fn deref_mut(&mut self) -> &mut Self::Target { &mut self.widget }
}

#[derive(Default)]
pub struct Attrs(LinkedList<Box<dyn Any>>);

impl Attrs {
  /// Remove the type `A` attribute out of the attributes.
  pub fn remove_attr<A: Any>(&mut self) -> Option<A> {
    let mut cursor = self.0.cursor_front_mut();

    while !cursor.current()?.is::<A>() {
      cursor.move_next();
    }

    cursor.remove_current().and_then(|mut any| {
      let attr = any.downcast_mut::<A>().unwrap();
      let tmp = unsafe { std::mem::transmute_copy(attr) };
      std::mem::forget(any);
      tmp
    })
  }

  pub fn front_push_attr<A: Any>(&mut self, attr: A) { self.0.push_front(Box::new(attr)); }

  pub fn find_attr<A: Any>(&self) -> Option<&A> { self.0.iter().find_map(|a| a.downcast_ref()) }

  pub fn find_attr_mut<A: Any>(&mut self) -> Option<&mut A> {
    self.0.iter_mut().find_map(|a| a.downcast_mut())
  }
}
