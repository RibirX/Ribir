use crate::prelude::*;

// todo: provide a position unit to help ribir to calc widget position in
// axises, support start/center/end, pixel,percent.

/// Describe how to anchor a widget on x-axis.
pub enum XAnchor {
  /// widget relative to left boundary.
  Left(f32),
  /// widget relative to right boundary.
  Right(f32),
}

/// Describe how to anchor a widget on y-axis.
pub enum YAnchor {
  /// widget relative to top boundary.
  Top(f32),
  /// widget relative to bottom boundary.
  Bottom(f32),
}

/// Widget use to anchor child by relative position.
#[derive(Declare, SingleChild)]
pub struct Anchor {
  // todo: use pos_x & pos_y as a builtin field
  #[declare(convert=into, default = XAnchor::Left(0.))]
  pub x: XAnchor,
  #[declare(convert=into, default = YAnchor::Top(0.))]
  pub y: YAnchor,
}

impl Render for Anchor {
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    ctx.single_child().map_or_else(Size::zero, |c| {
      let child_size = ctx.perform_child_layout(c, clamp);
      let x = match self.x {
        XAnchor::Left(l) => l,
        XAnchor::Right(r) => clamp.max.width - r - child_size.width,
      };
      let y = match self.y {
        YAnchor::Top(t) => t,
        YAnchor::Bottom(b) => clamp.max.height - b - child_size.height,
      };
      ctx.update_position(c, Point::new(x, y));
      child_size
    })
  }

  fn paint(&self, _: &mut PaintingCtx) {}
}

impl Query for Anchor {
  crate::impl_query_self_only!();
}

impl From<f32> for XAnchor {
  #[inline]
  fn from(v: f32) -> Self { XAnchor::Left(v) }
}

impl From<f32> for YAnchor {
  #[inline]
  fn from(v: f32) -> Self { YAnchor::Top(v) }
}

#[cfg(test)]
mod test {
  use super::*;
  use crate::test::widget_and_its_children_box_rect;
  const CHILD_SIZE: Size = Size::new(50., 50.);
  const WND_SIZE: Size = Size::new(100., 100.);

  fn sample_test(x: XAnchor, y: YAnchor, pos: Point) {
    let w = widget! {
      Anchor { x, y, SizedBox { size: CHILD_SIZE }}
    };
    let (rect, child) = widget_and_its_children_box_rect(w, WND_SIZE);

    assert_eq!(rect, CHILD_SIZE.into());
    assert_eq!(child.len(), 1);
    assert_eq!(child[0], Rect::new(pos, CHILD_SIZE));
  }

  #[test]
  fn simple_test_cases() {
    sample_test(XAnchor::Left(1.), YAnchor::Top(1.), Point::new(1., 1.));
    sample_test(XAnchor::Left(1.), YAnchor::Bottom(1.), Point::new(1., 49.));
    sample_test(XAnchor::Right(1.), YAnchor::Top(1.), Point::new(49., 1.));
    sample_test(
      XAnchor::Right(1.),
      YAnchor::Bottom(1.),
      Point::new(49., 49.),
    );
  }

  #[test]
  fn default_value() {
    let w = widget! {
      Anchor { SizedBox { size: CHILD_SIZE } }
    };
    let (_, child) = widget_and_its_children_box_rect(w, WND_SIZE);

    assert_eq!(child[0], Rect::new(Point::zero(), CHILD_SIZE));
  }
}
