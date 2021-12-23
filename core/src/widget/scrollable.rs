use crate::{prelude::*, render::render_tree::RenderTree, widget::widget_tree::WidgetTree};

/// A widget let its child horizontal scrollable and the scroll view is as large
/// as its parent allow.
#[stateful]
#[derive(SingleChildWidget, Default, Clone, PartialEq)]
pub struct ScrollableX {
  pos: f32,
}

/// A widget let its child vertical scrollable and the scroll view is as large
/// as its parent allow.
#[stateful]
#[derive(SingleChildWidget, Default, Clone, PartialEq)]
pub struct ScrollableY {
  pos: f32,
}

/// A widget let its child both scrollable in horizontal and vertical, and the
/// scroll view is as large as its parent allow.
#[stateful]
#[derive(SingleChildWidget, Default, Clone, PartialEq)]
pub struct ScrollableBoth {
  pos: Point,
}

impl ScrollableX {
  #[inline]
  pub fn x_scroll(pos: f32) -> StatefulScrollableX {
    let scroll = ScrollableX { pos }.into_stateful();
    let scroll_ref = scroll.ref_cell();
    scroll.on_wheel(move |event| {
      let (view, content) = view_content(event);
      let old = scroll_ref.borrow().pos;
      let new = validate_pos(view.width(), content.width(), old - event.delta_x);
      if (new - old).abs() > f32::EPSILON {
        scroll_ref.borrow_mut().pos = new;
      }
    })
  }
}

impl ScrollableY {
  #[inline]
  pub fn y_scroll(pos: f32) -> StatefulScrollableY {
    let scroll = ScrollableY { pos }.into_stateful();
    let scroll_ref = scroll.ref_cell();
    scroll.on_wheel(move |event| {
      let (view, content) = view_content(event);
      let old = scroll_ref.borrow().pos;
      let new = validate_pos(view.height(), content.height(), old - event.delta_y);
      if (new - old).abs() > f32::EPSILON {
        scroll_ref.borrow_mut().pos = new;
      }
    })
  }
}

impl ScrollableBoth {
  #[inline]
  pub fn both_scroll(pos: Point) -> StatefulScrollableBoth {
    let scroll = ScrollableBoth { pos }.into_stateful();
    let scroll_ref = scroll.ref_cell();
    scroll.on_wheel(move |event| {
      let (view, content) = view_content(event);
      let old = scroll_ref.borrow().pos;
      let new = Point::new(
        validate_pos(view.width(), content.width(), old.x - event.delta_x),
        validate_pos(view.height(), content.height(), old.y - event.delta_y),
      );
      if new != old {
        scroll_ref.borrow_mut().pos = new;
      }
    })
  }
}

macro scroll_render_widget_impl($widget: ty, $state: ty) {
  impl RenderWidget for $widget {
    type RO = Self;

    #[inline]
    fn create_render_object(&self) -> Self::RO { self.clone() }

    #[inline]
    fn update_render_object(&self, object: &mut Self::RO, ctx: &mut UpdateCtx) {
      if self != object {
        *object = self.clone();
        ctx.mark_needs_layout();
      }
    }
  }

  impl RenderObject for $widget {
    #[inline]
    fn perform_layout(&mut self, clamp: BoxClamp, ctx: &mut RenderCtx) -> Size {
      debug_assert_eq!(ctx.children().count(), 1);
      let size = clamp.max;
      if let Some(mut child) = ctx.children().next() {
        let content_clamp = self.content_clamp(clamp);
        let content = child.perform_layout(content_clamp);
        let pos = self.content_pos(content, &size);
        child.update_position(pos);
      }

      size
    }

    fn only_sized_by_parent(&self) -> bool { true }

    fn paint<'a>(&'a self, _ctx: &mut PaintingContext<'a>) {
      // nothing to paint, just a layout widget.
    }
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
}

fn view_content(event: &WheelEvent) -> (Rect, Rect) {
  fn widget_rect(wid: WidgetId, tree: &WidgetTree, r_tree: &RenderTree) -> Rect {
    wid
      .relative_to_render(tree)
      .and_then(|rid| rid.layout_box_rect(r_tree))
      .unwrap_or_else(Rect::zero)
  }

  let w_tree = event.widget_tree();
  let r_tree = event.render_tree();
  let target = event.current_target();
  let view = widget_rect(target, w_tree, r_tree);
  let content = widget_rect(target.first_child(w_tree).unwrap(), w_tree, r_tree);
  (view, content)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test::root_and_children_rect;
  use winit::event::{DeviceId, ModifiersState, MouseScrollDelta, TouchPhase, WindowEvent};

  fn test_assert(widget: BoxedWidget, delta_x: f32, delta_y: f32, child_pos: Point) {
    let mut wnd = window::NoRenderWindow::without_render(widget, Size::new(100., 100.));

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
    #[derive(Debug)]
    struct X;

    impl CombinationWidget for X {
      fn build(&self, _: &mut BuildCtx) -> BoxedWidget {
        ScrollableX::x_scroll(0.)
          .have(SizedBox::from_size(Size::new(1000., 1000.)).box_it())
          .box_it()
      }
    }

    test_assert(X.box_it(), 10., 10., Point::new(-10., 0.));
    test_assert(X.box_it(), 10000., 10., Point::new(-900., 0.));
    test_assert(X.box_it(), -100., 10., Point::new(0., 0.));
  }

  #[test]
  fn y_scroll() {
    #[derive(Debug)]
    struct Y;

    impl CombinationWidget for Y {
      fn build(&self, _: &mut BuildCtx) -> BoxedWidget {
        ScrollableY::y_scroll(0.)
          .have(SizedBox::from_size(Size::new(1000., 1000.)).box_it())
          .box_it()
      }
    }

    test_assert(Y.box_it(), 10., 10., Point::new(0., -10.));
    test_assert(Y.box_it(), 10., 10000., Point::new(0., -900.));
    test_assert(Y.box_it(), -10., -100., Point::new(0., 0.));
  }

  #[test]
  fn both_scroll() {
    #[derive(Debug)]
    struct Both;

    impl CombinationWidget for Both {
      fn build(&self, _: &mut BuildCtx) -> BoxedWidget {
        ScrollableBoth::both_scroll(Point::default())
          .have(SizedBox::from_size(Size::new(1000., 1000.)).box_it())
          .box_it()
      }
    }

    test_assert(Both.box_it(), 10., 10., Point::new(-10., -10.));
    test_assert(Both.box_it(), 10000., 10000., Point::new(-900., -900.));
    test_assert(Both.box_it(), -100., -100., Point::new(0., 0.));
  }
}
