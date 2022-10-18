builtin! {
  KeyWidget {
    #[doc="assign a key to widget, use for track if two widget is same widget in two frames."]
    key: Key
  }

  PerformedLayoutListener {
    #[doc="action perform after widget performed layout."]
    performed_layout: Box<dyn for<'r> FnMut(LifeCycleCtx<'r>)>,
  }

  MountedListener {
    #[doc="action perform after widget be added to the widget tree."]
    mounted: Box<dyn for<'r> FnMut(LifeCycleCtx<'r>, MountedType)>,
  }

  DisposedListener {
    #[doc="action perform after widget remove from widget tree."]
    disposed: Box<dyn for<'r> FnMut(LifeCycleCtx<'r>)>,
  }

  PointerDownListener {
    #[doc="specify the event handler for the pointer down event."]
    pointer_down: impl FnMut(&mut PointerEvent),
  }

  PointerUpListener {
    #[doc="specify the event handler for the pointer up event."]
    pointer_up: impl FnMut(&mut PointerEvent),
  }

  PointerMoveListener {
    #[doc="specify the event handler for the pointer move event."]
    pointer_move: impl FnMut(&mut PointerEvent),
  }

  TapListener {
    #[doc="specify the event handler for the pointer tap event."]
    tap: impl FnMut(&mut PointerEvent),
  }

  DoubleTapListener {
    #[doc="specify the event handler for the pointer double tap event."]
    double_tap: Box<dyn for<'r> FnMut(&'r mut PointerEvent)>,
  }

  TripleTapListener {
    #[doc="specify the event handler for the pointer triple tap event."]
    tripe_tap: Box<dyn for<'r> FnMut(&'r mut PointerEvent)>,
  }

  XTimesTapListener {
    #[doc="specify the event handler for the pointer `x` times tap event."]
    x_times_tap: (u8, Box<dyn for<'r> FnMut(&'r mut PointerEvent)>),
  }

  PointerCancelListener {
    #[doc="specify the event handler to process pointer cancel event."]
    pointer_cancel: impl FnMut(&mut PointerEvent),
  }

  PointerEnterListener {
    #[doc="specify the event handler when pointer enter this widget."]
    pointer_enter: impl FnMut(&mut PointerEvent),
  }

  PointerLeaveListener {
    #[doc="specify the event handler when pointer leave this widget."]
    pointer_leave: impl FnMut(&mut PointerEvent),
  }

  FocusListener {
    #[doc="Indicates whether the widget should automatically get focus when the window loads."]
    auto_focus: bool,
    #[doc="indicates that widget can be focused, and where it participates in \
          sequential keyboard navigation (usually with the Tab key, hence the name."]
    tab_index: i16,
    #[doc="specify the event handler to process focus event."]
    focus: impl FnMut(&mut FocusEvent),
    #[doc="specify the event handler to process blur event."]
    blur: impl FnMut(&mut FocusEvent),
    #[doc="specify the event handler to process focusin event."]
    focus_in: impl FnMut(&mut FocusEvent),
    #[doc="specify the event handler to process focusout event."]
    focus_out: impl FnMut(&mut FocusEvent),
  }

  KeyDownListener {
    #[doc="specify the event handler when keyboard press down."]
    key_down: impl FnMut(&mut KeyboardEvent),
  }

  KeyUpListener {
    #[doc="specify the event handler when a key is released."]
    key_up: impl FnMut(&mut KeyboardEvent),
  }

  CharListener {
    #[doc="specify the event handler when received a unicode character."]
    char: impl FnMut(&mut CharEvent)
  }

  WheelListener {
    #[doc="specify the event handler when user moving a mouse wheel or similar input device."]
    wheel: impl FnMut(&mut WheelEvent),
  }

  Cursor {
    #[doc="assign cursor to the widget."]
    cursor: CursorIcon
  }

  ThemeWidget {
    #[doc="assign theme to the widget."]
    theme: Theme
  }

  FittedBox {
    #[doc=" set how its child should be resized to its box."]
    box_fit: BoxFit,
  }

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

  Margin {
    #[doc="expand space around widget wrapped."]
    margin: impl EdgeInsets,
  }

  HAlignWidget {
    #[doc="describe how widget align to its box in x-axis."]
    h_align: HAlign,
  }

  VAlignWidget {
    #[doc="describe how widget align to its box in y-axis."]
    v_align: VAlign,
  }


  LeftAnchor {
    #[doc="use to anchor child constraints with the left edge of parent widget."]
    left_anchor: PositionUnit,
  }

  RightAnchor {
    #[doc="use to anchor child constraints with the right edge of parent widget."]
    right_anchor: PositionUnit,
  }

  TopAnchor {
    #[doc="use to anchor child constraints with the top edge of parent widget"]
    top_anchor: PositionUnit,
  }

  BottomAnchor {
    #[doc="use to anchor child constraints with the bottom edge of parent widget."]
    bottom_anchor: PositionUnit,
  }

  ScrollableWidget {
    #[doc= "enumerate to describe which direction allow widget to scroll."]
    scrollable: Scrollable
  }

  TransformWidget {
    #[doc="A widget that applies a transformation its child. Doesn't change size, only apply painting"]
    transform: Transform
  }

  Visibility {
    #[doc="Whether to show or hide a child"]
    visible: bool
  }

  Opacity {
    #[doc="Opacity is the degree to which content behind an element is hidden, and is the opposite of transparency."]
    opacity: f32
  }
}
