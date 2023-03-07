use crate::{impl_query_self_only, prelude::*};

#[derive(Debug, Copy, Clone, Default, PartialEq, Lerp)]
pub struct EdgeInsets {
  pub left: f32,
  pub right: f32,
  pub bottom: f32,
  pub top: f32,
}

/// A widget that create space around its child.
#[derive(SingleChild, Default, Clone, PartialEq, Declare)]
pub struct Margin {
  #[declare(builtin, default)]
  pub margin: EdgeInsets,
}

impl Render for Margin {
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    let thickness = self.margin.thickness();
    let zero = Size::zero();
    let min = (clamp.min - thickness).max(zero);
    let max = (clamp.max - thickness).max(zero);
    let child_clamp = BoxClamp { min, max };

    let mut layouter = ctx.assert_single_child_layouter();
    let size = layouter.perform_widget_layout(child_clamp);
    layouter.update_position(Point::new(self.margin.left, self.margin.top));

    size + thickness
  }

  #[inline]
  fn only_sized_by_parent(&self) -> bool { false }

  #[inline]
  fn paint(&self, _: &mut PaintingCtx) {}
}

impl Query for Margin {
  impl_query_self_only!();
}

impl Margin {
  #[inline]
  pub fn new(margin: EdgeInsets) -> Self { Self { margin } }
}

impl EdgeInsets {
  #[inline]
  pub fn all(value: f32) -> Self {
    Self {
      top: value,
      left: value,
      bottom: value,
      right: value,
    }
  }

  #[inline]
  pub fn only_left(left: f32) -> Self { Self { left, ..Default::default() } }

  #[inline]
  pub fn only_right(right: f32) -> Self { Self { right, ..Default::default() } }
  #[inline]
  pub fn only_bottom(bottom: f32) -> Self { Self { bottom, ..Default::default() } }

  #[inline]
  pub fn only_top(top: f32) -> Self { Self { top, ..Default::default() } }

  #[inline]
  pub fn symmetrical(vertical: f32, horizontal: f32) -> Self {
    Self {
      top: vertical,
      bottom: vertical,
      left: horizontal,
      right: horizontal,
    }
  }

  #[inline]
  pub fn vertical(vertical: f32) -> Self {
    Self {
      top: vertical,
      bottom: vertical,
      ..Default::default()
    }
  }

  #[inline]
  pub fn horizontal(horizontal: f32) -> Self {
    Self {
      left: horizontal,
      right: horizontal,
      ..Default::default()
    }
  }

  pub fn thickness(&self) -> Size { Size::new(self.right + self.left, self.bottom + self.top) }

  /// Convert to an array by the top, right, bottom, left order.
  #[inline]
  pub fn to_array(&self) -> [f32; 4] { [self.top, self.right, self.bottom, self.left] }
}

impl std::ops::Add for EdgeInsets {
  type Output = Self;

  #[inline]
  fn add(mut self, rhs: Self) -> Self::Output {
    self += rhs;
    self
  }
}

impl std::ops::AddAssign for EdgeInsets {
  fn add_assign(&mut self, rhs: Self) {
    self.left += rhs.left;
    self.right += rhs.right;
    self.bottom += rhs.bottom;
    self.top += rhs.top;
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test::*;

  #[test]
  fn smoke() {
    let widget = widget! {
      MockBox {
        margin: EdgeInsets::symmetrical(1., 1.),
        size: Size::new(100., 100.)
      }
    };

    expect_layout_result(
      widget,
      Some(Size::new(200., 200.)),
      &[
        LayoutTestItem {
          path: &[0],
          expect: ExpectRect::from_size(Size::new(102., 102.)),
        },
        LayoutTestItem {
          path: &[0, 0],
          expect: ExpectRect::new(1., 1.0, 100., 100.),
        },
      ],
    );
  }
}
