use crate::prelude::*;

#[derive(Debug, Copy, Clone, Default, PartialEq, Lerp)]
pub struct EdgeInsets {
  pub left: f32,
  pub right: f32,
  pub bottom: f32,
  pub top: f32,
}

/// The widget utilizes empty space to surround the child widget.
///
/// ```
/// use ribir::prelude::*;
///
/// let _padding = text! {
///   text: "Background includes the empty space",
///   padding: EdgeInsets::all(10.),
///   background: Color::GREEN,
/// };
///
/// let _margin = text! {
///   text: "Background does not include the empty space",
///   margin: EdgeInsets::all(10.),
///   background: Color::GREEN,
/// };
/// ```
#[derive(SingleChild, Default, Clone, PartialEq)]
pub struct Margin {
  pub margin: EdgeInsets,
}

impl Declare for Margin {
  type Builder = FatObj<()>;
  #[inline]
  fn declarer() -> Self::Builder { FatObj::new(()) }
}

impl Render for Margin {
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    let Some(child) = ctx.single_child() else { return clamp.min };

    // Reset child position before layout
    ctx.update_position(child, Point::zero());

    let thickness = self.margin.thickness().min(clamp.max);
    let min = (clamp.min - thickness).max(ZERO_SIZE);
    let max = (clamp.max - thickness).max(ZERO_SIZE);

    // Shrink the clamp of child.
    let child_clamp = BoxClamp { min, max };
    let size = ctx.perform_child_layout(child, child_clamp);
    let pos = ctx.position(child).unwrap();
    let pos = pos + Vector::new(self.margin.left, self.margin.top);
    ctx.update_position(child, pos);

    size + thickness
  }
}

impl Margin {
  #[inline]
  pub fn new(margin: EdgeInsets) -> Self { Self { margin } }
}

impl EdgeInsets {
  pub const ZERO: Self = Self { top: 0., right: 0., bottom: 0., left: 0. };

  #[inline]
  pub const fn all(value: f32) -> Self { Self::new(value, value, value, value) }

  #[inline]
  pub const fn new(top: f32, right: f32, bottom: f32, left: f32) -> Self {
    Self { top, right, bottom, left }
  }

  #[inline]
  pub const fn only_left(left: f32) -> Self { Self { left, ..Self::ZERO } }

  #[inline]
  pub const fn only_right(right: f32) -> Self { Self { right, ..Self::ZERO } }
  #[inline]
  pub const fn only_bottom(bottom: f32) -> Self { Self { bottom, ..Self::ZERO } }

  #[inline]
  pub const fn only_top(top: f32) -> Self { Self { top, ..Self::ZERO } }

  #[inline]
  pub const fn symmetrical(vertical: f32, horizontal: f32) -> Self {
    Self { top: vertical, bottom: vertical, left: horizontal, right: horizontal }
  }

  #[inline]
  pub const fn vertical(vertical: f32) -> Self {
    Self { top: vertical, bottom: vertical, ..Self::ZERO }
  }

  #[inline]
  pub const fn horizontal(horizontal: f32) -> Self {
    Self { left: horizontal, right: horizontal, ..Self::ZERO }
  }

  #[inline]
  pub const fn with_top(mut self, top: f32) -> Self {
    self.top = top;
    self
  }

  #[inline]
  pub const fn with_right(mut self, right: f32) -> Self {
    self.right = right;
    self
  }

  #[inline]
  pub const fn with_bottom(mut self, bottom: f32) -> Self {
    self.bottom = bottom;
    self
  }

  #[inline]
  pub const fn with_left(mut self, left: f32) -> Self {
    self.left = left;
    self
  }

  pub const fn thickness(&self) -> Size {
    Size::new(self.right + self.left, self.bottom + self.top)
  }

  /// Convert to an array by the top, right, bottom, left order.
  #[inline]
  pub const fn to_array(&self) -> [f32; 4] { [self.top, self.right, self.bottom, self.left] }
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
  use ribir_dev_helper::*;

  use super::*;
  use crate::test_helper::*;

  widget_layout_test!(
    smoke,
    WidgetTester::new(fn_widget! {
      @MockBox {
        margin: EdgeInsets::symmetrical(1., 1.),
        size: Size::new(100., 100.)
      }
    })
    .with_wnd_size(Size::new(200., 200.)),
    LayoutCase::default().with_size(Size::new(102., 102.)),
    LayoutCase::new(&[0, 0]).with_rect(ribir_geom::rect(1., 1.0, 100., 100.))
  );
}
