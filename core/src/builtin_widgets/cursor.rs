use crate::prelude::*;
use winit::window::CursorIcon;

/// `Cursor` is an attribute to assign an `cursor` to a widget.

#[derive(Declare, Default, Debug, Declare2)]
pub struct Cursor {
  #[declare(builtin, default)]
  pub cursor: CursorIcon,
}

impl ComposeChild for Cursor {
  type Child = Widget;
  fn compose_child(this: State<Self>, child: Self::Child) -> Widget {
    fn_widget! {
      let mut save_cursor = Stateful::new(CursorIcon::Default);
      @$child {
        on_pointer_enter: move |e: &mut PointerEvent| {
          if e.point_type == PointerType::Mouse
            && e.mouse_buttons() == MouseButtons::empty()
          {
            let wnd = e.window();
            *$save_cursor.write() = wnd.get_cursor();
            wnd.set_cursor($this.get_cursor());
          }
        },
        on_pointer_leave: move |e: &mut PointerEvent| {
          e.window().set_cursor(*$save_cursor);
        }
      }
    }
    .into()
  }
}

impl Cursor {
  fn get_cursor(&self) -> CursorIcon { self.cursor }
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
    wnd.run_frame_tasks();
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
    wnd.run_frame_tasks();
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
    wnd.run_frame_tasks();
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
    wnd.run_frame_tasks();
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
    wnd.run_frame_tasks();
    assert_eq!(wnd.get_cursor(), CursorIcon::Help);
  }
}
