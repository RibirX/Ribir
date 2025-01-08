use crate::prelude::*;
/// Enumerate to describe which direction allow widget to scroll.
#[derive(Debug, Clone, Copy, Default, PartialEq, PartialOrd, Hash)]
pub enum Scrollable {
  /// let child widget horizontal scrollable and the scroll view is as large as
  /// its parent allow.
  X,
  /// Let child widget vertical scrollable and the scroll view is as large  as
  /// its parent allow.
  #[default]
  Y,
  /// Let child widget both scrollable in horizontal and vertical, and the
  /// scroll view is as large as its parent allow.
  Both,
}

/// Helper struct for builtin scrollable field.
#[derive(Default)]
pub struct ScrollableWidget {
  pub scrollable: Scrollable,
  scroll_pos: Point,
  page: Size,
  content_size: Size,
}

impl Declare for ScrollableWidget {
  type Builder = FatObj<()>;
  #[inline]
  fn declarer() -> Self::Builder { FatObj::new(()) }
}

impl<'c> ComposeChild<'c> for ScrollableWidget {
  type Child = Widget<'c>;
  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
    fn_widget! {
      let mut view = @UnconstrainedBox {
        dir: distinct_pipe!{
          let this = $this;
          match this.scrollable {
            Scrollable::X => UnconstrainedDir::X,
            Scrollable::Y => UnconstrainedDir::Y,
            Scrollable::Both => UnconstrainedDir::Both,
          }
        },
        clamp_dim: ClampDim::MAX_SIZE,
        on_wheel: move |e| $this.write().scroll(-e.delta_x, -e.delta_y),
      };

      let child = FatObj::new(child);
      let mut child = @ $child {
        anchor: distinct_pipe!{
          let this = $this;
          let pos = this.get_scroll_pos();
          Anchor::left_top(-pos.x, -pos.y)
        }
      };

      watch!($child.layout_size())
        .distinct_until_changed()
        .subscribe(move |v| $this.write().set_content_size(v));
      watch!($view.layout_size())
        .distinct_until_changed()
        .subscribe(move |v| $this.write().set_page(v));

      @Clip { @ $view { @ { child } } }
    }
    .into_widget()
  }
}

impl ScrollableWidget {
  pub fn scroll(&mut self, x: f32, y: f32) {
    let mut new = self.scroll_pos;
    if self.scrollable != Scrollable::X {
      new.y += y;
    }
    if self.scrollable != Scrollable::Y {
      new.x += x;
    }
    self.jump_to(new);
  }

  pub fn jump_to(&mut self, top_left: Point) {
    let max = self.max_scrollable();
    self.scroll_pos = top_left.clamp(Point::zero(), max.to_vector().to_point());
  }

  #[inline]
  pub fn scroll_view_size(&self) -> Size { self.page }

  #[inline]
  pub fn scroll_content_size(&self) -> Size { self.content_size }

  pub fn is_x_scrollable(&self) -> bool {
    self.scrollable != Scrollable::Y && self.content_size.width > self.page.width
  }

  pub fn is_y_scrollable(&self) -> bool {
    self.scrollable != Scrollable::X && self.content_size.height > self.page.height
  }

  pub fn max_scrollable(&self) -> Point {
    let max = self.scroll_content_size() - self.scroll_view_size();
    max.to_vector().to_point().max(Point::zero())
  }

  /// Return the pixel along the axis of the scrollable widget that you want
  /// displayed in the upper left.
  pub fn get_scroll_pos(&self) -> Point { self.scroll_pos }

  pub fn get_x_scroll_rate(&self) -> f32 {
    let content = self.content_size.width;
    if content.is_infinite() || content.is_nan() { 0. } else { self.scroll_pos.x / content }
  }

  pub fn get_y_scroll_rate(&self) -> f32 {
    let content = self.content_size.height;
    if content.is_infinite() || content.is_nan() { 0. } else { self.scroll_pos.y / content }
  }

  fn sync_pos(&mut self) { self.jump_to(self.scroll_pos) }

  fn set_content_size(&mut self, content_size: Size) {
    self.content_size = content_size;
    self.sync_pos()
  }

  fn set_page(&mut self, page: Size) {
    self.page = page;
    self.sync_pos()
  }
}

#[cfg(test)]
mod tests {
  use winit::event::{DeviceId, MouseScrollDelta, TouchPhase, WindowEvent};

  use super::*;
  use crate::{reset_test_env, test_helper::*};

  fn test_assert(scrollable: Scrollable, delta_x: f32, delta_y: f32, expect_x: f32, expect_y: f32) {
    let w = fn_widget! {
      @MockBox {
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
    });

    wnd.draw_frame();
    let pos = wnd.layout_info_by_path(&[0, 0, 0]).unwrap().pos;
    assert_eq!(pos, Point::new(expect_x, expect_y));
  }

  #[test]
  fn x_scroll() {
    reset_test_env!();

    test_assert(Scrollable::X, -10., -10., -10., 0.);
    test_assert(Scrollable::X, -10000., -10., -900., 0.);
    test_assert(Scrollable::X, 100., -10., 0., 0.);
  }

  #[test]
  fn y_scroll() {
    reset_test_env!();

    test_assert(Scrollable::Y, -10., -10., 0., -10.);
    test_assert(Scrollable::Y, -10., -10000., 0., -900.);
    test_assert(Scrollable::Y, 10., 100., 0., 0.);
  }

  #[test]
  fn both_scroll() {
    reset_test_env!();

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

  #[test]
  fn scroll_content_expand() {
    reset_test_env!();

    let w = fn_widget! {
      @FixedBox {
        size: Size::new(200., 200.),
        @ScrollableWidget {
          scrollable: Scrollable::Both,
          on_performed_layout: move |ctx| {
            assert_eq!(ctx.box_size(), Some(Size::new(200., 200.)));
          },
          @MockBox {
            size: Size::new(100., 100.),
            on_performed_layout: move |ctx| {
              assert_eq!(ctx.box_size(), Some(Size::new(200., 200.)));
            },
          }
        }
      }
    };

    let mut wnd = TestWindow::new_with_size(w, Size::new(200., 200.));
    wnd.draw_frame();
  }
}
