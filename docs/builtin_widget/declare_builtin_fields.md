# Full builtin fields list 

- on_performed_layout : [`Box < dyn for < 'r > FnMut(LifeCycleCtx < 'r >) >`] 
 	 - action perform after widget performed layout.
- performed_layout_stream : [`LifecycleSubject`] 
 	 - return an observable stream of the performed layout event
- on_pointer_down : [`impl FnMut(& mut PointerEvent)`] 
 	 - specify the event handler for the pointer down event.
- on_pointer_up : [`impl FnMut(& mut PointerEvent)`] 
 	 - specify the event handler for the pointer up event.
- on_pointer_move : [`impl FnMut(& mut PointerEvent)`] 
 	 - specify the event handler for the pointer move event.
- on_tap : [`impl FnMut(& mut PointerEvent)`] 
 	 - specify the event handler for the pointer tap event.
- on_double_tap : [`Box < dyn for < 'r > FnMut(& 'r mut PointerEvent) >`] 
 	 - specify the event handler for the pointer double tap event.
- on_tripe_tap : [`Box < dyn for < 'r > FnMut(& 'r mut PointerEvent) >`] 
 	 - specify the event handler for the pointer triple tap event.
- on_x_times_tap : [`(usize, Box < dyn for < 'r > FnMut(& 'r mut PointerEvent) >)`] 
 	 - specify the event handler for the pointer `x` times tap event.
- on_pointer_cancel : [`impl FnMut(& mut PointerEvent)`] 
 	 - specify the event handler to process pointer cancel event.
- on_pointer_enter : [`impl FnMut(& mut PointerEvent)`] 
 	 - specify the event handler when pointer enter this widget.
- on_pointer_leave : [`impl FnMut(& mut PointerEvent)`] 
 	 - specify the event handler when pointer leave this widget.
- auto_focus : [`bool`] 
 	 - Indicates whether the widget should automatically get focus when the window loads.
