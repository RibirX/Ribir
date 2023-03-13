use crate::{impl_query_self_only, prelude::*};

/// The BoxDecoration provides a variety of ways to draw a box.
#[derive(SingleChild, Default, Clone, Declare)]
pub struct BoxDecoration {
  /// The background of the box.
  #[declare(builtin, default, convert=custom)]
  pub background: Option<Brush>,
  /// A border to draw above the background
  #[declare(builtin, default, convert=strip_option)]
  pub border: Option<Border>,
  /// The corners of this box are rounded by this `BorderRadius`. The round
  /// corner only work if the two borders beside it are same style.
  #[declare(builtin, default, convert=strip_option)]
  pub border_radius: Option<Radius>,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Border {
  pub left: BorderSide,
  pub right: BorderSide,
  pub top: BorderSide,
  pub bottom: BorderSide,
}

#[derive(Debug, Default, Clone, PartialEq, Lerp)]
pub struct BorderSide {
  pub color: Brush,
  pub width: f32,
}

impl BorderSide {
  #[inline]
  pub fn new(width: f32, color: Brush) -> Self { Self { width, color } }
}

impl Render for BoxDecoration {
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    let mut layouter = ctx.assert_single_child_layouter();
    let mut size = layouter.perform_widget_layout(clamp);
    if let Some(ref border) = self.border {
      size.width += border.left.width + border.right.width;
      size.height += border.top.width + border.bottom.width;
      layouter.update_position(Point::new(border.left.width, border.top.width));
    }
    size
  }

  fn paint(&self, ctx: &mut PaintingCtx) {
    let child = ctx.assert_single_child();
    if let Some(content_rect) = ctx.widget_box_rect(child) {
      let painter = ctx.painter();
      if let Some(ref background) = self.background {
        painter.set_brush(background.clone());
        if let Some(radius) = &self.border_radius {
          painter.rect_round(&content_rect, radius);
        } else {
          painter.rect(&content_rect);
        }
        painter.fill();
      }
      self.paint_border(painter, &content_rect);
    }
  }
}

impl Query for BoxDecoration {
  impl_query_self_only!();
}

pub trait IntoBackground<M> {
  fn into_background(self) -> Option<Brush>;
}

impl<T: Into<Brush>> IntoBackground<Brush> for T {
  #[inline]
  fn into_background(self) -> Option<Brush> { Some(self.into()) }
}

impl IntoBackground<Option<Brush>> for Option<Brush> {
  #[inline]
  fn into_background(self) -> Option<Brush> { self }
}

impl BoxDecorationDeclarer {
  #[inline]
  pub fn background<M>(mut self, b: impl IntoBackground<M>) -> Self {
    self.background = Some(b.into_background());
    self
  }
}

impl BoxDecoration {
  #[inline]
  pub fn set_declare_background<M>(&mut self, b: impl IntoBackground<M>) {
    self.background = b.into_background();
  }
}

impl BoxDecoration {
  fn paint_border(&self, painter: &mut Painter, rect: &Rect) {
    if self.border.is_none() {
      return;
    }
    let border = self.border.as_ref().unwrap();
    if let Some(radius) = &self.border_radius {
      self.paint_round_border(painter, radius, border, rect);
    } else {
      self.paint_rect_border(painter, border, rect);
    }
  }

  fn is_border_uniform(&self) -> bool {
    self.border.as_ref().map_or(true, |border| {
      border.top == border.left && border.top == border.right && border.top == border.bottom
    })
  }

  fn paint_round_border(
    &self,
    painter: &mut Painter,
    radius: &Radius,
    border: &Border,
    content_rect: &Rect,
  ) {
    assert!(
      self.is_border_uniform(),
      "radius can't be setted with different border"
    );
    let width_half = border.left.width / 2.;
    let min_x = content_rect.min_x() - width_half;
    let max_x = content_rect.max_x() + width_half;
    let min_y = content_rect.min_y() - width_half;
    let max_y = content_rect.max_y() + width_half;
    let radius = Radius::new(
      radius.top_left + width_half,
      radius.top_right + width_half,
      radius.bottom_left + width_half,
      radius.bottom_right + width_half,
    );

    painter
      .set_line_width(border.top.width)
      .set_brush(border.top.color.clone());
    painter.rect_round(
      &Rect::new(
        Point::new(min_x, min_y),
        Size::new(max_x - min_x, max_y - min_y),
      ),
      &radius,
    );
    painter.stroke();
  }

