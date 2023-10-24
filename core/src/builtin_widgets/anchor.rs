use crate::prelude::*;

/// Widget use to anchor child constraints with the left edge of parent widget.
#[derive(Declare, Query, SingleChild)]
pub struct LeftAnchor {
  #[declare(builtin, default)]
  pub left_anchor: f32,
}

/// Widget use to anchor child constraints with the right edge of parent widget.
#[derive(Declare, Query, SingleChild)]
pub struct RightAnchor {
  #[declare(builtin, default)]
  pub right_anchor: f32,
}

/// Widget use to anchor child constraints with the top edge of parent widget.
#[derive(Declare, Query, SingleChild)]
pub struct TopAnchor {
  #[declare(builtin, default)]
  pub top_anchor: f32,
}

/// Widget use to anchor child constraints with the bottom edge of parent
/// widget.
#[derive(Declare, Query, SingleChild)]
pub struct BottomAnchor {
  #[declare(builtin, default)]
  pub bottom_anchor: f32,
}

impl Render for LeftAnchor {
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    let mut layouter = ctx.assert_single_child_layouter();
    let child_size = layouter.perform_widget_layout(clamp);
    let left = self.left_anchor;
    layouter.update_position(Point::new(left, 0.));
    Size::new((child_size.width + left).max(0.), child_size.height)
  }

  fn paint(&self, _: &mut PaintingCtx) {}

  fn hit_test(&self, _: &HitTestCtx, _: Point) -> HitTest {
    HitTest { hit: false, can_hit_child: true }
  }
}

impl Render for RightAnchor {
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    let mut layouter = ctx.assert_single_child_layouter();
    let child_size = layouter.perform_widget_layout(clamp);
    let right = self.right_anchor;
    let x = clamp.max.width - child_size.width - right;
    layouter.update_position(Point::new(x, 0.));

    Size::new((child_size.width + x).max(0.), child_size.height)
  }

  fn paint(&self, _: &mut PaintingCtx) {}

  fn hit_test(&self, _: &HitTestCtx, _: Point) -> HitTest {
    HitTest { hit: false, can_hit_child: true }
  }
}

impl Render for TopAnchor {
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    let mut layouter = ctx.assert_single_child_layouter();
    let child_size = layouter.perform_widget_layout(clamp);
    let top = self.top_anchor;
    layouter.update_position(Point::new(0., top));
    Size::new(child_size.width, (child_size.height + top).max(0.))
  }

  fn paint(&self, _: &mut PaintingCtx) {}

  fn hit_test(&self, _: &HitTestCtx, _: Point) -> HitTest {
    HitTest { hit: false, can_hit_child: true }
  }
}

impl Render for BottomAnchor {
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    let mut layouter = ctx.assert_single_child_layouter();
    let child_size = layouter.perform_widget_layout(clamp);
    let bottom = self.bottom_anchor;
    let y = clamp.max.height - child_size.height - bottom;
    layouter.update_position(Point::new(0., y));
    Size::new(child_size.width, (child_size.height + y).max(0.))
  }

  fn paint(&self, _: &mut PaintingCtx) {}

  fn hit_test(&self, _: &HitTestCtx, _: Point) -> HitTest {
    HitTest { hit: false, can_hit_child: true }
  }
}
#[cfg(test)]
mod test {
  use super::*;
  use crate::test_helper::*;
  use ribir_dev_helper::*;
  const CHILD_SIZE: Size = Size::new(50., 50.);
  const WND_SIZE: Size = Size::new(100., 100.);

  fn pixel_left_top() -> impl WidgetBuilder {
    fn_widget! {
      @MockBox {
        size: CHILD_SIZE,
        left_anchor: 1.,
        top_anchor: 1.,
      }
    }
  }
  widget_layout_test!(
    pixel_left_top,
    wnd_size = WND_SIZE,
    { path = [0, 0], y == 1., }
    { path = [0, 0, 0], x == 1., }
  );

  fn pixel_left_bottom() -> impl WidgetBuilder {
    fn_widget! {
      @MockBox {
        size: CHILD_SIZE,
        left_anchor: 1.,
        bottom_anchor: 1.,
      }
    }
  }
  widget_layout_test!(
    pixel_left_bottom,
    wnd_size = WND_SIZE,
    { path = [0, 0], y == 49.,}
    { path = [0, 0, 0], x == 1., }
  );

  fn pixel_top_right() -> impl WidgetBuilder {
    fn_widget! {
      @MockBox {
        size: CHILD_SIZE,
        right_anchor: 1.,
        top_anchor: 1.,
      }
    }
  }
  widget_layout_test!(
    pixel_top_right,
    wnd_size = WND_SIZE,
    { path = [0, 0], y == 1.,}
    { path = [0, 0, 0], x == 49.,}
  );

  fn pixel_bottom_right() -> impl WidgetBuilder {
    fn_widget! {
      @MockBox {
        size: CHILD_SIZE,
        right_anchor: 1.,
        bottom_anchor: 1.,
      }
    }
  }
  widget_layout_test!(
    pixel_bottom_right,
    wnd_size = WND_SIZE,
    { path = [0, 0], y == 49.,}
    { path = [0, 0, 0], x== 49.,}
  );
}
