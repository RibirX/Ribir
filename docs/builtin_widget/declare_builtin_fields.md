# Full builtin fields list 

- on_pointer_down : [`impl FnMut(& mut PointerEvent)`] 
 	 - specify the event handler for the pointer down event in bubble phase.
- on_pointer_down_capture : [`impl FnMut(& mut PointerEvent)`] 
 	 - specify the event handler for the pointer down event in capture phase.
- on_pointer_up : [`impl FnMut(& mut PointerEvent)`] 
 	 - specify the event handler for the pointer up event in bubble phase.
- on_pointer_up_capture : [`impl FnMut(& mut PointerEvent)`] 
 	 - specify the event handler for the pointer up event in capture phase.
- on_pointer_move : [`impl FnMut(& mut PointerEvent)`] 
 	 - specify the event handler for the pointer move event in bubble phase.
- on_pointer_move_capture : [`impl FnMut(& mut PointerEvent)`] 
 	 - specify the event handler for the pointer move event in capture phase.
- on_pointer_cancel : [`impl FnMut(& mut PointerEvent)`] 
 	 - specify the event handler to process pointer cancel event.
- on_pointer_enter : [`impl FnMut(& mut PointerEvent)`] 
 	 - specify the event handler when pointer enter this widget.
- on_pointer_leave : [`impl FnMut(& mut PointerEvent)`] 
 	 - specify the event handler when pointer leave this widget.
- on_tap : [`impl FnMut(& mut PointerEvent)`] 
 	 - specify the event handler for the pointer tap event in bubble phase.
- on_tap_capture : [`impl FnMut(& mut PointerEvent)`] 
 	 - specify the event handler for the pointer tap event in capture phase.
- on_double_tap : [`Box < dyn for < 'r > FnMut(& 'r mut PointerEvent) >`] 
 	 - specify the event handler for the pointer double tap event in bubble phase.
- on_double_tap_capture : [`Box < dyn for < 'r > FnMut(& 'r mut PointerEvent) >`] 
 	 - specify the event handler for the pointer double tap event in capture phase.
- on_triple_tap : [`Box < dyn for < 'r > FnMut(& 'r mut PointerEvent) >`] 
 	 - specify the event handler for the pointer triple tap event in bubble phase.
- on_triple_tap_capture : [`Box < dyn for < 'r > FnMut(& 'r mut PointerEvent) >`] 
 	 - specify the event handler for the pointer triple tap event in capture phase.
- on_x_times_tap : [`(usize, Box < dyn for < 'r > FnMut(& 'r mut PointerEvent) >)`] 
 	 - specify the event handler for the pointer `x` times tap event in bubble phase.
- on_x_times_tap_capture : [`(usize, Box < dyn for < 'r > FnMut(& 'r mut PointerEvent) >)`] 
 	 - specify the event handler for the pointer `x` times tap event in capture phase.
- auto_focus : [`bool`] 
 	 - Indicates whether the widget should automatically get focus when the window loads.
