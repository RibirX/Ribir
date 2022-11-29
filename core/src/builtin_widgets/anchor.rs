use crate::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PositionUnit {
  /// Pixels
  Pixel(f32),
  /// Describe percent of widget self size. For example,  `Percent(10)` use in
  /// x-axis means 10 percent of widget's width, in y-axis means 10 percent of
  /// widget's height.
  Percent(f32),
}

/// Widget use to anchor child constraints with the left edge of parent widget.
#[derive(Declare, SingleChild)]
pub struct LeftAnchor {
  #[declare(convert=into, builtin)]
  pub left_anchor: PositionUnit,
}

/// Widget use to anchor child constraints with the right edge of parent widget.
#[derive(Declare, SingleChild)]
pub struct RightAnchor {
  #[declare(convert=into, builtin)]
  pub right_anchor: PositionUnit,
}

/// Widget use to anchor child constraints with the top edge of parent widget.
#[derive(Declare, SingleChild)]
pub struct TopAnchor {
  #[declare(convert=into, builtin)]
  pub top_anchor: PositionUnit,
}

/// Widget use to anchor child constraints with the bottom edge of parent
/// widget.
#[derive(Declare, SingleChild)]
pub struct BottomAnchor {
  #[declare(convert=into, builtin)]
  pub bottom_anchor: PositionUnit,
}

impl Query for LeftAnchor {
  crate::impl_query_self_only!();
}

impl Query for TopAnchor {
  crate::impl_query_self_only!();
}

impl Query for RightAnchor {
  crate::impl_query_self_only!();
}
impl Query for BottomAnchor {
  crate::impl_query_self_only!();
}

impl Render for LeftAnchor {
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    let mut layouter = ctx.assert_single_child_layouter();
    let child_size = layouter.perform_widget_layout(clamp);
    let left = self.left_anchor.abs_value(child_size.width);
    layouter.update_position(Point::new(left, 0.));
    child_size
  }

  fn paint(&self, _: &mut PaintingCtx) {}

  fn can_overflow(&self) -> bool { true }

  fn hit_test(&self, _: &HitTestCtx, _: Point) -> HitTest {
    HitTest { hit: false, can_hit_child: true }
  }
}

impl Render for RightAnchor {
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    let mut layouter = ctx.assert_single_child_layouter();
    let child_size = layouter.perform_widget_layout(clamp);
    let right = self.right_anchor.abs_value(child_size.width);
    let x = clamp.max.width - child_size.width - right;
    layouter.update_position(Point::new(x, 0.));
    child_size
  }

  fn paint(&self, _: &mut PaintingCtx) {}

  fn can_overflow(&self) -> bool { true }

  fn hit_test(&self, _: &HitTestCtx, _: Point) -> HitTest {
    HitTest { hit: false, can_hit_child: true }
  }
}

impl Render for TopAnchor {
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    let mut layouter = ctx.assert_single_child_layouter();
    let child_size = layouter.perform_widget_layout(clamp);
    let top = self.top_anchor.abs_value(child_size.height);
    layouter.update_position(Point::new(0., top));
    child_size
  }

  fn paint(&self, _: &mut PaintingCtx) {}

  fn can_overflow(&self) -> bool { true }

  fn hit_test(&self, _: &HitTestCtx, _: Point) -> HitTest {
    HitTest { hit: false, can_hit_child: true }
  }
}

impl Render for BottomAnchor {
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    let mut layouter = ctx.assert_single_child_layouter();
    let child_size = layouter.perform_widget_layout(clamp);
    let bottom = self.bottom_anchor.abs_value(child_size.height);
    let y = clamp.max.height - child_size.height - bottom;
    layouter.update_position(Point::new(0., y));
    child_size
  }

  fn paint(&self, _: &mut PaintingCtx) {}

  fn can_overflow(&self) -> bool { true }

  fn hit_test(&self, _: &HitTestCtx, _: Point) -> HitTest {
    HitTest { hit: false, can_hit_child: true }
  }
}

impl From<f32> for PositionUnit {
  #[inline]
  fn from(v: f32) -> Self { PositionUnit::Pixel(v) }
}

impl PositionUnit {
  pub fn abs_value(self, self_size: f32) -> f32 {
    match self {
      PositionUnit::Pixel(pixel) => pixel,
      PositionUnit::Percent(factor) => self_size * factor / 100.,
    }
  }

  pub fn lerp_fn(self_size: f32) -> impl Fn(&Self, &Self, f32) -> Self {
    move |from, to, rate| {
      let from = from.abs_value(self_size);
      let to = to.abs_value(self_size);
      PositionUnit::Pixel(from.lerp(&to, rate))
    }
  }
}

