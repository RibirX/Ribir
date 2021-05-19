use crate::prelude::*;
use std::{cell::Cell, rc::Rc};
use winit::window::CursorIcon;

#[derive(Debug)]
pub struct CursorAttr(Rc<Cell<CursorIcon>>);

/// `Cursor` is a widget inherit from another widget and assign an `cursor` to
/// it.
pub type Cursor<W> = AttrWidget<W, CursorAttr>;

impl<W: Widget> Cursor<W> {
  pub fn new<A: AttachAttr<W = W>>(cursor: CursorIcon, widget: A) -> Self {
    let (major, mut others, widget) = widget.take_attr();

    let major = major.map_or_else(
      || {
        let cursor = Rc::new(Cell::new(cursor));
        let c_cursor = cursor.clone();
        let other_attrs = others.get_or_insert_with(<_>::default);
        let mut attr: PointerAttr = other_attrs.remove_attr().unwrap_or_default();
        attr.listen_on(PointerEventType::Move, move |e| {
          if e.point_type == PointerType::Mouse
            && e.buttons == MouseButtons::empty()
            && e.as_ref().window.borrow().updated_cursor().is_none()
          {
            e.as_ref().window.borrow_mut().set_cursor(c_cursor.get())
          }
        });
        other_attrs.front_push_attr(attr);

        CursorAttr(cursor)
      },
      |c: CursorAttr| {
        c.0.set(cursor);
        c
      },
    );

    Cursor { major, widget, others }
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
    let widget_tree = SizedBox::expanded({
      Row::default()
        .with_cross_align(CrossAxisAlign::Start)
        .with_main_align(MainAxisAlign::Start)
        .push(
          SizedBox::from_size(Size::new(200., 200.), {
            Row::default()
              .with_cross_align(CrossAxisAlign::Start)
              .with_main_align(MainAxisAlign::Start)
              .push(SizedBox::empty_box(Size::new(100., 100.)).with_cursor(CursorIcon::Help))
          })
          .with_cursor(CursorIcon::Hand),
        )
    })
    .with_cursor(CursorIcon::AllScroll)
    .box_it();
    let mut wnd = NoRenderWindow::without_render(widget_tree, Size::new(400., 400.));

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
