# Full builtin fields list 

- performed_layout : [`Box < dyn for < 'r > FnMut(LifeCycleCtx < 'r >) >`] 
 	 - action perform after widget performed layout.
- pointer_down : [`impl FnMut(& mut PointerEvent)`] 
 	 - specify the event handler for the pointer down event.
- pointer_up : [`impl FnMut(& mut PointerEvent)`] 
 	 - specify the event handler for the pointer up event.
- pointer_move : [`impl FnMut(& mut PointerEvent)`] 
 	 - specify the event handler for the pointer move event.
- tap : [`impl FnMut(& mut PointerEvent)`] 
 	 - specify the event handler for the pointer tap event.
- double_tap : [`Box < dyn for < 'r > FnMut(& 'r mut PointerEvent) >`] 
 	 - specify the event handler for the pointer double tap event.
- tripe_tap : [`Box < dyn for < 'r > FnMut(& 'r mut PointerEvent) >`] 
 	 - specify the event handler for the pointer triple tap event.
- x_times_tap : [`(u8, Box < dyn for < 'r > FnMut(& 'r mut PointerEvent) >)`] 
 	 - specify the event handler for the pointer `x` times tap event.
- pointer_cancel : [`impl FnMut(& mut PointerEvent)`] 
 	 - specify the event handler to process pointer cancel event.
- pointer_enter : [`impl FnMut(& mut PointerEvent)`] 
 	 - specify the event handler when pointer enter this widget.
- pointer_leave : [`impl FnMut(& mut PointerEvent)`] 
 	 - specify the event handler when pointer leave this widget.
- auto_focus : [`bool`] 
 	 - Indicates whether the widget should automatically get focus when the window loads.
- tab_index : [`i16`] 
 	 - indicates that widget can be focused, and where it participates in sequential keyboard navigation (usually with the Tab key, hence the name.
- focus : [`impl FnMut(& mut FocusEvent)`] 
 	 - specify the event handler to process focus event.
- blur : [`impl FnMut(& mut FocusEvent)`] 
 	 - specify the event handler to process blur event.
- focus_in : [`impl FnMut(& mut FocusEvent)`] 
 	 - specify the event handler to process focusin event.
- focus_out : [`impl FnMut(& mut FocusEvent)`] 
 	 - specify the event handler to process focusout event.
- key_down : [`impl FnMut(& mut KeyboardEvent)`] 
 	 - specify the event handler when keyboard press down.
- key_up : [`impl FnMut(& mut KeyboardEvent)`] 
 	 - specify the event handler when a key is released.
- char : [`impl FnMut(& mut CharEvent)`] 
 	 - specify the event handler when received a unicode character.
- wheel : [`impl FnMut(& mut WheelEvent)`] 
 	 - specify the event handler when user moving a mouse wheel or similar input device.
- compose_styles : [`SmallVec < [ComposeStyleIdent ; 1] >`] 
 	 - compose_styles specify one or more `compose style` to a widget, `compose style` is an identify of a function defined in `Theme` which support convert a widget to another, in normal do some thing decoration in it.
- cursor : [`CursorIcon`] 
 	 - assign cursor to the widget.
- box_fit : [`BoxFit`] 
 	 -  set how its child should be resized to its box.
- padding : [`EdgeInsets`] 
 	 - set the padding area on all four sides of a widget.
- background : [`Brush`] 
 	 - specify the background of the widget box.
- border : [`Border`] 
 	 - specify the border of the widget which draw above the background
- radius : [`Radius`] 
 	 - specify how rounded the corners have of the widget.
- cursor : [`CursorIcon`] 
 	 - assign cursor to the widget.
- compose_styles : [`SmallVec < [ComposeStyleIdent ; 1] >`] 
 	 - compose_styles specify one or more `compose style` to a widget, `compose style` is an identify of a function defined in `Theme` which support convert a widget to another, in normal do some thing decoration in it.
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
- theme : [`Theme`] 
 	 - assign theme to the widget.
- mounted : [`Box < dyn for < 'r > FnMut(LifeCycleCtx < 'r >, MountedType) >`] 
 	 - action perform after widget be added to the widget tree.
- disposed : [`Box < dyn for < 'r > FnMut(LifeCycleCtx < 'r >, DisposedType) >`] 
 	 - action perform after widget remove from widget tree.
- key : [`Key`] 
 	 - assign a key to widget, use for track if two widget is same widget in two frames.

 - `fn has_focus(& self) -> bool`
 	- specify the element is focus now

 - `fn is_hover(& self) -> bool`
 	- specify the element is mouse hover now

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

 - `fn scroll_view(& self) -> Size`
 	- return the scroll view of the scrollable widget

 - `fn scroll_content(& self) -> Size`
 	- return the content widget size of the scrollable widget.
