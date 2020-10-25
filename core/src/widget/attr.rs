use crate::prelude::*;
use rxrust::prelude::*;
use std::{any::Any, fmt::Debug, marker::PhantomData};

/// WidgetAttr is use to extend ability of a widget but not increase the widget
/// number. If a widget is not a combination widget and will not do layout or
/// paint, it should be consider as a WidgetAttr. Like the event listeners,
/// `KeyDetect`, `Stateful` and so on.
///
/// WidgetAttr attach the ability to a widget, if many `WidgetAttr` attached to
/// a same widget, they are organized like a linked list, an `WidgetAttr` hold
/// another `WidgetAttr` until the `WidgetAttr` hold a real widget.
///
/// ## Notice
/// When you implement a new `WidgetAttr`, you should remember a widget can only
/// attach one attr of same `WidgetAttr` type. If user attach it many times, you
/// should merge them.
#[derive(Debug)]
pub struct WidgetAttr<W: Widget, AttrData> {
  pub attr: AttrData,
  widget: BoxWidget,
  type_info: PhantomData<*const W>,
}

pub trait Attribute: Widget {
  fn widget(&self) -> &BoxWidget;
  fn widget_mut(&mut self) -> &mut BoxWidget;
}

pub enum AttrOrWidget<W: Widget, A> {
  Attr(WidgetAttr<W, A>),
  Widget(BoxWidget),
}

pub trait AttributeAttach: Widget {
  type HostWidget: Widget;

  /// Assign a key to the widget to help framework to track if two widget is a
  /// same widget in two frame.
  #[inline]
  fn with_key<K: Into<Key> + 'static>(self, key: K) -> KeyDetect<Self::HostWidget>
  where
    Self: Sized,
  {
    let key = key.into();
    self.attach_attr(key)
  }

  /// Assign the type of mouse cursor, show when the mouse pointer is over this
  /// widget.
  #[inline]
  fn with_cursor(self, cursor: CursorIcon) -> Cursor<Self::HostWidget>
  where
    Self: Sized,
  {
    Cursor::new(cursor, self)
  }

  /// Assign whether the `widget` should automatically get focus when the window
  /// loads. Indicates the `widget` can be focused.
  #[inline]
  fn with_auto_focus(self, auto_focus: bool) -> FocusListener<Self::HostWidget>
  where
    Self: Sized,
  {
    FocusListener::from_widget(self, Some(auto_focus), None)
  }

  /// Assign where the widget participates in sequential keyboard navigation.
  /// Indicates the `widget` can be focused and
  #[inline]
  fn with_tab_index(self, tab_index: i16) -> FocusListener<Self::HostWidget>
  where
    Self: Sized,
  {
    FocusListener::from_widget(self, None, Some(tab_index))
  }

  /// Convert a stateless widget to stateful, and will split to a stateful
  /// widget, and a `StateRef` which can be use to modify the states of the
  /// widget.
  #[inline]
  fn into_stateful(self) -> Stateful<Self::HostWidget>
  where
    Self: Sized,
  {
    self.unwrap_attr_or_else_with(|mut widget| {
      let attr = widget::stateful::StatefulAttr::new(&mut widget);
      (widget, attr)
    })
  }

  #[inline]
  fn with_theme(self, data: ThemeData) -> Theme<Self::HostWidget>
  where
    Self: Sized,
  {
    self.attach_attr(data)
  }

  /// Used to specify the event handler for the pointer down event, which is
  /// fired when the pointing device is initially pressed.
  #[inline]
  fn on_pointer_down<F>(self, handler: F) -> PointerListener<Self::HostWidget>
  where
    Self: Sized,
    F: FnMut(&PointerEvent) + 'static,
  {
    PointerListener::listen_on(self, PointerEventType::Down, handler)
  }

  /// Used to specify the event handler for the pointer up event, which is
  /// fired when the all pressed pointing device is released.
  #[inline]
  fn on_pointer_up<F>(self, handler: F) -> PointerListener<Self::HostWidget>
  where
    Self: Sized,
    F: FnMut(&PointerEvent) + 'static,
  {
    PointerListener::listen_on(self, PointerEventType::Up, handler)
  }

