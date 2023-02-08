builtin! {
  PerformedLayoutListener {
    #[doc="action perform after widget performed layout."]
    on_performed_layout: Box<dyn for<'r> FnMut(LifeCycleCtx<'r>)>,
  }

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

  DoubleTapListener {
    #[doc="specify the event handler for the pointer double tap event."]
    on_double_tap: Box<dyn for<'r> FnMut(&'r mut PointerEvent)>,
  }

  TripleTapListener {
    #[doc="specify the event handler for the pointer triple tap event."]
    on_tripe_tap: Box<dyn for<'r> FnMut(&'r mut PointerEvent)>,
  }

  XTimesTapListener {
    #[doc="specify the event handler for the pointer `x` times tap event."]
    on_x_times_tap: (u8, Box<dyn for<'r> FnMut(&'r mut PointerEvent)>),
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

  FocusNode {
    #[doc="Indicates whether the widget should automatically get focus when the window loads."]
    auto_focus: bool,
    #[doc="indicates that widget can be focused, and where it participates in \
          sequential keyboard navigation (usually with the Tab key, hence the name."]
    tab_index: i16,
  }

  RequestFocus{
    #[doc="request the this node to be focused."]
    fn request_focus(&self),
    #[doc="removes the focus from this node."]
    fn unfocus(&self),
  }

  FocusListener {
    #[doc="specify the event handler to process focus event."]
    on_focus: impl FnMut(&mut FocusEvent),
  }

  BlurListener {
    #[doc="specify the event handler to process blur event."]
    on_blur: impl FnMut(&mut FocusEvent),
  }

  FocusInListener {
    #[doc="specify the event handler to process focusin event."]
    on_focus_in: impl FnMut(&mut FocusEvent),
  }

  FocusOutListener{
    #[doc="specify the event handler to process focusout event."]
    on_focus_out: impl FnMut(&mut FocusEvent),
  }

  HasFocus {
    #[doc="return if the widget has focus."]
    fn has_focus(&self) -> bool,
  }

  KeyDownListener {
    #[doc="specify the event handler when keyboard press down."]
    on_key_down: impl FnMut(&mut KeyboardEvent),
  }

  KeyUpListener {
    #[doc="specify the event handler when a key is released."]
    on_key_up: impl FnMut(&mut KeyboardEvent),
  }

  CharListener {
    #[doc="specify the event handler when received a unicode character."]
    on_char: impl FnMut(&mut CharEvent)
  }

  WheelListener {
    #[doc="specify the event handler when user moving a mouse wheel or similar input device."]
    on_wheel: impl FnMut(&mut WheelEvent),
  }
  MouseHover {
    #[doc="return if the pointer is hover on the widget"]
    fn mouse_hover(&self) -> bool,
  }

  PointerPressed {
    #[doc="return if the widget is pressed"]
    fn pointer_pressed(&self) -> bool,
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
    border_radius: Radius,
  }

  LayoutBox {
    #[doc= "return the rect after layout of the widget"]
    fn layout_rect(&self) -> Rect,
    #[doc= "return the position relative to parent after layout of the widget"]
    fn layout_pos(&self) -> Point,
    #[doc= "return the size after layout of the widget"]
    fn layout_size(&self) -> Size,
    #[doc= "return the left position relative parent after layout of the widget"]
    fn layout_left(&self) -> f32,
    #[doc= "return the top position relative parent after layout of the widget"]
    fn layout_top(&self) -> f32,
    #[doc= "return the width after layout of the widget"]
    fn layout_width(&self) -> f32,
    #[doc= "return the height after layout of the widget"]
    fn layout_height(&self) -> f32,
  }

  Cursor {
    #[doc="assign cursor to the widget."]
    cursor: CursorIcon
  }

  Margin {
    #[doc="expand space around widget wrapped."]
    margin: impl EdgeInsets,
  }

  ScrollableWidget {
    #[doc= "enumerate to describe which direction allow widget to scroll."]
    scrollable: Scrollable,
    #[doc= "specify the scroll position of this widget, also means that the host widget scrollable."]
    scroll_pos: Point,
    #[doc= "return the scroll view of the scrollable widget"]
    fn scroll_view_size(&self) -> Size,
    #[doc= "return the content widget size of the scrollable widget."]
    fn scroll_content_size(&self) -> Size,
    #[doc= "jump to the special position."]
    fn jump_to(&mut self, left_top: Point)
  }

  TransformWidget {
    #[doc="A widget that applies a transformation its child. Doesn't change size, only apply painting"]
    transform: Transform
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

  Visibility {
    #[doc="Whether to show or hide a child"]
    visible: bool
  }

  Opacity {
    #[doc="Opacity is the degree to which content behind an element is hidden, and is the opposite of transparency."]
    opacity: f32
  }

  MountedListener {
    #[doc="action perform after widget be added to the widget tree."]
    on_mounted: Box<dyn for<'r> FnMut(LifeCycleCtx<'r>, MountedType)>,
  }

  DisposedListener {
    #[doc="action perform after widget remove from widget tree."]
    on_disposed: Box<dyn for<'r> FnMut(LifeCycleCtx<'r>, DisposedType)>,
  }
}
