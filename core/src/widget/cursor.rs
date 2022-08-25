use crate::prelude::*;
use std::{cell::Cell, rc::Rc};
use winit::window::CursorIcon;

/// `Cursor` is an attribute to assign an `cursor` to a widget.

#[derive(Declare, Debug)]
pub struct Cursor {
  #[declare(convert=custom, builtin, default)]
  pub cursor: Rc<Cell<CursorIcon>>,
}

impl ComposeSingleChild for Cursor {
  fn compose_single_child(this: StateWidget<Self>, child: Widget, _: &mut BuildCtx) -> Widget {
    widget_try_track! {
      try_track { this }
      ExprWidget {
        expr: child,
        on_pointer_move: move |e: &mut PointerEvent| {

          if e.point_type == PointerType::Mouse
            && e.mouse_buttons() == MouseButtons::empty()
          {
            let mut ctx = e.context();
            if ctx.stage_cursor_icon().is_none () {
              ctx.set_cursor_icon(this.cursor.get());
            }
          }
        },
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

impl CursorBuilder {
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
  use winit::event::{DeviceId, WindowEvent};

  #[test]
  fn tree_down_up() {
    let row_tree = widget! {
      SizedBox {
        size: Size::new(f32::INFINITY, f32::INFINITY),
        cursor: CursorIcon::AllScroll,
        Row{
          align_items: Align::Start,
          justify_content: JustifyContent::Start,
          SizedBox {
            size: Size::new(200., 200.),
            cursor: CursorIcon::Hand,
            Row {
              align_items: Align::Start,
              justify_content: JustifyContent::Start,
              SizedBox {
                size:  Size::new(100., 100.),
                cursor: CursorIcon::Help,
              }
            }
          }
        }
      }
    };

    let mut wnd = Window::without_render(row_tree, Size::new(400., 400.));

    wnd.draw_frame();
    let tree = &mut wnd.widget_tree;

    let device_id = unsafe { DeviceId::dummy() };
    let dispatcher = &mut wnd.dispatcher;
    dispatcher.dispatch(
      WindowEvent::CursorMoved {
        device_id,
        position: (1f64, 1.).into(),
        modifiers: ModifiersState::default(),
      },
      tree,
      1.,
    );
    assert_eq!(dispatcher.take_cursor_icon(), Some(CursorIcon::Help));

    let device_id = unsafe { DeviceId::dummy() };
    dispatcher.dispatch(
      WindowEvent::CursorMoved {
        device_id,
        position: (101f64, 1.).into(),
        modifiers: ModifiersState::default(),
      },
      tree,
      1.,
    );
    assert_eq!(dispatcher.take_cursor_icon(), Some(CursorIcon::Hand));

    let device_id = unsafe { DeviceId::dummy() };
    dispatcher.dispatch(
      WindowEvent::CursorMoved {
        device_id,
        position: (201f64, 1.).into(),
        modifiers: ModifiersState::default(),
      },
      tree,
      1.,
    );
    assert_eq!(dispatcher.take_cursor_icon(), Some(CursorIcon::AllScroll));

    let device_id = unsafe { DeviceId::dummy() };
    dispatcher.dispatch(
      WindowEvent::CursorMoved {
        device_id,
        position: (101f64, 1.).into(),
        modifiers: ModifiersState::default(),
      },
      tree,
      1.,
    );
    assert_eq!(dispatcher.take_cursor_icon(), Some(CursorIcon::Hand));

    let device_id = unsafe { DeviceId::dummy() };
    dispatcher.dispatch(
      WindowEvent::CursorMoved {
        device_id,
        position: (1f64, 1.).into(),
        modifiers: ModifiersState::default(),
      },
      tree,
      1.,
    );
    assert_eq!(dispatcher.take_cursor_icon(), Some(CursorIcon::Help));
  }
}
