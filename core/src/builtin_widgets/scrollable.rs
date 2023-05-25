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
  fn compose_child(this: State<Self>, child: Self::Child) -> Widget {
    widget! {
      states { this: this.into_writable() }
      init ctx => {
        let smooth_scroll = transitions::SMOOTH_SCROLL.of(ctx);
      }
      Clip {
        UnconstrainedBox {
          id: view,
          dir: match this.scrollable {
            Scrollable::X => UnconstrainedDir::X,
            Scrollable::Y => UnconstrainedDir::Y,
            Scrollable::Both => UnconstrainedDir::Both,
          },
          clamp_dim: ClampDim::MAX_SIZE,
          on_wheel: move |e| this.validate_scroll(Point::new(e.delta_x, e.delta_y)),
          DynWidget {
            id: content,
            dyns: child,
            left_anchor: this.scroll_pos.x,
            top_anchor: this.scroll_pos.y,
          }
      }}

      transition (
        prop!(content.left_anchor, PositionUnit::lerp_fn(content.layout_width())),
        prop!(content.top_anchor, PositionUnit::lerp_fn(content.layout_height()))
      ) { by: smooth_scroll.clone() }

      finally {
        let_watch!(content.layout_size())
          .distinct_until_changed()
          .subscribe(move |v| this.content_size = v);
        let_watch!(view.layout_size())
          .distinct_until_changed()
          .subscribe(move |v| this.page = v);
      }

    }
  }
}

impl ScrollableWidget {
  #[inline]
  pub fn jump_to(&mut self, top_left: Point) {
    let min = self.scroll_view_size() - self.scroll_content_size();
    self.scroll_pos = top_left.clamp(min.to_vector().to_point(), Point::zero());
  }
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
    self.jump_to(new);
  }
}

#[cfg(test)]
mod tests {
  use crate::{
    impl_query_self_only,
    test_helper::{MockBox, TestWindow},
  };

  use super::*;
  use winit::event::{DeviceId, ModifiersState, MouseScrollDelta, TouchPhase, WindowEvent};

  fn test_assert(scrollable: Scrollable, delta_x: f32, delta_y: f32, expect_x: f32, expect_y: f32) {
    let w = widget! {
      MockBox {
        size: Size::new(1000., 1000.),
        scrollable,
      }
    };

    let mut wnd = TestWindow::new_with_size(w, Size::new(100., 100.));
    wnd.draw_frame();

    let device_id = unsafe { DeviceId::dummy() };
    #[allow(deprecated)]
    wnd.processes_native_event(WindowEvent::MouseWheel {
      device_id,
      delta: MouseScrollDelta::PixelDelta((delta_x, delta_y).into()),
      phase: TouchPhase::Started,
      modifiers: ModifiersState::default(),
    });

    wnd.layout();
    let pos = wnd.layout_info_by_path(&[0, 0, 0, 0]).unwrap().pos;
    assert_eq!(pos.y, expect_y);
    let pos = wnd.layout_info_by_path(&[0, 0, 0, 0, 0]).unwrap().pos;
    assert_eq!(pos.x, expect_x);
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

  #[derive(SingleChild, Declare, Clone)]
  pub struct FixedBox {
    pub size: Size,
  }
  impl Render for FixedBox {
    #[inline]
    fn perform_layout(&self, _: BoxClamp, ctx: &mut LayoutCtx) -> Size {
      ctx.perform_single_child_layout(BoxClamp { min: self.size, max: self.size });
      self.size
    }
    #[inline]
    fn only_sized_by_parent(&self) -> bool { true }
    #[inline]
    fn paint(&self, _: &mut PaintingCtx) {}
  }
  impl Query for FixedBox {
    impl_query_self_only!();
  }

  #[test]
  fn scroll_content_expand() {
    let w = widget! {
      FixedBox {
        size: Size::new(200., 200.),
        ScrollableWidget {
          scrollable: Scrollable::Both,
          on_performed_layout: move |ctx| {
            let size = ctx.layout_info().and_then(|info| info.size);
            assert_eq!(size, Some(Size::new(200., 200.)));
          },
          MockBox {
            size: Size::new(100., 100.),
            on_performed_layout: move |ctx| {
              let size = ctx.layout_info().and_then(|info| info.size);
              assert_eq!(size, Some(Size::new(100., 100.)));
            },
          }
        }
      }
    };

    let mut wnd = TestWindow::new_with_size(w, Size::new(200., 200.));
    wnd.draw_frame();
  }
}