  /// Specify the event handler to process pointer move event.
  #[inline]
  fn on_pointer_move<F>(self, handler: F) -> PointerListener<Self::HostWidget>
  where
    Self: Sized,
    F: FnMut(&PointerEvent) + 'static,
  {
    PointerListener::listen_on(self, PointerEventType::Move, handler)
  }

  /// Specify the event handler to process pointer tap event.
  #[inline]
  fn on_tap<F>(self, handler: F) -> PointerListener<Self::HostWidget>
  where
    Self: Sized,
    F: FnMut(&PointerEvent) + 'static,
  {
    PointerListener::listen_on(self, PointerEventType::Tap, handler)
  }

  /// Specify the event handler to process pointer tap event.
  fn on_tap_times<F>(self, times: u8, mut handler: F) -> PointerListener<Self::HostWidget>
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
  #[inline]
  fn on_pointer_cancel<F>(self, handler: F) -> PointerListener<Self::HostWidget>
  where
    Self: Sized,
    F: FnMut(&PointerEvent) + 'static,
  {
    PointerListener::listen_on(self, PointerEventType::Cancel, handler)
  }

  /// specify the event handler when pointer enter this widget.
  #[inline]
  fn on_pointer_enter<F>(self, handler: F) -> PointerListener<Self::HostWidget>
  where
    Self: Sized,
    F: FnMut(&PointerEvent) + 'static,
  {
    PointerListener::listen_on(self, PointerEventType::Enter, handler)
  }

  /// Specify the event handler when pointer leave this widget.
  #[inline]
  fn on_pointer_leave<F>(self, handler: F) -> PointerListener<Self::HostWidget>
  where
    Self: Sized,
    F: FnMut(&PointerEvent) + 'static,
  {
    PointerListener::listen_on(self, PointerEventType::Leave, handler)
  }

  /// Specify the event handler to process focus event. The focus event is
  /// raised when when the user sets focus on an element.
  #[inline]
  fn on_focus<F>(self, handler: F) -> FocusListener<Self::HostWidget>
  where
    Self: Sized,
    F: FnMut(&FocusEvent) + 'static,
  {
    FocusListener::listen_on(self, FocusEventType::Focus, handler)
  }

  /// Specify the event handler to process blur event. The blur event is raised
  /// when an widget loses focus.
  #[inline]
  fn on_blur<F>(self, handler: F) -> FocusListener<Self::HostWidget>
  where
    Self: Sized,
    F: FnMut(&FocusEvent) + 'static,
  {
    FocusListener::listen_on(self, FocusEventType::Blur, handler)
  }

  /// Specify the event handler to process focusin event.  The main difference
  /// between this event and blur is that focusin bubbles while blur does not.
  #[inline]
  fn on_focus_in<F>(self, handler: F) -> FocusListener<Self::HostWidget>
  where
    Self: Sized,
    F: FnMut(&FocusEvent) + 'static,
  {
    FocusListener::listen_on(self, FocusEventType::FocusIn, handler)
  }

  /// Specify the event handler to process focusout event. The main difference
  /// between this event and blur is that focusout bubbles while blur does not.
  #[inline]
  fn on_focus_out<F>(self, handler: F) -> FocusListener<Self::HostWidget>
  where
    Self: Sized,
    F: FnMut(&FocusEvent) + 'static,
  {
    FocusListener::listen_on(self, FocusEventType::FocusOut, handler)
  }

  /// Specify the event handler when keyboard press down.
  #[inline]
  fn on_key_down<F>(self, handler: F) -> KeyboardListener<Self::HostWidget>
  where
    Self: Sized,
    F: FnMut(&KeyboardEvent) + 'static,
  {
    KeyboardListener::listen_on(self, KeyboardEventType::KeyDown, handler)
  }

  /// Specify the event handler when a key is released.
  #[inline]
  fn on_key_up<F>(self, handler: F) -> KeyboardListener<Self::HostWidget>
  where
    Self: Sized,
    F: FnMut(&KeyboardEvent) + 'static,
  {
    KeyboardListener::listen_on(self, KeyboardEventType::KeyUp, handler)
  }

