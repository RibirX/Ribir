use ribir_core::prelude::*;

/// A layout widget that organizes its children in a horizontal line and aligns
/// them at the center along the vertical axes.
///
/// If the parent necessitates a minimum width greater than the total width of
/// the children, the children will also be centered along the horizontal axis.
///
/// This layout is a streamlined alternative to [Row](super::Row) without any
/// additional configurations. It is designed for straightforward and typical
/// use cases. For more customization options, consider using [Row](super::Row).
#[derive(Declare, MultiChild)]
pub struct HorizontalLine;

/// A layout widget that organizes its children in a vertical line and aligns
/// them at the center along both the vertical and horizontal axes.
///
/// If the parent necessitates a minimum height greater than the total height of
/// the children, the children will also be centered along the vertical axis.
///
/// This layout is a streamlined alternative to [Column](super::Column) without
/// any additional configurations. It is designed for straightforward and
/// typical use cases. For more customization options, consider using
/// [Column](super::Column).
#[derive(Declare, MultiChild)]
pub struct VerticalLine;

impl Render for HorizontalLine {
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    let mut size = ZERO_SIZE;
    let child_clamp = clamp.with_min_size(ZERO_SIZE);
    let (ctx, children) = ctx.split_children();
    for c in children {
      let clamp = child_clamp.with_max_width(child_clamp.max.width - size.width);
      let child_size = ctx.perform_child_layout(c, clamp);
      size.width += child_size.width;
      size.height = size.height.max(child_size.height);
    }

    let clamped_size = clamp.clamp(size);

    let (ctx, children) = ctx.split_children();
    let mut x = (clamped_size.width - size.width) / 2.;
    for c in children {
      let c_size = ctx.widget_box_size(c).unwrap();
      let y = (clamped_size.height - c_size.height) / 2.;
      ctx.update_position(c, Point::new(x, y));
      x += c_size.width;
    }

    clamped_size
  }
}

impl Render for VerticalLine {
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    let mut size = ZERO_SIZE;
    let child_clamp = clamp.with_min_size(ZERO_SIZE);
    let (ctx, children) = ctx.split_children();
    for c in children {
      let clamp = child_clamp.with_max_height(child_clamp.max.height - size.height);
      let child_size = ctx.perform_child_layout(c, clamp);
      size.width = size.width.max(child_size.width);
      size.height += child_size.height;
    }

    let clamped_size = clamp.clamp(size);

    let (ctx, children) = ctx.split_children();
    let mut y = (clamped_size.height - size.height) / 2.;
    for c in children {
      let c_size = ctx.widget_box_size(c).unwrap();
      let x = (clamped_size.width - c_size.width) / 2.;
      ctx.update_position(c, Point::new(x, y));
      y += c_size.height;
    }

    clamped_size
  }
}

#[cfg(test)]
mod tests {
  use ribir_core::test_helper::*;
  use ribir_dev_helper::*;

  use super::*;
  use crate::layout::*;

  widget_test_suit!(
    horizontal_line,
    WidgetTester::new(horizontal_line! {
      @SizedBox { size: Size::new(10., 10.), background: Color::RED }
      @SizedBox { size: Size::new(10., 20.), background: Color::GREEN }
      @SizedBox { size: Size::new(10., 30.), background: Color::BLUE }
    })
    .with_wnd_size(Size::new(30., 30.)),
    LayoutCase::default().with_size(Size::new(30., 30.))
  );

  widget_test_suit!(
    vertical_line,
    WidgetTester::new(vertical_line! {
      @SizedBox { size: Size::new(10., 10.), background: Color::RED }
      @SizedBox { size: Size::new(20., 10.), background: Color::GREEN }
      @SizedBox { size: Size::new(30., 10.), background: Color::BLUE }
    })
    .with_wnd_size(Size::new(30., 30.)),
    LayoutCase::default().with_size(Size::new(30., 30.))
  );
}
