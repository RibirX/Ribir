use crate::prelude::*;
use winit::window::CursorIcon;

/// `Cursor` is an attribute to assign an `cursor` to a widget.

#[derive(Default, Debug, Declare)]
pub struct Cursor {
  #[declare(builtin, default)]
  pub cursor: CursorIcon,
}

impl ComposeChild for Cursor {
  type Child = Widget;
  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> impl WidgetBuilder {
    fn_widget! {
      let save_cursor = Stateful::new(CursorIcon::Default);
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
  }
}

impl Cursor {
  fn get_cursor(&self) -> CursorIcon { self.cursor }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{reset_test_env, test_helper::*};
  use winit::event::{DeviceId, WindowEvent};

  #[test]
  fn tree_down_up() {
    reset_test_env!();

    let row_tree = fn_widget! {
      @MockBox {
        size: Size::new(f32::INFINITY, f32::INFINITY),
        cursor: CursorIcon::AllScroll,
        @MockMulti{
          @MockBox {
            size: Size::new(200., 200.),
            cursor: CursorIcon::Pointer,
            @MockBox {
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
      },
      1.,
    );
    wnd.run_frame_tasks();
    assert_eq!(wnd.get_cursor(), CursorIcon::Pointer);

    let device_id = unsafe { DeviceId::dummy() };
    wnd.dispatcher.borrow_mut().dispatch(
      WindowEvent::CursorMoved {
        device_id,
        position: (201f64, 1.).into(),
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
      },
      1.,
    );
    wnd.run_frame_tasks();
    assert_eq!(wnd.get_cursor(), CursorIcon::Pointer);

    let device_id = unsafe { DeviceId::dummy() };
    wnd.dispatcher.borrow_mut().dispatch(
      WindowEvent::CursorMoved {
        device_id,
        position: (1f64, 1.).into(),
      },
      1.,
    );
    wnd.run_frame_tasks();
    assert_eq!(wnd.get_cursor(), CursorIcon::Help);
  }
}
