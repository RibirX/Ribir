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
  scrollable: Scrollable,
  #[declare(default)]
  pos: Point,
}

impl Render for ScrollableWidget {
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    let size = clamp.max;
    if let Some(child) = ctx.single_child() {
      let content_clamp = self.content_clamp(clamp);
      let content = ctx.perform_child_layout(child, content_clamp);
      let pos = self.content_pos(content, &size);
      ctx.update_position(child, pos);
    }

    size
  }

  #[inline]
  fn paint(&self, _: &mut PaintingCtx) {}
}

impl ComposeSingleChild for ScrollableWidget {
  fn compose_single_child(this: Stateful<Self>, child: Widget, _: &mut BuildCtx) -> Widget
  where
    Self: Sized,
  {
    SingleChildWidget {
      widget: this.clone(),
      child: widget! {
        track { this }
        declare ExprWidget {
          expr: child,
          on_wheel: move |e| {
            let (view, content) = view_content(e);
            let old = this.pos;
            let mut new = old;
            if this.scrollable != Scrollable::X {
              new.x = validate_pos(view.width(), content.width(), old.x - e.delta_x);
            }
            if this.scrollable != Scrollable::Y {
              new.y = validate_pos(view.height(), content.height(), old.y - e.delta_y)
            }
            if new != old {
              this.pos = new;
            }
          }
        }
      },
    }
    .into_widget()
  }
}

#[inline]
fn validate_pos(view: f32, content: f32, pos: f32) -> f32 { pos.min(0.).max(view - content) }

impl ScrollableWidget {
  fn content_clamp(&self, _: BoxClamp) -> BoxClamp {
    BoxClamp {
      min: Size::zero(),
      max: Size::new(f32::MAX, f32::MAX),
    }
  }

  fn content_pos(&self, content: Size, view: &Size) -> Point {
    Point::new(
      validate_pos(view.width, content.width, self.pos.x),
      validate_pos(view.height, content.height, self.pos.y),
    )
  }
}

fn view_content(event: &WheelEvent) -> (Rect, Rect) {
  let ctx = event.context();

  let view = ctx.box_rect().unwrap();
  let child = ctx.single_child().unwrap();
  let content = ctx.widget_box_rect(child).unwrap();

  (view, content)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test::root_and_children_rect;
  use winit::event::{DeviceId, ModifiersState, MouseScrollDelta, TouchPhase, WindowEvent};

  fn test_assert(scrollable: Scrollable, delta_x: f32, delta_y: f32, child_pos: Point) {
    let w = widget! {
     declare SizedBox {
       size: Size::new(1000., 1000.),
       scrollable,
     }
    };

    let mut wnd = Window::without_render(w, Size::new(100., 100.));

    wnd.render_ready();

    let device_id = unsafe { DeviceId::dummy() };
    wnd.processes_native_event(WindowEvent::MouseWheel {
      device_id,
      delta: MouseScrollDelta::LineDelta(delta_x, delta_y),
      phase: TouchPhase::Started,
      modifiers: ModifiersState::default(),
    });
    wnd.render_ready();

    let (_, children) = root_and_children_rect(&mut wnd);
    assert_eq!(children[0].origin, child_pos);
  }

  #[test]
  fn x_scroll() {
    test_assert(Scrollable::X, 10., 10., Point::new(-10., 0.));
    test_assert(Scrollable::X, 10000., 10., Point::new(-900., 0.));
    test_assert(Scrollable::X, -100., 10., Point::new(0., 0.));
  }

  #[test]
  fn y_scroll() {
    test_assert(Scrollable::Y, 10., 10., Point::new(0., -10.));
    test_assert(Scrollable::Y, 10., 10000., Point::new(0., -900.));
    test_assert(Scrollable::Y, -10., -100., Point::new(0., 0.));
  }

  #[test]
  fn both_scroll() {
    test_assert(Scrollable::Both, 10., 10., Point::new(-10., -10.));
    test_assert(Scrollable::Both, 10000., 10000., Point::new(-900., -900.));
    test_assert(Scrollable::Both, -100., -100., Point::new(0., 0.));
  }
}