  fn paint_rect_border(&self, painter: &mut Painter, border: &Border, content_rect: &Rect) {
    let min_x = content_rect.min_x() - border.left.width;
    let max_x = content_rect.max_x() + border.right.width;
    let min_y = content_rect.min_y() - border.top.width;
    let max_y = content_rect.max_y() + border.bottom.width;
    let vertexs = [
      Point::new(min_x, min_y), // lt
      Point::new(max_x, min_y), // rt
      Point::new(max_x, max_y), // rb
      Point::new(min_x, max_y), // lb
    ];
    let edges = [(0, 1), (1, 2), (2, 3), (3, 0)];
    let borders = [&border.top, &border.right, &border.bottom, &border.left];
    let borders_offset = [
      Size::new(0., border.top.width / 2.),
      Size::new(-border.right.width / 2., 0.),
      Size::new(0., -border.bottom.width / 2.),
      Size::new(border.left.width / 2., 0.),
    ];
    edges
      .iter()
      .zip(borders.iter())
      .zip(borders_offset.iter())
      .for_each(|((edge, border), offset)| {
        if border.is_visible() {
          painter
            .set_line_width(border.width)
            .set_brush(border.color.clone());
          painter.begin_path(vertexs[edge.0] + *offset);
          painter.line_to(vertexs[edge.1] + *offset);
          painter.end_path(false).stroke();
        }
      });
  }
}

impl BorderSide {
  fn is_visible(&self) -> bool {
    match self.color {
      Brush::Color(color) => color.alpha > 0 && self.width > 0.,
      _ => true,
    }
  }
}

impl Border {
  #[inline]
  pub fn all(side: BorderSide) -> Self {
    Self {
      left: side.clone(),
      right: side.clone(),
      top: side.clone(),
      bottom: side,
    }
  }

  #[inline]
  pub fn only_left(left: BorderSide) -> Self { Self { left, ..Default::default() } }

  #[inline]
  pub fn only_right(right: BorderSide) -> Self { Self { right, ..Default::default() } }

  #[inline]
  pub fn only_bottom(bottom: BorderSide) -> Self { Self { bottom, ..Default::default() } }

  #[inline]
  pub fn only_top(top: BorderSide) -> Self { Self { top, ..Default::default() } }

  #[inline]
  pub fn none() -> Self { Self { ..Default::default() } }
}
#[cfg(test)]
mod tests {
  use super::*;
  use crate::test::*;

  #[test]
  fn default_value_is_none() {
    let dummy = std::mem::MaybeUninit::uninit();
    // just for test, we know BoxDecoration not use `ctx` to build.
    let ctx: BuildCtx<'static> = unsafe { dummy.assume_init() };
    let w = BoxDecoration::declare_builder().build(&ctx);

    assert_eq!(w.border, None);
    assert_eq!(w.border_radius, None);
    assert_eq!(w.background, None);
  }

  #[test]
  fn layout() {
    const SIZE: Size = Size::new(100., 100.);
    let w = widget! {
      MockBox {
        size: SIZE,
        border: Border {
          left: BorderSide::new(1., Color::BLACK.into()),
          right: BorderSide::new(2., Color::BLACK.into()),
          top: BorderSide::new(3., Color::BLACK.into()),
          bottom: BorderSide::new(4., Color::BLACK.into()),
        },
      }
    };

    expect_layout_result(
      w,
      None,
      &[
        LayoutTestItem {
          path: &[0],
          expect: ExpectRect::from_size(Size::new(103., 107.)),
        },
        LayoutTestItem {
          path: &[0, 0],
          expect: ExpectRect::new(1., 3., 100., 100.),
        },
      ],
    );
  }
}
