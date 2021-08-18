use crate::prelude::*;
use std::{cell::Cell, rc::Rc};
use winit::window::CursorIcon;

/// `Cursor` is an attribute to assign an `cursor` to a widget.
#[derive(Debug)]
pub struct Cursor(Rc<Cell<CursorIcon>>);

pub fn cursor_attach<A: AttachAttr>(cursor: CursorIcon, widget: A) -> AttrWidget<A::W> {
  let mut w = widget.into_attr_widget();

  if let Some(c) = w.attrs.get_mut::<Cursor>() {
    c.0.set(cursor);
    w
  } else {
    let cursor = Rc::new(Cell::new(cursor));
    let c_cursor = cursor.clone();
    let mut w = w.on_pointer_move(move |e| {
      if e.point_type == PointerType::Mouse
        && e.buttons == MouseButtons::empty()
        && e.as_ref().window.borrow().updated_cursor().is_none()
      {
        e.as_ref().window.borrow_mut().set_cursor(c_cursor.get())
      }
    });
    w.attrs.insert(cursor);
    w
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::widget::window::{MockRawWindow, NoRenderWindow, RawWindow};
  use winit::event::{DeviceId, WindowEvent};

  fn submit_cursor(wnd: &mut NoRenderWindow) -> CursorIcon {
    let ptr = (&mut **wnd.raw_window.borrow_mut()) as *mut dyn RawWindow;
    #[allow(clippy::cast_ptr_alignment)]
    let mock_window = unsafe { &mut *(ptr as *mut MockRawWindow) };
    let cursor = mock_window.cursor.unwrap();
    mock_window.submit_cursor();
    cursor
  }

  #[test]
  fn tree_down_up() {
    let row_tree = SizedBox::expanded()
      .with_cursor(CursorIcon::AllScroll)
      .have(
        {
          Row::default()
            .with_cross_align(CrossAxisAlign::Start)
            .with_main_align(MainAxisAlign::Start)
            .push(
              SizedBox::from_size(Size::new(200., 200.))
                .with_cursor(CursorIcon::Hand)
                .have(
                  {
                    Row::default()
                      .with_cross_align(CrossAxisAlign::Start)
                      .with_main_align(MainAxisAlign::Start)
                      .push(
                        SizedBox::from_size(Size::new(100., 100.))
                          .with_cursor(CursorIcon::Help)
                          .box_it(),
                      )
                  }
                  .box_it(),
                )
                .box_it(),
            )
        }
        .box_it(),
      )
      .box_it();
    let mut wnd = NoRenderWindow::without_render(row_tree, Size::new(400., 400.));

    wnd.render_ready();

    let device_id = unsafe { DeviceId::dummy() };
    wnd.dispatcher.dispatch(WindowEvent::CursorMoved {
      device_id,
      position: (1, 1).into(),
      modifiers: ModifiersState::default(),
    });
    assert_eq!(submit_cursor(&mut wnd), CursorIcon::Help);

    let device_id = unsafe { DeviceId::dummy() };
    wnd.dispatcher.dispatch(WindowEvent::CursorMoved {
      device_id,
      position: (101, 1).into(),
      modifiers: ModifiersState::default(),
    });
    assert_eq!(submit_cursor(&mut wnd), CursorIcon::Hand);

    let device_id = unsafe { DeviceId::dummy() };
    wnd.dispatcher.dispatch(WindowEvent::CursorMoved {
      device_id,
      position: (201, 1).into(),
      modifiers: ModifiersState::default(),
    });
    assert_eq!(submit_cursor(&mut wnd), CursorIcon::AllScroll);

    let device_id = unsafe { DeviceId::dummy() };
    wnd.dispatcher.dispatch(WindowEvent::CursorMoved {
      device_id,
      position: (101, 1).into(),
      modifiers: ModifiersState::default(),
    });
    assert_eq!(submit_cursor(&mut wnd), CursorIcon::Hand);

    let device_id = unsafe { DeviceId::dummy() };
    wnd.dispatcher.dispatch(WindowEvent::CursorMoved {
      device_id,
      position: (1, 1).into(),
      modifiers: ModifiersState::default(),
    });
    assert_eq!(submit_cursor(&mut wnd), CursorIcon::Help);
  }
}
