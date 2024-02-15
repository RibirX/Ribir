builtin! {
  BuiltinObj {
    #[doc = "Return the LazyWidgetId of the host widget, through which you can access the WidgetId after building."]
    fn lazy_host_id(&self) -> LazyWidgetId,
    #[doc = "Return the LazyWidgetId of the external widget (wrapped with the built-in host), through which you can access the WidgetId after building."]
    fn lazy_id(&self) -> LazyWidgetId,
  }
  MixBuiltin {
    #[doc="Indicates whether the widget should automatically get focus when the window loads."]
    auto_focus: bool,
    #[doc="indicates that widget can be focused, and where it participates in \
          sequential keyboard navigation (usually with the Tab key, hence the name."]
    tab_index: i16,
    #[doc = "action perform after any event received."]
    on_event: impl FnMut(&mut Event),
    #[doc="action perform after widget be added to the widget tree."]
    on_mounted: Box<dyn for<'r> FnMut(LifeCycleCtx<'r>, MountedType)>,
    #[doc="action perform after widget remove from widget tree."]
    on_disposed: Box<dyn for<'r> FnMut(LifeCycleCtx<'r>, DisposedType)>,
    #[doc="specify the event handler for the pointer down event in bubble phase."]
    on_performed_layout: impl FnMut(&mut PointerEvent),
    #[doc="return the stream of lifecycle."]
    lifecycle_stream: LifecycleSubject,
    #[doc="specify the event handler for the pointer down event in bubble phase."]
    on_pointer_down: impl FnMut(&mut PointerEvent),
    #[doc="specify the event handler for the pointer down event in capture phase."]
    on_pointer_down_capture: impl FnMut(&mut PointerEvent),
    #[doc="specify the event handler for the pointer up event in bubble phase."]
    on_pointer_up: impl FnMut(&mut PointerEvent),
    #[doc="specify the event handler for the pointer up event in capture phase."]
    on_pointer_up_capture: impl FnMut(&mut PointerEvent),
    #[doc="specify the event handler for the pointer move event in bubble phase."]
    on_pointer_move: impl FnMut(&mut PointerEvent),
    #[doc="specify the event handler for the pointer move event in capture phase."]
    on_pointer_move_capture: impl FnMut(&mut PointerEvent),
    #[doc="specify the event handler to process pointer cancel event."]
    on_pointer_cancel: impl FnMut(&mut PointerEvent),
    #[doc="specify the event handler when pointer enter this widget."]
    on_pointer_enter: impl FnMut(&mut PointerEvent),
    #[doc="specify the event handler when pointer leave this widget."]
    on_pointer_leave: impl FnMut(&mut PointerEvent),
    #[doc="specify the event handler for the pointer tap event in bubble phase."]
    on_tap: impl FnMut(&mut PointerEvent),
    #[doc="specify the event handler for the pointer tap event in capture phase."]
    on_tap_capture: impl FnMut(&mut PointerEvent),
    #[doc="specify the event handler for the pointer double tap event in bubble phase."]
    on_double_tap: Box<dyn for<'r> FnMut(&'r mut PointerEvent)>,
    #[doc="specify the event handler for the pointer double tap event in capture phase."]
    on_double_tap_capture: Box<dyn for<'r> FnMut(&'r mut PointerEvent)>,
    #[doc="specify the event handler for the pointer triple tap event in bubble phase."]
    on_triple_tap: Box<dyn for<'r> FnMut(&'r mut PointerEvent)>,
    #[doc="specify the event handler for the pointer triple tap event in capture phase."]
    on_triple_tap_capture: Box<dyn for<'r> FnMut(&'r mut PointerEvent)>,
    #[doc="specify the event handler for the pointer `x` times tap event in bubble phase."]
    on_x_times_tap: (usize, Box<dyn for<'r> FnMut(&'r mut PointerEvent)>),
    #[doc="specify the event handler for the pointer `x` times tap event in capture phase."]
    on_x_times_tap_capture: (usize, Box<dyn for<'r> FnMut(&'r mut PointerEvent)>),
    #[doc="specify the event handler when received unicode characters in ime pre edit bubble phase."]
    on_ime_pre_edit: impl FnMut(&mut ImePreEditEvent),
    #[doc="specify the event handler when received unicode characters in ime pre edit capture phase."]
    on_ime_pre_edit_capture: impl FnMut(&mut ImePreEditEvent),
    #[doc="specify the event handler when user moving a mouse wheel or similar input device in bubble phase."]
    on_wheel: impl FnMut(&mut WheelEvent),
    #[doc="specify the event handler when user moving a mouse wheel or similar input device in capture phase."]
    on_wheel_capture: impl FnMut(&mut WheelEvent),
    #[doc="specify the event handler when received unicode characters in bubble phase."]
    on_chars: impl FnMut(&mut CharsEvent),
    #[doc="specify the event handler when received unicode characters in capture phase."]
    on_chars_capture: impl FnMut(&mut CharsEvent),
    #[doc="specify the event handler when keyboard press down in capture phase."]
    on_key_down_capture: impl FnMut(&mut KeyboardEvent),
    #[doc="specify the event handler when keyboard press down in bubble phase."]
    on_key_down: impl FnMut(&mut KeyboardEvent),
    #[doc="specify the event handler when a key is released in bubble phase."]
    on_key_up: impl FnMut(&mut KeyboardEvent),
    #[doc="specify the event handler when a key is released in capture phase."]
    on_key_up_capture: impl FnMut(&mut KeyboardEvent),
    #[doc="specify the event handler to process focus event."]
    on_focus: impl FnMut(&mut FocusEvent),
    #[doc="specify the event handler to process blur event."]
    on_blur: impl FnMut(&mut FocusEvent),
    #[doc="specify the event handler to process focusin event in bubble phase."]
    on_focus_in: impl FnMut(&mut FocusEvent),
    #[doc="specify the event handler to process focusin event in capture phase."]
    on_focus_in_capture: impl FnMut(&mut FocusEvent),
    #[doc="specify the event handler to process focusout event in bubble phase."]
    on_focus_out: impl FnMut(&mut FocusEvent),
    #[doc="specify the event handler to process focusout event in capture phase."]
    on_focus_out_capture: impl FnMut(&mut FocusEvent),
    #[doc="return the stream of the event."]
    fn subject(&self) -> EventSubject,
  }

  RequestFocus{
    #[doc="request the this node to be focused."]
    fn request_focus(&self),
    #[doc="removes the focus from this node."]
    fn unfocus(&self),
  }

  HasFocus {
    #[doc="return if the widget has focus."]
    fn has_focus(&self) -> bool,
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

  BoxDecoration {
    #[doc="specify the background of the widget box."]
    background: Brush,
    #[doc="specify the border of the widget which draw above the background"]
    border: Border,
    #[doc= "specify how rounded the corners have of the widget."]
    border_radius: Radius,
  }

  Padding {
    #[doc="set the padding area on all four sides of a widget."]
    padding: EdgeInsets
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

  GlobalAnchor {
    #[doc= "use to anchor child position, and the positioning used is relative to the window"]
    global_anchor: Anchor,
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

  RelativeAnchor {
    #[doc="use to anchor child position, and the positioning used is relative to the parent"]
    anchor: Anchor,
  }

  Visibility {
    #[doc="Whether to show or hide a child"]
    visible: bool
  }

  Opacity {
    #[doc="Opacity is the degree to which content behind an element is hidden, and is the opposite of transparency."]
    opacity: f32
  }

  DelayDrop {
    #[doc= "The widget delay the drop of its child until the field delay_drop_until is false, but not affect its dispose event emit time. It's useful to ensure the disappear-animate display fully."]
    delay_drop_until: bool,
  }
}