  /// Specify the event handler when received a unicode character.
  #[inline]
  fn on_char<F>(self, mut handler: F) -> CharListener<Self::HostWidget>
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

  /// If this widget attached an `AttrData`, unwrap it, otherwise attach
  /// an attribute data computes from a closure..
  fn unwrap_attr_or_else<AttrData: 'static, F: FnOnce() -> AttrData>(
    self,
    f: F,
  ) -> WidgetAttr<Self::HostWidget, AttrData>
  where
    Self: Sized,
  {
    match pop_attr(self) {
      AttrOrWidget::Attr(attr) => attr,
      AttrOrWidget::Widget(widget) => WidgetAttr {
        widget,
        attr: f(),
        type_info: PhantomData,
      },
    }
  }

  /// If this widget attached an `AttrData`, unwrap it, otherwise attach
  /// `attr_data` on it.
  fn unwrap_attr_or<AttrData: 'static>(
    self,
    attr_data: AttrData,
  ) -> WidgetAttr<Self::HostWidget, AttrData>
  where
    Self: Sized,
  {
    match pop_attr(self) {
      AttrOrWidget::Attr(attr) => attr,
      AttrOrWidget::Widget(widget) => WidgetAttr {
        widget,
        attr: attr_data,
        type_info: PhantomData,
      },
    }
  }

  /// If this widget attached an `AttrData`, unwrap it, otherwise attach
  /// an attribute data to a new widget, both the widget and attribute data
  /// computes from a closure.
  fn unwrap_attr_or_else_with<AttrData: 'static, F: FnOnce(BoxWidget) -> (BoxWidget, AttrData)>(
    self,
    f: F,
  ) -> WidgetAttr<Self::HostWidget, AttrData>
  where
    Self: Sized,
  {
    match pop_attr(self) {
      AttrOrWidget::Attr(attr) => attr,
      AttrOrWidget::Widget(widget) => {
        let (widget, attr) = f(widget);
        WidgetAttr {
          widget,
          attr,
          type_info: PhantomData,
        }
      }
    }
  }

  /// Attach `attr_data` to this widget, If it's attached a same type attribute
  /// data, overwrite it.
  fn attach_attr<AttrData: 'static>(
    self,
    attr_data: AttrData,
  ) -> WidgetAttr<Self::HostWidget, AttrData>
  where
    Self: Sized,
  {
    match pop_attr(self) {
      AttrOrWidget::Attr(mut attr) => {
        attr.attr = attr_data;
        attr
      }
      AttrOrWidget::Widget(widget) => WidgetAttr {
        widget,
        attr: attr_data,
        type_info: PhantomData,
      },
    }
  }

  fn has_attr<AttrData: 'static>(&self) -> bool
  where
    Self: Sized,
  {
    let mut attr = self.as_attr();
    let mut first = true;
    while let Some(a) = attr {
      if first && a.as_any().is::<WidgetAttr<Self::HostWidget, AttrData>>() {
        return true;
      } else if a.as_any().is::<WidgetAttr<BoxWidget, AttrData>>() {
        return true;
      } else {
        first = false;
        attr = a.widget().as_attr();
      }
    }
    false
  }
}

/// If this widget is has the `AttrData` attribute, this method pop the
/// `AttrData` to the most outside, and return it, otherwise return a
/// `BoxWidget`
fn pop_attr<A: AttributeAttach, AttrData: 'static>(
  widget: A,
) -> AttrOrWidget<A::HostWidget, AttrData>
where
  A: Sized,
{
  let mut boxed = widget.box_it();
  // Safety: if we success copy the attribute, we will forget the origin object.
  if let Some((widget, attr)) = unsafe { copy_attr(&mut boxed) } {
    std::mem::forget(boxed);
    AttrOrWidget::Attr(WidgetAttr {
      attr,
      widget,
      type_info: PhantomData,
    })
  } else {
    let mut target = boxed.as_attr_mut();
    let mut attr = None;
    while let Some(attr_widget) = target.take() {
      // Safety: if we success copy the attribute, we will forget the origin object.
      if let Some((widget, a)) = unsafe { copy_attr(attr_widget.widget_mut()) } {
        let detached = std::mem::replace(attr_widget.widget_mut(), widget);
        std::mem::forget(detached);
        attr = Some(a);
        break;
      } else {
        target = attr_widget.widget_mut().as_attr_mut();
      }
    }

    if let Some(attr) = attr {
      AttrOrWidget::Attr(WidgetAttr {
        attr,
        widget: boxed,
        type_info: PhantomData,
      })
    } else {
      AttrOrWidget::Widget(boxed)
    }
  }
}

