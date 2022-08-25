use crate::prelude::*;

/// Enumerate to describe which direction allow widget to scroll.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Hash)]
pub enum Scrollable {
  /// let child widget horizontal scrollable and the scroll view is as large as
  /// its parent allow.
  X,
  /// Let child widget vertical scrollable and the scroll view is as large  as
  /// its parent allow.
  Y,
  /// Let child widget both scrollable in horizontal and vertical, and the
  /// scroll view is as large as its parent allow.
  Both,
}

/// Helper struct for builtin scrollable field.
#[derive(Declare)]
pub struct ScrollableWidget {
  #[declare(builtin)]
  pub scrollable: Scrollable,
  #[declare(default)]
  pub pos: Point,
}

impl ComposeSingleChild for ScrollableWidget {
  fn compose_single_child(this: StateWidget<Self>, child: Widget, _: &mut BuildCtx) -> Widget {
    widget! {
      track { this: this.into_stateful() }
      UnconstrainedBox {
        left_anchor: this.pos.x,
        top_anchor: this.pos.y,
        on_wheel: move |e| {
          let ctx = e.context();
          let view_area = ctx.box_rect().unwrap();
          let content_area =  ctx.single_child_box().expect("must have a scrollable widget");
          let old = this.pos;
          let mut new = old;
          if this.scrollable != Scrollable::X {
            new.y = validate_pos(view_area.height(), content_area.height(), old.y - e.delta_y)
          }
          if this.scrollable != Scrollable::Y {
            new.x = validate_pos(view_area.width(), content_area.width(), old.x - e.delta_x);
          }
          if new != old {
            this.pos = new;
          }
        },
        ExprWidget { expr: child }
      }
    }
  }
}

#[inline]
fn validate_pos(view: f32, content: f32, pos: f32) -> f32 { pos.min(0.).max(view - content) }

#[cfg(test)]
mod tests {
  use crate::test::layout_info_by_path;

  use super::*;
  use winit::event::{DeviceId, ModifiersState, MouseScrollDelta, TouchPhase, WindowEvent};

  fn test_assert(scrollable: Scrollable, delta_x: f32, delta_y: f32, expect_x: f32, expect_y: f32) {
    let w = widget! {
     SizedBox {
       size: Size::new(1000., 1000.),
       scrollable,
     }
    };

    let mut wnd = Window::without_render(w, Size::new(100., 100.));

    wnd.draw_frame();

    let device_id = unsafe { DeviceId::dummy() };
    wnd.processes_native_event(WindowEvent::MouseWheel {
      device_id,
      delta: MouseScrollDelta::LineDelta(delta_x, delta_y),
      phase: TouchPhase::Started,
      modifiers: ModifiersState::default(),
    });
    wnd.draw_frame();

    assert_eq!(layout_info_by_path(&wnd, &[0, 0]).min_y(), expect_y);
    assert_eq!(layout_info_by_path(&wnd, &[0, 0, 0]).min_x(), expect_x);
  }

  #[test]
  fn x_scroll() {
    test_assert(Scrollable::X, 10., 10., -10., 0.);
    test_assert(Scrollable::X, 10000., 10., -900., 0.);
    test_assert(Scrollable::X, -100., 10., 0., 0.);
  }

  #[test]
  fn y_scroll() {
    test_assert(Scrollable::Y, 10., 10., 0., -10.);
    test_assert(Scrollable::Y, 10., 10000., 0., -900.);
    test_assert(Scrollable::Y, -10., -100., 0., 0.);
  }

  #[test]
  fn both_scroll() {
    test_assert(Scrollable::Both, 10., 10., -10., -10.);
    test_assert(Scrollable::Both, 10000., 10000., -900., -900.);
    test_assert(Scrollable::Both, -100., -100., 0., 0.);
  }
}