#[cfg(test)]
mod test {
  use super::*;
  use crate::test::*;
  use PositionUnit::*;
  const CHILD_SIZE: Size = Size::new(50., 50.);
  const WND_SIZE: Size = Size::new(100., 100.);

  #[test]
  fn pixel_left_top() {
    expect_layout_result(
      widget! {
        MockBox {
          size: CHILD_SIZE,
          left_anchor: 1.,
          top_anchor: 1.,
        }
      },
      Some(WND_SIZE),
      &[
        LayoutTestItem {
          path: &[0, 0],
          expect: ExpectRect { y: Some(1.), ..<_>::default() },
        },
        LayoutTestItem {
          path: &[0, 0, 0],
          expect: ExpectRect { x: Some(1.), ..<_>::default() },
        },
      ],
    );
  }

  #[test]
  fn pixel_left_bottom() {
    expect_layout_result(
      widget! {
        MockBox {
          size: CHILD_SIZE,
          left_anchor: 1.,
          bottom_anchor: 1.,
        }
      },
      Some(WND_SIZE),
      &[
        LayoutTestItem {
          path: &[0, 0],
          expect: ExpectRect { y: Some(49.), ..<_>::default() },
        },
        LayoutTestItem {
          path: &[0, 0, 0],
          expect: ExpectRect { x: Some(1.), ..<_>::default() },
        },
      ],
    );
  }

  #[test]
  fn pixel_top_right() {
    expect_layout_result(
      widget! {
        MockBox {
          size: CHILD_SIZE,
          right_anchor: 1.,
          top_anchor: 1.,
        }
      },
      Some(WND_SIZE),
      &[
        LayoutTestItem {
          path: &[0, 0],
          expect: ExpectRect { y: Some(1.), ..<_>::default() },
        },
        LayoutTestItem {
          path: &[0, 0, 0],
          expect: ExpectRect { x: Some(49.), ..<_>::default() },
        },
      ],
    );
  }

  #[test]
  fn pixel_bottom_right() {
    expect_layout_result(
      widget! {
        MockBox {
          size: CHILD_SIZE,
          right_anchor: 1.,
          bottom_anchor: 1.,
        }
      },
      Some(WND_SIZE),
      &[
        LayoutTestItem {
          path: &[0, 0],
          expect: ExpectRect { y: Some(49.), ..<_>::default() },
        },
        LayoutTestItem {
          path: &[0, 0, 0],
          expect: ExpectRect { x: Some(49.), ..<_>::default() },
        },
      ],
    );
  }

  #[test]
  fn percent_left_top() {
    expect_layout_result(
      widget! {
        MockBox {
          size: CHILD_SIZE,
          left_anchor: Percent(10.),
          top_anchor: Percent(10.),
        }
      },
      Some(WND_SIZE),
      &[
        LayoutTestItem {
          path: &[0, 0],
          expect: ExpectRect { y: Some(5.), ..<_>::default() },
        },
        LayoutTestItem {
          path: &[0, 0, 0],
          expect: ExpectRect { x: Some(5.), ..<_>::default() },
        },
      ],
    );
  }

  #[test]
  fn percent_left_bottom() {
    expect_layout_result(
      widget! {
        MockBox {
          size: CHILD_SIZE,
          left_anchor: Percent( 10.),
          bottom_anchor: Percent( 10.),
        }
      },
      Some(WND_SIZE),
      &[
        LayoutTestItem {
          path: &[0, 0],
          expect: ExpectRect { y: Some(45.), ..<_>::default() },
        },
        LayoutTestItem {
          path: &[0, 0, 0],
          expect: ExpectRect { x: Some(5.), ..<_>::default() },
        },
      ],
    );
  }

  #[test]
  fn percent_top_right() {
    expect_layout_result(
      widget! {
        MockBox {
          size: CHILD_SIZE,
          right_anchor: Percent(10.),
          top_anchor: Percent(10.),
        }
      },
      Some(WND_SIZE),
      &[
        LayoutTestItem {
          path: &[0, 0],
          expect: ExpectRect { y: Some(5.), ..<_>::default() },
        },
        LayoutTestItem {
          path: &[0, 0, 0],
          expect: ExpectRect { x: Some(45.), ..<_>::default() },
        },
      ],
    );
  }

  #[test]
  fn percent_bottom_right() {
    expect_layout_result(
      widget! {
        MockBox {
          size: CHILD_SIZE,
          right_anchor: Percent(10.),
          bottom_anchor: Percent(10.),
        }
      },
      Some(WND_SIZE),
      &[
        LayoutTestItem {
          path: &[0, 0],
          expect: ExpectRect { y: Some(45.), ..<_>::default() },
        },
        LayoutTestItem {
          path: &[0, 0, 0],
          expect: ExpectRect { x: Some(45.), ..<_>::default() },
        },
      ],
    );
  }
}