- tab_index : [`i16`] 
 	 - indicates that widget can be focused, and where it participates in sequential keyboard navigation (usually with the Tab key, hence the name.
- on_focus : [`impl FnMut(& mut FocusEvent)`] 
 	 - specify the event handler to process focus event.
- on_blur : [`impl FnMut(& mut FocusEvent)`] 
 	 - specify the event handler to process blur event.
- on_focus_in : [`impl FnMut(& mut FocusEvent)`] 
 	 - specify the event handler to process focusin event.
- on_focus_out : [`impl FnMut(& mut FocusEvent)`] 
 	 - specify the event handler to process focusout event.
- on_key_down : [`impl FnMut(& mut KeyboardEvent)`] 
 	 - specify the event handler when keyboard press down.
- on_key_up : [`impl FnMut(& mut KeyboardEvent)`] 
 	 - specify the event handler when a key is released.
- on_chars : [`impl FnMut(& mut CharsEvent)`] 
 	 - specify the event handler when received unicode characters.
- on_wheel : [`impl FnMut(& mut WheelEvent)`] 
 	 - specify the event handler when user moving a mouse wheel or similar input device.
- box_fit : [`BoxFit`] 
 	 -  set how its child should be resized to its box.
- padding : [`EdgeInsets`] 
 	 - set the padding area on all four sides of a widget.
- background : [`Brush`] 
 	 - specify the background of the widget box.
- border : [`Border`] 
 	 - specify the border of the widget which draw above the background
- border_radius : [`Radius`] 
 	 - specify how rounded the corners have of the widget.
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
- left_anchor : [`PositionUnit`] 
 	 - use to anchor child constraints with the left edge of parent widget.
- right_anchor : [`PositionUnit`] 
 	 - use to anchor child constraints with the right edge of parent widget.
- top_anchor : [`PositionUnit`] 
 	 - use to anchor child constraints with the top edge of parent widget
- bottom_anchor : [`PositionUnit`] 
 	 - use to anchor child constraints with the bottom edge of parent widget.
- visible : [`bool`] 
 	 - Whether to show or hide a child
- opacity : [`f32`] 
 	 - Opacity is the degree to which content behind an element is hidden, and is the opposite of transparency.
- on_mounted : [`Box < dyn for < 'r > FnMut(LifeCycleCtx < 'r >, MountedType) >`] 
 	 - action perform after widget be added to the widget tree.
- mounted_stream : [`LifecycleSubject`] 
 	 - return an observable stream of the widget mounted event
- on_disposed : [`Box < dyn for < 'r > FnMut(LifeCycleCtx < 'r >, DisposedType) >`] 
 	 - action perform after widget remove from widget tree.
- disposed_stream : [`LifecycleSubject`] 
 	 - return an observable stream of the widget disposed event
- delay_drop_until : [`bool`] 
 	 - The widget delay the drop of its child until the field delay_drop_until is false, but not affect its dispose event emit time. It's useful to ensure the disappear-animate display fully.

 - `fn pointer_down_stream(& self) -> MutRefItemSubject < 'static, PointerEvent,
() >`
 	- return an observable stream of the pointer down event

 - `fn pointer_up_stream(& self) -> MutRefItemSubject < 'static, PointerEvent, ()
>`
 	- return an observable stream of the pointer up event

 - `fn pointer_move_stream(& self) -> MutRefItemSubject < 'static, PointerEvent,
() >`
 	- return an observable stream of the pointer move event

 - `fn tap_stream(& self) -> MutRefItemSubject < 'static, PointerEvent, () >`
 	- return an observable stream of the pointer tap event

 - `fn double_tap_stream(& self,) -> FilterMapOp < MutRefItemSubject < 'static,
PointerEvent, () >, impl FnMut(& mut PointerEvent) -> Option < & mut
PointerEvent >, & mut PointerEvent, >`
 	-  Return an observable stream of double tap event

 - `fn triple_tap_stream(& self,) -> FilterMapOp < MutRefItemSubject < 'static,
PointerEvent, () >, impl FnMut(& mut PointerEvent) -> Option < & mut
PointerEvent >, & mut PointerEvent, >`
 	- Return an observable stream of tripe tap event

 - `fn x_times_tap_stream(& self, x : usize, dur : Duration,) -> FilterMapOp <
MutRefItemSubject < 'static, PointerEvent, () >, impl
FnMut(& mut PointerEvent) -> Option < & mut PointerEvent >, & mut
PointerEvent, >`
 	-  Return an observable stream of x-tap event that user tapped 'x' times in the specify duration `dur`.

 - `fn pointer_cancel_stream(& self) -> MutRefItemSubject < 'static, PointerEvent,
() >`
 	- return an observable stream of the pointer cancel event

 - `fn pointer_enter_stream(& self) -> MutRefItemSubject < 'static, PointerEvent,
() >`
 	- return an observable stream of the pointer enter event

 - `fn pointer_leave_stream(& self) -> MutRefItemSubject < 'static, PointerEvent,
() >`
 	- return an observable stream of the pointer leave event

 - `fn request_focus(& self)`
 	- request the this node to be focused.

 - `fn unfocus(& self)`
 	- removes the focus from this node.

 - `fn focus_stream(& self) -> MutRefItemSubject < 'static, FocusEvent, () >`
 	- return an observable stream of the pointer focus event

 - `fn blur_stream(& self) -> MutRefItemSubject < 'static, FocusEvent, () >`
 	- return an observable stream of the pointer blur event

 - `fn focus_in_stream(& self) -> MutRefItemSubject < 'static, FocusEvent, () >`
 	- return an observable stream of the pointer focus-in event

 - `fn focus_out_stream(& self) -> MutRefItemSubject < 'static, FocusEvent, () >`
 	- return an observable stream of the pointer focus-out event

 - `fn has_focus(& self) -> bool`
 	- return if the widget has focus.

 - `fn key_down_stream(& self) -> MutRefItemSubject < 'static, KeyboardEvent, () >`
 	- return an observable stream of the key down event

 - `fn key_up_stream(& self) -> MutRefItemSubject < 'static, KeyboardEvent, () >`
 	- return an observable stream of the key up event

 - `fn chars_stream(& self) -> MutRefItemSubject < 'static, CharsEvent, () >`
 	- return an observable stream of the char event

 - `fn key_down_stream(& self) -> MutRefItemSubject < 'static, WheelEvent, () >`
 	- return an observable stream of the wheel event

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
