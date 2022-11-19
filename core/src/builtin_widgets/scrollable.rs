use crate::prelude::*;
/// Enumerate to describe which direction allow widget to scroll.
#[derive(Debug, Clone, Copy, Default, PartialEq, PartialOrd, Hash)]
pub enum Scrollable {
  /// let child widget horizontal scrollable and the scroll view is as large as
  /// its parent allow.
  #[default]
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
  #[declare(builtin, default)]
  pub scrollable: Scrollable,
  #[declare(builtin, default)]
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
      Clip { UnconstrainedBox {
        id: view,
        dir: match this.scrollable {
          Scrollable::X => UnconstrainedDir::X,
          Scrollable::Y => UnconstrainedDir::Y,
          Scrollable::Both => UnconstrainedDir::Both,
        },
        wheel: move |e| this.validate_scroll(Point::new(e.delta_x, e.delta_y)),
        DynWidget {
          id: content,
          dyns: child,
          left_anchor: this.scroll_pos.x,
          top_anchor: this.scroll_pos.y,
        }
      }}

      change_on content.layout_size() ~> this.content_size
      change_on view.layout_size() ~> this.page
      change_on content.left_anchor Animate {
        transition: transitions::SMOOTH_SCROLL.get_from_or_default(ctx.theme()),
        lerp_fn: move |from, to, rate| {
          let from = from.abs_value(content.layout_width());
          let to = to.abs_value(content.layout_width());
          PositionUnit::Pixel(from.lerp(&to, rate))
        }
      }
      change_on content.top_anchor Animate {
        transition: transitions::SMOOTH_SCROLL.get_from_or_default(ctx.theme()),
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
  pub fn scroll_view_size(&self) -> Size { self.page }

  #[inline]
  pub fn scroll_content_size(&self) -> Size { self.content_size }

  /// return if the content greater than the view.
  pub fn can_scroll(&self) -> bool {
    match self.scrollable {
      Scrollable::X => self.scroll_content_size().width > self.scroll_view_size().width,
      Scrollable::Y => self.scroll_content_size().height > self.scroll_view_size().height,
      Scrollable::Both => self
        .scroll_content_size()
        .greater_than(self.scroll_view_size())
        .any(),
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
    let min = self.scroll_view_size() - self.scroll_content_size();
    self.scroll_pos = new.clamp(min.to_vector().to_point(), Point::zero());
  }
}

#[cfg(test)]
mod tests {
  use crate::test::{layout_info_by_path, MockBox};

  use super::*;
  use winit::event::{DeviceId, ModifiersState, MouseScrollDelta, TouchPhase, WindowEvent};

  fn test_assert(scrollable: Scrollable, delta_x: f32, delta_y: f32, expect_x: f32, expect_y: f32) {
    let w = widget! {
      MockBox {
        size: Size::new(1000., 1000.),
        scrollable,
      }
    };

    let mut wnd = Window::default_mock(w, Some(Size::new(100., 100.)));

    wnd.draw_frame();

    let device_id = unsafe { DeviceId::dummy() };
    wnd.processes_native_event(WindowEvent::MouseWheel {
      device_id,
      delta: MouseScrollDelta::PixelDelta((delta_x, delta_y).into()),
      phase: TouchPhase::Started,
      modifiers: ModifiersState::default(),
    });

    wnd.layout();
    let rect = layout_info_by_path(&wnd, &[0, 0, 0, 0]);
    assert_eq!(rect.origin.y, expect_y);
    let rect = layout_info_by_path(&wnd, &[0, 0, 0, 0, 0]);
    assert_eq!(rect.origin.x, expect_x);
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
