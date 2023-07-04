use crate::prelude::*;
use std::{cell::Cell, rc::Rc};
use winit::window::CursorIcon;

/// `Cursor` is an attribute to assign an `cursor` to a widget.

#[derive(Declare, Debug, Declare2)]
pub struct Cursor {
  #[declare(convert=custom, builtin, default)]
  pub cursor: Rc<Cell<CursorIcon>>,
}

impl ComposeChild for Cursor {
  type Child = Widget;
  fn compose_child(this: State<Self>, child: Self::Child) -> Widget {
    widget! {
      states {
        save_cursor: Stateful::new(CursorIcon::Default),
        this: this.into_readonly()
      }
      DynWidget {
        dyns: child,
        on_pointer_enter: move |e: &mut PointerEvent| {
          if e.point_type == PointerType::Mouse
            && e.mouse_buttons() == MouseButtons::empty()
          {
            let wnd = e.window();
            *save_cursor = wnd.get_cursor();
            wnd.set_cursor(this.cursor.get());
          }
        },
        on_pointer_leave: move |e: &mut PointerEvent| {
          e.window().set_cursor(*save_cursor);
        }
      }
    }
    .into()
  }
}

pub trait IntoCursorIcon {
  fn into_cursor_icon(self) -> Rc<Cell<CursorIcon>>;
}

impl IntoCursorIcon for Rc<Cell<CursorIcon>> {
  #[inline]
  fn into_cursor_icon(self) -> Rc<Cell<CursorIcon>> { self }
}

impl IntoCursorIcon for CursorIcon {
  #[inline]
  fn into_cursor_icon(self) -> Rc<Cell<CursorIcon>> { Rc::new(Cell::new(self)) }
}

impl CursorDeclarer {
  #[inline]
  pub fn cursor<C: IntoCursorIcon>(mut self, icon: C) -> Self {
    self.cursor = Some(icon.into_cursor_icon());
    self
  }
}

impl Cursor {
  #[inline]
  pub fn set_declare_cursor<C: IntoCursorIcon>(&mut self, icon: C) {
    self.cursor = icon.into_cursor_icon();
  }
}

impl Cursor {
  #[inline]
  pub fn icon(&self) -> CursorIcon { self.cursor.get() }

  #[inline]
  pub fn set_icon(&self, icon: CursorIcon) { self.cursor.set(icon) }

  #[inline]
  pub fn new_icon(icon: CursorIcon) -> Rc<Cell<CursorIcon>> { Rc::new(Cell::new(icon)) }
}

impl Default for Cursor {
  #[inline]
  fn default() -> Self {
    Cursor {
      cursor: Rc::new(Cell::new(CursorIcon::Default)),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_helper::*;
  use winit::event::{DeviceId, WindowEvent};

  #[test]
  fn tree_down_up() {
    let _guard = unsafe { AppCtx::new_lock_scope() };

    let row_tree = widget! {
      MockBox {
        size: Size::new(f32::INFINITY, f32::INFINITY),
        cursor: CursorIcon::AllScroll,
        MockMulti{
          MockBox {
            size: Size::new(200., 200.),
            cursor: CursorIcon::Hand,
            MockBox {
              size:  Size::new(100., 100.),
              cursor: CursorIcon::Help,
            }
          }
        }
      }
    };

    let mut wnd = TestWindow::new(row_tree);

    wnd.draw_frame();
    let device_id = unsafe { DeviceId::dummy() };
    wnd.dispatcher.borrow_mut().dispatch(
      WindowEvent::CursorMoved {
        device_id,
        position: (1f64, 1.).into(),
        modifiers: ModifiersState::default(),
      },
      1.,
    );
    wnd.emit_events();
    assert_eq!(wnd.get_cursor(), CursorIcon::Help);

    let device_id = unsafe { DeviceId::dummy() };
    wnd.dispatcher.borrow_mut().dispatch(
      WindowEvent::CursorMoved {
        device_id,
        position: (101f64, 1.).into(),
        modifiers: ModifiersState::default(),
      },
      1.,
    );
    wnd.emit_events();
    assert_eq!(wnd.get_cursor(), CursorIcon::Hand);

    let device_id = unsafe { DeviceId::dummy() };
    wnd.dispatcher.borrow_mut().dispatch(
      WindowEvent::CursorMoved {
        device_id,
        position: (201f64, 1.).into(),
        modifiers: ModifiersState::default(),
      },
      1.,
    );
    wnd.emit_events();
    assert_eq!(wnd.get_cursor(), CursorIcon::AllScroll);

    let device_id = unsafe { DeviceId::dummy() };
    wnd.dispatcher.borrow_mut().dispatch(
      WindowEvent::CursorMoved {
        device_id,
        position: (101f64, 1.).into(),
        modifiers: ModifiersState::default(),
      },
      1.,
    );
    wnd.emit_events();
    assert_eq!(wnd.get_cursor(), CursorIcon::Hand);

    let device_id = unsafe { DeviceId::dummy() };
    wnd.dispatcher.borrow_mut().dispatch(
      WindowEvent::CursorMoved {
        device_id,
        position: (1f64, 1.).into(),
        modifiers: ModifiersState::default(),
      },
      1.,
    );
    wnd.emit_events();
    assert_eq!(wnd.get_cursor(), CursorIcon::Help);
  }
}