- tab_index : [`i16`] 
 	 - indicates that widget can be focused, and where it participates in sequential keyboard navigation (usually with the Tab key, hence the name.
- on_focus : [`impl FnMut(& mut FocusEvent)`] 
 	 - specify the event handler to process focus event.
- on_blur : [`impl FnMut(& mut FocusEvent)`] 
 	 - specify the event handler to process blur event.
- on_focus_in : [`impl FnMut(& mut FocusEvent)`] 
 	 - specify the event handler to process focusin event in bubble phase.
- on_focus_in_capture : [`impl FnMut(& mut FocusEvent)`] 
 	 - specify the event handler to process focusin event in capture phase.
- on_focus_out : [`impl FnMut(& mut FocusEvent)`] 
 	 - specify the event handler to process focusout event in bubble phase.
- on_focus_out_capture : [`impl FnMut(& mut FocusEvent)`] 
 	 - specify the event handler to process focusout event in capture phase.
- on_key_down_capture : [`impl FnMut(& mut KeyboardEvent)`] 
 	 - specify the event handler when keyboard press down in capture phase.
- on_key_down : [`impl FnMut(& mut KeyboardEvent)`] 
 	 - specify the event handler when keyboard press down in bubble phase.
- on_key_up : [`impl FnMut(& mut KeyboardEvent)`] 
 	 - specify the event handler when a key is released in bubble phase.
- on_key_up_capture : [`impl FnMut(& mut KeyboardEvent)`] 
 	 - specify the event handler when a key is released in capture phase.
- on_chars : [`impl FnMut(& mut CharsEvent)`] 
 	 - specify the event handler when received unicode characters in bubble phase.
- on_chars_capture : [`impl FnMut(& mut CharsEvent)`] 
 	 - specify the event handler when received unicode characters in capture phase.
- on_ime_pre_edit : [`impl FnMut(& mut ImePreEditEvent)`] 
 	 - specify the event handler when received unicode characters in ime pre edit bubble phase.
- on_ime_pre_edit_capture : [`impl FnMut(& mut ImePreEditEvent)`] 
 	 - specify the event handler when received unicode characters in ime pre edit capture phase.
- on_wheel : [`impl FnMut(& mut WheelEvent)`] 
 	 - specify the event handler when user moving a mouse wheel or similar input device in bubble phase.
- on_wheel_capture : [`impl FnMut(& mut WheelEvent)`] 
 	 - specify the event handler when user moving a mouse wheel or similar input device in capture phase.
- box_fit : [`BoxFit`] 
 	 -  set how its child should be resized to its box.
- background : [`Brush`] 
 	 - specify the background of the widget box.
- border : [`Border`] 
 	 - specify the border of the widget which draw above the background
- border_radius : [`Radius`] 
 	 - specify how rounded the corners have of the widget.
- padding : [`EdgeInsets`] 
 	 - set the padding area on all four sides of a widget.
- global_anchor : [`Anchor`] 
 	 - use to anchor child position, and the positioning used is relative to the window
- cursor : [`CursorIcon`] 
 	 - assign cursor to the widget.
- margin : [`impl EdgeInsets`] 
 	 - expand space around widget wrapped.
- scrollable : [`Scrollable`] 
 	 - enumerate to describe which direction allow widget to scroll.
- scroll_pos : [`Point`] 
 	 - specify the scroll position of this widget, also means that the host widget scrollable.
- transform : [`Transform`] 
 	 - A widget that applies a transformation its child. Doesn't change size, only apply painting
- h_align : [`HAlign`] 
 	 - describe how widget align to its box in x-axis.
- v_align : [`VAlign`] 
 	 - describe how widget align to its box in y-axis.
- anchor : [`Anchor`] 
 	 - use to anchor child position, and the positioning used is relative to the parent
- visible : [`bool`] 
 	 - Whether to show or hide a child
- opacity : [`f32`] 
 	 - Opacity is the degree to which content behind an element is hidden, and is the opposite of transparency.
- on_mounted : [`Box < dyn for < 'r > FnMut(LifeCycleCtx < 'r >, MountedType) >`] 
 	 - action perform after widget be added to the widget tree.
- on_disposed : [`Box < dyn for < 'r > FnMut(LifeCycleCtx < 'r >, DisposedType) >`] 
 	 - action perform after widget remove from widget tree.
- on_performed_layout : [`impl FnMut(& mut PointerEvent)`] 
 	 - specify the event handler for the pointer down event in bubble phase.
- lifecycle_stream : [`LifecycleSubject`] 
 	 - return the stream of lifecycle.
- delay_drop_until : [`bool`] 
 	 - The widget delay the drop of its child until the field delay_drop_until is false, but not affect its dispose event emit time. It's useful to ensure the disappear-animate display fully.

 - `fn lazy_host_id(& self) -> LazyWidgetId`
 	- Return the LazyWidgetId of the host widget, through which you can access the WidgetId after building.

 - `fn lazy_id(& self) -> LazyWidgetId`
 	- Return the LazyWidgetId of the external widget (wrapped with the built-in host), through which you can access the WidgetId after building.

 - `fn double_tap_stream(& self,) -> FilterMapOp < MutRefItemSubject < 'static,
PointerEvent, () >, impl FnMut(& mut PointerEvent) -> Option < & mut
PointerEvent >, & mut PointerEvent, >`
 	-  Return an observable stream of double tap event in bubble phase.

 - `fn triple_tap_stream(& self,) -> FilterMapOp < MutRefItemSubject < 'static,
PointerEvent, () >, impl FnMut(& mut PointerEvent) -> Option < & mut
PointerEvent >, & mut PointerEvent, >`
 	- Return an observable stream of tripe tap event in bubble phase.

 - `fn x_times_tap_stream(& self, x : usize, dur : Duration,) -> FilterMapOp <
MutRefItemSubject < 'static, PointerEvent, () >, impl
FnMut(& mut PointerEvent) -> Option < & mut PointerEvent >, & mut
PointerEvent, >`
 	-  Return an observable stream of x-tap event that user tapped 'x' times in the specify duration `dur` in bubble phase.

 - `fn tap_capture_stream(& self) -> MutRefItemSubject < 'static, PointerEvent, ()
>`
 	- return an observable stream of the pointer tap event in capture phase.

 - `fn double_tap_capture_stream(& self,) -> FilterMapOp < MutRefItemSubject <
'static, PointerEvent, () >, impl FnMut(& mut PointerEvent) -> Option < & mut
PointerEvent >, & mut PointerEvent, >`
 	- return an observable stream of double tap event in capture phase.

 - `fn triple_tap_capture_stream(& self,) -> FilterMapOp < MutRefItemSubject <
'static, PointerEvent, () >, impl FnMut(& mut PointerEvent) -> Option < & mut
PointerEvent >, & mut PointerEvent, >`
 	- Return an observable stream of tripe tap event in capture phase.

 - `fn x_times_tap_capture_stream(& self, x : usize, dur : Duration,) ->
FilterMapOp < MutRefItemSubject < 'static, PointerEvent, () >, impl
FnMut(& mut PointerEvent) -> Option < & mut PointerEvent >, & mut
PointerEvent, >`
 	-  Return an observable stream of x-tap event that user tapped 'x' times in the specify duration `dur` in capture phase.

 - `fn pointer_stream(& self) -> PointerSubject`
 	- return the stream include all pointer events.

 - `fn request_focus(& self)`
 	- request the this node to be focused.

 - `fn unfocus(& self)`
 	- removes the focus from this node.

 - `fn focus_stream(& self) -> MutRefItemSubject < 'static, FocusEvent, () >`
 	- Return the stream include all focus and blur events.

 - `fn focus_bubble_stream(& self) -> MutRefItemSubject < 'static, FocusEvent, ()
>`
 	- Return the stream include all focus in/out related events.

 - `fn has_focus(& self) -> bool`
 	- return if the widget has focus.

 - `fn keyboard_stream(& self) -> KeyboardSubject`
 	- return the stream include all keyboard events.

 - `fn chars_stream(& self) -> CharsSubject`
 	- return the stream include all chars events.

 - `fn wheel_stream(& self) -> MutRefItemSubject < 'static, WheelEvent, () >`
 	- return the stream include all wheel events.

 - `fn mouse_hover(& self) -> bool`
 	- return if the pointer is hover on the widget

 - `fn pointer_pressed(& self) -> bool`
 	- return if the widget is pressed

 - `fn layout_rect(& self) -> Rect`
 	- return the rect after layout of the widget

 - `fn layout_pos(& self) -> Point`
 	- return the position relative to parent after layout of the widget

 - `fn layout_size(& self) -> Size`
 	- return the size after layout of the widget

 - `fn layout_left(& self) -> f32`
 	- return the left position relative parent after layout of the widget

 - `fn layout_top(& self) -> f32`
 	- return the top position relative parent after layout of the widget

 - `fn layout_width(& self) -> f32`
 	- return the width after layout of the widget

 - `fn layout_height(& self) -> f32`
 	- return the height after layout of the widget

 - `fn scroll_view_size(& self) -> Size`
 	- return the scroll view of the scrollable widget

 - `fn scroll_content_size(& self) -> Size`
 	- return the content widget size of the scrollable widget.

 - `fn jump_to(& mut self, left_top : Point)`
 	- jump to the special position.
