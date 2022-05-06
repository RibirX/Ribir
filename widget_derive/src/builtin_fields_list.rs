builtin! {
  Key {
    #[doc="assign a key to widget, use for track if two widget is same widget in two frames."]
    key: Key
  }
  // #[doc="assign cursor to the widget."]
  // cursor: "[`CursorIcon`](../ribir/widget/enum.CursorIcon.html)",
  // #[doc="assign theme to the widget."]
  // theme: "[`Theme`](../ribir/widget/struct.Theme.html)",
  // #[doc="Indicates whether the widget should automatically get focus when the window loads."]
  // auto_focus: "bool",
  // #[doc="indicates that widget can be focused, and where it participates in \
  //       sequential keyboard navigation (usually with the Tab key, hence the name."]
  // tab_index: "i16",

  // #listeners

  // #[doc="specify the event handler for the pointer down event."]
  // on_pointer_down: "FnMut(&[`PointerEvent`](../ribir/widget/events/struct.PointerEvent.html))",
  // #[doc="specify the event handler for the pointer up event."]
  // on_pointer_up: "FnMut(&[`PointerEvent`](../ribir/widget/events/struct.PointerEvent.html))",
  // #[doc="specify the event handler for the pointer move event."]
  // on_pointer_move: "FnMut(&[`PointerEvent`](../ribir/widget/events/struct.PointerEvent.html))",
  // #[doc="specify the event handler for the pointer tap event."]
  // on_tap: "FnMut(&[`PointerEvent`](../ribir/widget/events/struct.PointerEvent.html))",
  // #[doc="specify the event handler for processing the specified times tap."]
  // on_tap_times: "FnMut(&[`PointerEvent`](../ribir/widget/events/struct.PointerEvent.html))",
  // #[doc="specify the event handler to process pointer cancel event."]
  // on_pointer_cancel: "FnMut(&[`PointerEvent`](../ribir/widget/events/struct.PointerEvent.html))",
  // #[doc="specify the event handler when pointer enter this widget."]
  // on_pointer_enter: "FnMut(&[`PointerEvent`](../ribir/widget/events/struct.PointerEvent.html))",
  // #[doc="specify the event handler when pointer leave this widget."]
  // on_pointer_leave: "FnMut(&[`PointerEvent`](../ribir/widget/events/struct.PointerEvent.html))",

  
  // #[doc="specify the event handler to process focus event."]
  // on_focus: "FnMut(&[`FocusEvent`](../ribir/widget/events/type.FocusEvent.html))",
  // #[doc="specify the event handler to process blur event."]
  // on_blur: "FnMut(&[`FocusEvent`](../ribir/widget/events/type.FocusEvent.html))",
  // #[doc="specify the event handler to process focusin event."]
  // on_focus_in: "FnMut(&[`FocusEvent`](../ribir/widget/events/type.FocusEvent.html))",
  // #[doc="specify the event handler to process focusout event."]
  // on_focus_out: "FnMut(&[`FocusEvent`](../ribir/widget/events/type.FocusEvent.html))",
  // #[doc="specify the event handler when keyboard press down."]
  // on_key_down: "FnMut(&[`KeyboardEvent`](../ribir/widget/events/struct.KeyboardEvent.html))",
  // #[doc="specify the event handler when a key is released."]
  // on_key_up: "FnMut(&[`KeyboardEvent`](../ribir/widget/events/struct.KeyboardEvent.html))",
  // #[doc="specify the event handler when received a unicode character."]
  // on_char: "FnMut(&[`CharEvent`](../ribir/widget/events/struct.CharEvent.html))",
  // #[doc="specify the event handler when user moving a mouse wheel or similar input device."]
  // on_wheel: "FnMut(&[`WheelEvent`](../ribir/widget/events/struct.WheelEvent.html))",

  // #widget_wrap
  // // padding should always before margin, it widget have margin & padding both
  // // margin should wrap padding.

  Padding {
    #[doc="set the padding area on all four sides of a widget."]
    padding: EdgeInsets
  }
  // #[doc="expand space around widget wrapped."]
  // margin: "[`EdgeInsets`](../ribir/widget/struct.EdgeInsets.html)" -> Margin,
  // #[doc="specify the background of the widget box."]
  // background: "type which implement Into<[`FillStyle`](../ribir/widget/enum.FillStyle.html)>" -> BoxDecoration,
  // #[doc="specify the border of the widget which draw above the background"]
  // border: "[`Border`](ribir/widget/struct.Border.html)" -> BoxDecoration,
  // #[doc= "specify how rounded the corners have of the widget."]
  // radius: "[`BorderRadius`](../doc/canvas/layer/struct.BorderRadius.html)" -> BoxDecoration,
  // #[doc= "enumerate to describe which direction allow widget to scroll."]
  Scrollable {
    #[doc= "enumerate to describe which direction allow widget to scroll."]
    scrollable: Scrollable
  }
}
