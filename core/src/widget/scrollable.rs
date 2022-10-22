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
  pub scroll_pos: Point,
  #[declare(skip)]
  page: Size,
  #[declare(skip)]
  content_size: Size,
}

impl ComposeChild for ScrollableWidget {
  type Child = Widget;
  fn compose_child(this: StateWidget<Self>, child: Self::Child) -> Widget {
    widget! {
      track { this: this.into_stateful() }
      UnconstrainedBox {
        id: view,
        wheel: move |e| this.validate_scroll(Point::new(e.delta_x, e.delta_y)),
        ExprWidget {
          id: content,
          expr: child,
          left_anchor: this.scroll_pos.x,
          top_anchor: this.scroll_pos.y,
        }
      }

      change_on content.layout_size() ~> this.content_size
      change_on view.layout_size() ~> this.page
      change_on content.left_anchor Animate {
        transition: transitions::SMOOTH_SCROLL.get_from_or_default(ctx),
        lerp_fn: move |from, to, rate| {
          let from = from.abs_value(content.layout_width());
          let to = to.abs_value(content.layout_width());
          PositionUnit::Pixel(from.lerp(&to, rate))
        }
      }
      change_on content.top_anchor Animate {
        transition: transitions::SMOOTH_SCROLL.get_from_or_default(ctx),
        lerp_fn: move |from, to, rate| {
          let from = from.abs_value(content.layout_height());
          let to = to.abs_value(content.layout_height());
          PositionUnit::Pixel(from.lerp(&to, rate))
        }
      }
    }
  }
}

impl ScrollableWidget {
  #[inline]
  pub fn page_size(&self) -> Size { self.page }

  #[inline]
  pub fn content_size(&self) -> Size { self.content_size }

  /// return if the content greater than the view.
  pub fn can_scroll(&self) -> bool {
    match self.scrollable {
      Scrollable::X => self.content_size().width > self.page_size().width,
      Scrollable::Y => self.content_size().height > self.page_size().height,
      Scrollable::Both => self.content_size().greater_than(self.page_size()).any(),
    }
  }

  fn validate_scroll(&mut self, delta: Point) {
    let mut new = self.scroll_pos;
    if self.scrollable != Scrollable::X {
      new.y += delta.y;
    }
    if self.scrollable != Scrollable::Y {
      new.x += delta.x;
    }
    let min = self.page_size() - self.content_size();
    self.scroll_pos = new.clamp(min.to_vector().to_point(), Point::zero());
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use winit::event::{DeviceId, ModifiersState, MouseScrollDelta, TouchPhase, WindowEvent};

  fn test_assert(scrollable: Scrollable, delta_x: f32, delta_y: f32, expect_x: f32, expect_y: f32) {
    let global_pos = Stateful::new(Point::zero());
    let w = widget! {
      track { global_pos: global_pos.clone() }
      SizedBox {
        size: Size::new(1000., 1000.),
        scrollable,
        performed_layout: move|ctx| *global_pos = ctx.map_to_global(Point::zero())
      }
    };

    let mut wnd = Window::without_render(w, None, Some(Size::new(100., 100.)));

    wnd.draw_frame();

    let device_id = unsafe { DeviceId::dummy() };
    wnd.processes_native_event(WindowEvent::MouseWheel {
      device_id,
      delta: MouseScrollDelta::PixelDelta((delta_x, delta_y).into()),
      phase: TouchPhase::Started,
      modifiers: ModifiersState::default(),
    });

    wnd.layout_ready();

    assert_eq!(global_pos.raw_ref().x, expect_x);
    assert_eq!(global_pos.raw_ref().y, expect_y);
  }

  #[test]
  fn x_scroll() {
    test_assert(Scrollable::X, -10., -10., -10., 0.);
    test_assert(Scrollable::X, -10000., -10., -900., 0.);
    test_assert(Scrollable::X, 100., -10., 0., 0.);
  }

  #[test]
  fn y_scroll() {
    test_assert(Scrollable::Y, -10., -10., 0., -10.);
    test_assert(Scrollable::Y, -10., -10000., 0., -900.);
    test_assert(Scrollable::Y, 10., 100., 0., 0.);
  }

  #[test]
  fn both_scroll() {
    test_assert(Scrollable::Both, -10., -10., -10., -10.);
    test_assert(Scrollable::Both, -10000., -10000., -900., -900.);
    test_assert(Scrollable::Both, 100., 100., 0., 0.);
  }
}
