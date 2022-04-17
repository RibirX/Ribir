use crate::prelude::*;

/// A widget let its child horizontal scrollable and the scroll view is as large
/// as its parent allow.
#[derive(SingleChildWidget, Default, Clone, PartialEq)]
pub struct ScrollableX {
  pos: f32,
}

/// A widget let its child vertical scrollable and the scroll view is as large
/// as its parent allow.
#[derive(SingleChildWidget, Default, Clone, PartialEq)]
pub struct ScrollableY {
  pos: f32,
}

/// A widget let its child both scrollable in horizontal and vertical, and the
/// scroll view is as large as its parent allow.
#[derive(SingleChildWidget, Default, Clone, PartialEq)]
pub struct ScrollableBoth {
  pos: Point,
}

/// Enumerate to describe which direction allow widget to scroll.

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Hash)]
pub enum Scrollable {
  X,
  Y,
  Both,
}

/// Helper struct for builtin scrollable field.
#[derive(SingleChildWidget, Declare)]
pub struct ScrollableDeclare {
  #[declare(builtin)]
  scrollable: Scrollable,
}

impl ScrollableX {
  #[inline]
  pub fn x_scroll(pos: f32) -> Stateful<ScrollableX> {
    let scroll = ScrollableX { pos }.into_stateful();
    let mut scroll_ref = unsafe { scroll.state_ref() };
    scroll.on_wheel(move |event| {
      let (view, content) = view_content(event);
      let old = scroll_ref.pos;
      let new = validate_pos(view.width(), content.width(), old - event.delta_x);
      if (new - old).abs() > f32::EPSILON {
        scroll_ref.pos = new;
      }
    })
  }
}

impl ScrollableY {
  #[inline]
  pub fn y_scroll(pos: f32) -> Stateful<ScrollableY> {
    let scroll = ScrollableY { pos }.into_stateful();
    let mut scroll_ref = unsafe { scroll.state_ref() };
    scroll.on_wheel(move |event| {
      let (view, content) = view_content(event);
      let old = scroll_ref.pos;
      let new = validate_pos(view.height(), content.height(), old - event.delta_y);
      if (new - old).abs() > f32::EPSILON {
        scroll_ref.pos = new;
      }
    })
  }
}

impl ScrollableBoth {
  #[inline]
  pub fn both_scroll(pos: Point) -> Stateful<ScrollableBoth> {
    let scroll = ScrollableBoth { pos }.into_stateful();
    let mut scroll_ref = unsafe { scroll.state_ref() };
    scroll.on_wheel(move |event| {
      let (view, content) = view_content(event);
      let old = scroll_ref.pos;
      let new = Point::new(
        validate_pos(view.width(), content.width(), old.x - event.delta_x),
        validate_pos(view.height(), content.height(), old.y - event.delta_y),
      );
      if new != old {
        scroll_ref.pos = new;
      }
    })
  }
}

macro scroll_render_widget_impl($widget: ty, $state: ty) {
  impl RenderWidget for $widget {
    fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
      let size = clamp.max;
      if let Some(child) = ctx.single_child() {
        let content_clamp = self.content_clamp(clamp);
        let content = ctx.perform_render_child_layout(child, content_clamp);
        let pos = self.content_pos(content, &size);
        ctx.update_position(child, pos);
      }

      size
    }

    fn only_sized_by_parent(&self) -> bool { true }

    fn paint(&self, _: &mut PaintingCtx) {}
  }
}

scroll_render_widget_impl!(ScrollableX, ScrollableXState);
scroll_render_widget_impl!(ScrollableY, ScrollableYState);
scroll_render_widget_impl!(ScrollableBoth, ScrollableBothState);

#[inline]
fn validate_pos(view: f32, content: f32, pos: f32) -> f32 { pos.min(0.).max(view - content) }

pub trait ScrollWorker {
  fn content_clamp(&self, clamp: BoxClamp) -> BoxClamp;

  fn content_pos(&self, content: Size, view: &Size) -> Point;

  fn offset_x(&self) -> f32;

  fn offset_y(&self) -> f32;
}

impl ScrollWorker for ScrollableX {
  fn content_clamp(&self, clamp: BoxClamp) -> BoxClamp {
    let min = Size::zero();
    let mut max = clamp.max;
    max.width = f32::MAX;

    BoxClamp { min, max }
  }

  fn content_pos(&self, content: Size, view: &Size) -> Point {
    Point::new(validate_pos(view.width, content.width, self.pos), 0.)
  }

  fn offset_x(&self) -> f32 { self.pos }

  fn offset_y(&self) -> f32 { 0.0 }
}

impl ScrollWorker for ScrollableY {
  fn content_clamp(&self, clamp: BoxClamp) -> BoxClamp {
    let min = Size::zero();
    let mut max = clamp.max;
    max.height = f32::MAX;

    BoxClamp { min, max }
  }

  fn content_pos(&self, content: Size, view: &Size) -> Point {
    Point::new(0., validate_pos(view.height, content.height, self.pos))
  }

  fn offset_x(&self) -> f32 { 0.0 }

  fn offset_y(&self) -> f32 { self.pos }
}

impl ScrollWorker for ScrollableBoth {
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

  fn offset_x(&self) -> f32 { self.pos.x }

  fn offset_y(&self) -> f32 { self.pos.y }
}

fn view_content(event: &WheelEvent) -> (Rect, Rect) {
  let ctx = event.context();

  let view = ctx.box_rect().unwrap();
  let child = ctx.single_child().unwrap();
  let content = ctx.widget_box_rect(child).unwrap();

  (view, content)
}

impl BoxWidget<RenderMarker> for SingleChild<ScrollableDeclare> {
  fn box_it(self) -> BoxedWidget {
    match self.scrollable {
      Scrollable::X => SingleChild {
        widget: ScrollableX::x_scroll(0.),
        child: self.child,
      }
      .box_it(),
      Scrollable::Y => SingleChild {
        widget: ScrollableY::y_scroll(0.),
        child: self.child,
      }
      .box_it(),
      Scrollable::Both => SingleChild {
        widget: ScrollableBoth::both_scroll(Point::zero()),
        child: self.child,
      }
      .box_it(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test::root_and_children_rect;
  use winit::event::{DeviceId, ModifiersState, MouseScrollDelta, TouchPhase, WindowEvent};

  fn test_assert(scrollable: Scrollable, delta_x: f32, delta_y: f32, child_pos: Point) {
    impl CombinationWidget for Scrollable {
      #[widget]
      fn build(&self, ctx: &mut BuildCtx) -> BoxedWidget {
        widget! {
          declare SizedBox {
            size: Size::new(1000., 1000.),
            scrollable: *self,
          }
        }
      }
    }

    let mut wnd = Window::without_render(scrollable.box_it(), Size::new(100., 100.));

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