impl<W: Widget, AttrData: Any + Debug> Attribute for WidgetAttr<W, AttrData> {
  #[inline]
  fn widget(&self) -> &BoxWidget { &self.widget }

  #[inline]
  fn widget_mut(&mut self) -> &mut BoxWidget { &mut self.widget }
}

impl<W: Widget, Data: Any + Debug> Widget for WidgetAttr<W, Data> {
  fn classify(&self) -> WidgetClassify { self.widget.classify() }

  #[inline]
  fn classify_mut(&mut self) -> WidgetClassifyMut { self.widget.classify_mut() }

  #[inline]
  fn as_any(&self) -> &dyn Any { self }

  #[inline]
  fn as_any_mut(&mut self) -> &mut dyn Any { self }

  #[inline]
  fn as_attr(&self) -> Option<&dyn Attribute>
  where
    Self: Sized,
  {
    Some(self)
  }

  #[inline]
  fn as_attr_mut(&mut self) -> Option<&mut dyn Attribute>
  where
    Self: Sized,
  {
    Some(self)
  }

  fn box_it(self) -> BoxWidget
  where
    Self: Sized,
  {
    let erase_type: WidgetAttr<BoxWidget, Data> = WidgetAttr {
      widget: self.widget,
      attr: self.attr,
      type_info: PhantomData,
    };
    let widget: Box<dyn Widget> = Box::new(erase_type);
    widget.into()
  }
}

unsafe fn copy_attr<AttrData: 'static>(widget: &mut BoxWidget) -> Option<(BoxWidget, AttrData)> {
  if let Some(attr) = widget
    .as_any_mut()
    .downcast_mut::<WidgetAttr<BoxWidget, AttrData>>()
  {
    #[allow(invalid_value)]
    let mut tmp: WidgetAttr<BoxWidget, AttrData> = std::mem::MaybeUninit::uninit().assume_init();
    let to: *mut WidgetAttr<BoxWidget, AttrData> = &mut tmp;
    to.copy_from(attr, 1);

    Some((tmp.widget, tmp.attr))
  } else {
    None
  }
}

impl<W: Widget, Data: Debug + 'static> AttributeAttach for WidgetAttr<W, Data> {
  type HostWidget = W;
}

impl<W: Widget, Attr: Any + Debug> std::ops::Deref for WidgetAttr<W, Attr> {
  type Target = W;
  fn deref(&self) -> &Self::Target {
    let mut widget: &dyn Widget = self;
    while let Some(attr) = widget.as_attr() {
      widget = attr.widget();
    }
    debug_assert_eq!(
      widget.as_any().type_id(),
      std::any::TypeId::of::<Self::Target>()
    );
    widget
      .as_any()
      .downcast_ref::<Self::Target>()
      .expect("The type of widget should be equal to the `type_info`")
  }
}

impl<W: Widget, Attr: Any + Debug> std::ops::DerefMut for WidgetAttr<W, Attr> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    let mut widget = self as *mut dyn Widget;
    // Safety: the type info always hold the origin widget type.
    unsafe {
      while let Some(attr) = (&mut *widget).as_attr_mut() {
        widget = attr.widget_mut() as *mut dyn Widget;
      }

      debug_assert_eq!(
        (*widget).as_any().type_id(),
        std::any::TypeId::of::<Self::Target>()
      );

      (*widget)
        .as_any_mut()
        .downcast_mut::<Self::Target>()
        .expect("The type of widget should be equal to the `type_info`")
    }
  }
}
