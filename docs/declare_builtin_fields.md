# Full builtin fields list 

- key : [`Key`] 
 	 - assign a key to widget, use for track if two widget is same widget in two frames.
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
- on_wheel : [`FnMut(& mut WheelEvent)`] 
 	 - specify the event handler when user moving a mouse wheel or similar input device.
- padding : [`EdgeInsets`] 
 	 - set the padding area on all four sides of a widget.
- scrollable : [`Scrollable`] 
 	 - enumerate to describe which direction allow widget to scroll.

[`Key`]: prelude::Key

[`impl FnMut(& mut PointerEvent)`]: prelude::impl FnMut(& mut PointerEvent)

[`impl FnMut(& mut PointerEvent)`]: prelude::impl FnMut(& mut PointerEvent)

[`impl FnMut(& mut PointerEvent)`]: prelude::impl FnMut(& mut PointerEvent)

[`impl FnMut(& mut PointerEvent)`]: prelude::impl FnMut(& mut PointerEvent)

[`impl FnMut(& mut PointerEvent)`]: prelude::impl FnMut(& mut PointerEvent)

[`impl FnMut(& mut PointerEvent)`]: prelude::impl FnMut(& mut PointerEvent)

[`impl FnMut(& mut PointerEvent)`]: prelude::impl FnMut(& mut PointerEvent)

[`FnMut(& mut WheelEvent)`]: prelude::FnMut(& mut WheelEvent)

[`EdgeInsets`]: prelude::EdgeInsets

[`Scrollable`]: prelude::Scrollable
