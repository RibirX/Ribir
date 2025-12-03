use crate::prelude::*;

/// Assigns a `CursorIcon` to a widget to control the pointer cursor while
/// hovering it.
///
/// This is a built-in `FatObj` field. Setting the `cursor` field attaches a
/// `Cursor` widget that updates the window cursor on pointer enter/leave.
///
/// # Example
///
/// Show a pointer cursor when hovering the text.
///
/// ```rust
/// use ribir::prelude::*;
///
/// text! {
///   cursor: CursorIcon::Pointer,
///   text: "Hover me!"
/// };
/// ```
#[derive(Default, Debug)]
pub struct Cursor {
  pub cursor: CursorIcon,
}

impl Declare for Cursor {
  type Builder = FatObj<()>;
  #[inline]
  fn declarer() -> Self::Builder { FatObj::new(()) }
}

impl<'c> ComposeChild<'c> for Cursor {
  type Child = Widget<'c>;
  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
    fn_widget! {
      let save_cursor: Stateful<Option<CursorIcon>> = Stateful::new(None);
      let mut child = FatObj::new(child);
      @(child) {
        on_pointer_enter: move |e: &mut PointerEvent| {
          if e.point_type == PointerType::Mouse
            && e.mouse_buttons() == MouseButtons::empty()
          {
            let wnd = e.window();
            let old_cursor = *$read(save_cursor);
            if old_cursor != Some(wnd.get_cursor()) {
              *$write(save_cursor) = Some(wnd.get_cursor());
              wnd.set_cursor($read(this).cursor);
            }
          }
        },
        on_pointer_leave: move |e: &mut PointerEvent| {
          if let Some(cursor) = $write(save_cursor).take() {
            e.window().set_cursor(cursor);
          }
        },
        on_disposed: move |e| {
          if let Some(cursor) = $write(save_cursor).take() {
            e.window().set_cursor(cursor);
          }
        },
      }
    }
    .into_widget()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{reset_test_env, test_helper::*};

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

    let wnd = TestWindow::from_widget(row_tree);

    wnd.draw_frame();

    wnd.process_cursor_move(Point::new(1., 1.));
    wnd.run_frame_tasks();
    assert_eq!(wnd.get_cursor(), CursorIcon::Help);

    wnd.process_cursor_move(Point::new(101., 1.));
    wnd.run_frame_tasks();
    assert_eq!(wnd.get_cursor(), CursorIcon::Pointer);

    wnd.process_cursor_move(Point::new(201., 1.));
    wnd.run_frame_tasks();
    assert_eq!(wnd.get_cursor(), CursorIcon::AllScroll);

    wnd.process_cursor_move(Point::new(101., 1.));
    wnd.run_frame_tasks();
    assert_eq!(wnd.get_cursor(), CursorIcon::Pointer);

    wnd.process_cursor_move(Point::new(1., 1.));
    wnd.run_frame_tasks();
    assert_eq!(wnd.get_cursor(), CursorIcon::Help);
  }
}
