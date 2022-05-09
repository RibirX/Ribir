# Full builtin fields list 

- padding : [`EdgeInsets`] 
 	 - set the padding area on all four sides of a widget.
- background : [`Brush`] 
 	 - specify the background of the widget box.
- border : [`Border`] 
 	 - specify the border of the widget which draw above the background
- radius : [`Radius`] 
 	 - specify how rounded the corners have of the widget.
- key : [`Key`] 
 	 - assign a key to widget, use for track if two widget is same widget in two frames.
- cursor : [`CursorIcon`] 
 	 - assign cursor to the widget.
- on_pointer_down : [`impl FnMut(& mut PointerEvent)`] 
 	 - specify the event handler for the pointer down event.
- on_pointer_up : [`impl FnMut(& mut PointerEvent)`] 
 	 - specify the event handler for the pointer up event.
- on_pointer_move : [`impl FnMut(& mut PointerEvent)`] 
 	 - specify the event handler for the pointer move event.
- on_tap : [`impl FnMut(& mut PointerEvent)`] 
 	 - specify the event handler for the pointer tap event.
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
- on_wheel : [`impl FnMut(& mut WheelEvent)`] 
 	 - specify the event handler when user moving a mouse wheel or similar input device.
- scrollable : [`Scrollable`] 
 	 - enumerate to describe which direction allow widget to scroll.
- margin : [`impl EdgeInsets`] 
 	 - expand space around widget wrapped.

[`EdgeInsets`]: prelude::EdgeInsets

[`Brush`]: prelude::Brush

[`Border`]: prelude::Border

[`Radius`]: prelude::Radius

[`Key`]: prelude::Key

[`CursorIcon`]: prelude::CursorIcon

[`impl FnMut(& mut PointerEvent)`]: prelude::impl FnMut(& mut PointerEvent)

[`impl FnMut(& mut PointerEvent)`]: prelude::impl FnMut(& mut PointerEvent)

[`impl FnMut(& mut PointerEvent)`]: prelude::impl FnMut(& mut PointerEvent)

[`impl FnMut(& mut PointerEvent)`]: prelude::impl FnMut(& mut PointerEvent)

[`impl FnMut(& mut PointerEvent)`]: prelude::impl FnMut(& mut PointerEvent)

[`impl FnMut(& mut PointerEvent)`]: prelude::impl FnMut(& mut PointerEvent)

[`impl FnMut(& mut PointerEvent)`]: prelude::impl FnMut(& mut PointerEvent)

[`bool`]: prelude::bool

[`i16`]: prelude::i16

[`impl FnMut(& mut FocusEvent)`]: prelude::impl FnMut(& mut FocusEvent)

[`impl FnMut(& mut FocusEvent)`]: prelude::impl FnMut(& mut FocusEvent)

[`impl FnMut(& mut FocusEvent)`]: prelude::impl FnMut(& mut FocusEvent)

[`impl FnMut(& mut FocusEvent)`]: prelude::impl FnMut(& mut FocusEvent)

[`impl FnMut(& mut KeyboardEvent)`]: prelude::impl FnMut(& mut KeyboardEvent)

[`impl FnMut(& mut KeyboardEvent)`]: prelude::impl FnMut(& mut KeyboardEvent)

[`impl FnMut(& mut WheelEvent)`]: prelude::impl FnMut(& mut WheelEvent)

[`Scrollable`]: prelude::Scrollable

[`impl EdgeInsets`]: prelude::impl EdgeInsets
