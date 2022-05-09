builtin! {
  Padding {
    #[doc="set the padding area on all four sides of a widget."]
    padding: EdgeInsets
  }

  BoxDecoration {
    #[doc="specify the background of the widget box."]
    background: Brush,
    #[doc="specify the border of the widget which draw above the background"]
    border: Border,
    #[doc= "specify how rounded the corners have of the widget."]
    radius: Radius,
  }

  Key {
    #[doc="assign a key to widget, use for track if two widget is same widget in two frames."]
    key: Key
  }

  Cursor {
    #[doc="assign cursor to the widget."]
    cursor: CursorIcon
  }

  // theme: "[`Theme`](../ribir/widget/struct.Theme.html)",


  PointerDownListener {
    #[doc="specify the event handler for the pointer down event."]
    on_pointer_down: impl FnMut(&mut PointerEvent),
  }

  PointerUpListener {
    #[doc="specify the event handler for the pointer up event."]
    on_pointer_up: impl FnMut(&mut PointerEvent),
  }

  PointerMoveListener {
    #[doc="specify the event handler for the pointer move event."]
    on_pointer_move: impl FnMut(&mut PointerEvent),
  }

  TapListener {
    #[doc="specify the event handler for the pointer tap event."]
    on_tap: impl FnMut(&mut PointerEvent),
  }

  PointerCancelListener {
    #[doc="specify the event handler to process pointer cancel event."]
    on_pointer_cancel: impl FnMut(&mut PointerEvent),
  }

  PointerEnterListener {
    #[doc="specify the event handler when pointer enter this widget."]
    on_pointer_enter: impl FnMut(&mut PointerEvent),
  }

  PointerLeaveListener {
    #[doc="specify the event handler when pointer leave this widget."]
    on_pointer_leave: impl FnMut(&mut PointerEvent),
  }

  // #[doc="specify the event handler for processing the specified times tap."]
  // on_tap_times: "FnMut(&[`PointerEvent`](../ribir/widget/events/struct.PointerEvent.html))",


  FocusListener {
    #[doc="Indicates whether the widget should automatically get focus when the window loads."]
    auto_focus: bool,
    #[doc="indicates that widget can be focused, and where it participates in \
          sequential keyboard navigation (usually with the Tab key, hence the name."]
    tab_index: i16,
    #[doc="specify the event handler to process focus event."]
    on_focus: impl FnMut(&mut FocusEvent),
    #[doc="specify the event handler to process blur event."]
    on_blur: impl FnMut(&mut FocusEvent),
    #[doc="specify the event handler to process focusin event."]
    on_focus_in: impl FnMut(&mut FocusEvent),
    #[doc="specify the event handler to process focusout event."]
    on_focus_out: impl FnMut(&mut FocusEvent),
  }

  KeyDownListener {
    #[doc="specify the event handler when keyboard press down."]
    on_key_down: impl FnMut(&mut KeyboardEvent),
  }
  KeyUpListener {
    #[doc="specify the event handler when a key is released."]
    on_key_up: impl FnMut(&mut KeyboardEvent),
  }
  // #[doc="specify the event handler when received a unicode character."]
  // on_char: "FnMut(&[`CharEvent`](../ribir/widget/events/struct.CharEvent.html))",

  WheelListener {
    #[doc="specify the event handler when user moving a mouse wheel or similar input device."]
    on_wheel: impl FnMut(&mut WheelEvent),
  }

  ScrollableWidget {
    #[doc= "enumerate to describe which direction allow widget to scroll."]
    scrollable: Scrollable
  }

  Margin {
    #[doc="expand space around widget wrapped."]
    margin: impl EdgeInsets,
  }
}
