use crate::prelude::*;
use std::{cell::Cell, rc::Rc};

/// `Cursor` is an attribute to assign an `cursor` to a widget.
#[derive(Declare, Debug)]
pub struct Cursor {
  #[declare(convert=custom, builtin, default)]
  pub cursor: Rc<Cell<CursorIcon>>,
}

impl ComposeChild for Cursor {
  type Child = Widget;
  fn compose_child(this: State<Self>, child: Self::Child) -> Widget {
    widget! {
      states {
        save_cursor: Stateful::new(None),
        this: this.into_readonly()
      }
      DynWidget {
        dyns: child,
        on_pointer_enter: move |e: &mut PointerEvent| {
          if e.point_type == PointerType::Mouse
            && e.mouse_buttons() == MouseButtons::empty()
          {
            let mut ctx = e.context();
            *save_cursor = ctx.stage_cursor_icon();
            ctx.set_cursor_icon(this.cursor.get());
          }
        },
        on_pointer_leave: move |e: &mut PointerEvent| {
          let cursor = save_cursor.unwrap_or_default();
          e.context().set_cursor_icon(cursor);
        }
      }
    }
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
  use crate::test::*;

  #[test]
  fn tree_down_up() {
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

    let mut wnd = Window::default_mock(row_tree, None);

    wnd.draw_frame();
    let tree = &mut wnd.widget_tree;

    let device_id = MockPointerId::zero();
    let dispatcher = &mut wnd.dispatcher;
    dispatcher.dispatch(
      WindowEvent::CursorMoved {
        device_id,
        position: DevicePoint::new(1, 1),
      },
      tree,
      1.,
    );
    assert_eq!(dispatcher.take_cursor_icon(), Some(CursorIcon::Help));

    let device_id = MockPointerId::zero();
    dispatcher.dispatch(
      WindowEvent::CursorMoved {
        device_id,
        position: DevicePoint::new(101, 1),
      },
      tree,
      1.,
    );
    assert_eq!(dispatcher.take_cursor_icon(), Some(CursorIcon::Hand));

    let device_id = MockPointerId::zero();
    dispatcher.dispatch(
      WindowEvent::CursorMoved {
        device_id,
        position: DevicePoint::new(201, 1),
      },
      tree,
      1.,
    );
    assert_eq!(dispatcher.take_cursor_icon(), Some(CursorIcon::AllScroll));

    let device_id = MockPointerId::zero();
    dispatcher.dispatch(
      WindowEvent::CursorMoved {
        device_id,
        position: DevicePoint::new(101, 1),
      },
      tree,
      1.,
    );
    assert_eq!(dispatcher.take_cursor_icon(), Some(CursorIcon::Hand));

    let device_id = MockPointerId::zero();
    dispatcher.dispatch(
      WindowEvent::CursorMoved {
        device_id,
        position: DevicePoint::new(1, 1),
      },
      tree,
      1.,
    );
    assert_eq!(dispatcher.take_cursor_icon(), Some(CursorIcon::Help));
  }
}
