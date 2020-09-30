use crate::prelude::*;

/// The BoxDecoration provides a variety of ways to draw a box.
#[derive(Debug)]
pub struct BoxDecoration {
  pub child: BoxWidget,
  /// The background of the box.
  pub background: Option<FillStyle>,
  /// A border to draw above the background
  pub border: Option<Border>,
  /// The corners of this box are rounded by this `BorderRadius`. The round
  /// corner only work if the two borders beside it are same style.
  pub radius: Option<BorderRadius>,
}

#[derive(Debug, Default, Clone)]
pub struct Border {
  pub left: BorderSide,
  pub right: BorderSide,
  pub top: BorderSide,
  pub bottom: BorderSide,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct BorderSide {
  pub color: Color,
  pub width: f32,
}

#[derive(Debug)]
pub struct BoxDecorationRender {
  pub background: Option<FillStyle>,
  pub border: Option<Border>,
  pub radius: Option<BorderRadius>,
}

impl BoxDecoration {
  pub fn new(child: BoxWidget) -> Self {
    Self {
      child,
      border: None,
      background: None,
      radius: None,
    }
  }

  pub fn with_background(mut self, background: FillStyle) -> Self {
    self.background = Some(background);
    self
  }

  pub fn width_border(mut self, border: Border) -> Self {
    self.border = Some(border);
    self
  }

  pub fn with_border_radius(mut self, radius: BorderRadius) -> Self {
    self.radius = Some(radius);
    self
  }
}

render_widget_base_impl!(BoxDecoration);

impl RenderWidget for BoxDecoration {
  type RO = BoxDecorationRender;
  fn create_render_object(&self) -> Self::RO {
    BoxDecorationRender {
      border: self.border.clone(),
      radius: self.radius.clone(),
      background: self.background.clone(),
    }
  }
  fn take_children(&mut self) -> Option<SmallVec<[BoxWidget; 1]>> {
    Some(smallvec![std::mem::replace(
      &mut self.child,
      PhantomWidget.box_it()
    )])
  }
}

impl RenderObject for BoxDecorationRender {
  type Owner = BoxDecoration;

  #[inline]
  fn update(&mut self, _: &Self::Owner, _: &mut UpdateCtx) {}

  #[inline]
  fn only_sized_by_parent(&self) -> bool { false }

  fn perform_layout(&mut self, clamp: BoxClamp, ctx: &mut RenderCtx) -> Size {
    debug_assert_eq!(ctx.children().count(), 1);
    let mut child = ctx
      .children()
      .next()
      .expect("BoxDecoration must have one child.");
    let mut size = child.perform_layout(clamp);
    if let Some(ref border) = self.border {
      size.width += border.left.width + border.right.width;
      size.height += border.top.width + border.bottom.width;
      child.update_position(Point::new(border.left.width, border.top.width));
      size
    } else {
      size
    }
  }

  fn paint<'a>(&'a self, ctx: &mut PaintingContext<'a>) {
    let content_rect = ctx
      .children_rect()
      .next()
      .expect("BoxDecoration must have one child.");

    let painter = ctx.painter();
    if let Some(ref background) = self.background {
      painter.set_style(background.clone());
      if let Some(radius) = &self.radius {
        painter.rect_round(&content_rect, radius);
      } else {
        painter.rect(&content_rect);
      }
      painter.fill();
    }
    self.paint_border(painter, &content_rect);
  }
}

#[derive(Clone)]
enum BorderPosition {
  Top,
  Left,
  Bottom,
  Right,
}
impl BoxDecorationRender {
  fn paint_border(&self, painter: &mut Painter2D, rect: &Rect) {
    let path_to_paint = self.continues_border();
    if path_to_paint.is_empty() {
      return;
    }
    let border = self.border.as_ref().unwrap();
    // A continue rect round border.
    if path_to_paint.len() == 1 && path_to_paint[0].len() == 4 {
      let border_width = border.left.width;
      painter
        .set_line_width(border_width)
        .set_style(FillStyle::Color(border.left.color.clone()));

      let half_boder = border_width / 2.;
      let rect = rect.inflate(half_boder, half_boder);
      if let Some(ref radius) = self.radius {
        painter.rect_round(&rect, radius);
      } else {
        painter.rect(&rect);
      };
      painter.stroke();
    } else {
      let w = rect.width();
      let h = rect.height();
      let mut tl_x = 0.;
      let mut tl_y = 0.;
      let mut tr_x = 0.;
      let mut tr_y = 0.;
      let mut bl_x = 0.;
      let mut bl_y = 0.;
      let mut br_x = 0.;
      let mut br_y = 0.;
      if let Some(BorderRadius {
        top_left,
        top_right,
        bottom_left,
        bottom_right,
      }) = self.radius
      {
        tl_x = top_left.x.abs().min(w);
        tl_y = top_left.y.abs().min(h);
        tr_x = top_right.x.abs().min(w);
        tr_y = top_right.y.abs().min(h);
        bl_x = bottom_left.x.abs().min(w);
        bl_y = bottom_left.y.abs().min(h);
        br_x = bottom_right.x.abs().min(w);
        br_y = bottom_right.y.abs().min(h);
        if tl_x + tr_x > w {
          let shrink = (tl_x + tr_x - w) / 2.;
          tl_x -= shrink;
          tr_x -= shrink;
        }
        if bl_x + br_x > w {
          let shrink = (bl_x + br_x - w) / 2.;
          bl_x -= shrink;
          br_x -= shrink;
        }
        if tl_y + bl_y > h {
          let shrink = (tl_y + bl_y - h) / 2.;
          tl_y -= shrink;
          bl_y -= shrink;
        }
        if tr_y + br_y > h {
          let shrink = (tr_y + br_y - h) / 2.;
          tr_y -= shrink;
          br_y -= shrink;
        }
      }
      let max = rect.max();
      let half_left = border.left.width / 2.;
      let half_right = border.right.width / 2.;
      let half_top = border.top.width / 2.;
      let half_bottom = border.bottom.width / 2.;
      path_to_paint.iter().for_each(|path| {
        path.iter().enumerate().for_each(|(index, pos)| {
          let start = index == 0;
          let end = index == path.len() - 1;
          match pos {
            BorderPosition::Top => {
              let y = rect.min_y() - half_top;
              if start {
                painter
                  .begin_path(Point::new(rect.min_x() - border.left.width, y))
                  .set_line_width(border.top.width)
                  .set_style(border.top.color.clone());
              }
              if !end && tr_x > 0. && tr_y > 0. {
                let center = Point::new(max.x - tr_x, rect.min_y() + tr_y);
                painter.line_to(Point::new(max.x - tr_x, y)).ellipse_to(
                  center,
                  Vector::new(tr_x + half_right, tr_y + half_top),
                  Angle::degrees(270.),
                  Angle::degrees(360.),
                );
              } else {
                painter.line_to(Point::new(max.x, y));
              }
            }
            BorderPosition::Right => {
              let x = max.x + half_right;
              if start {
                painter
                  .begin_path(Point::new(x, rect.min_y() - border.top.width))
                  .set_line_width(border.right.width)
                  .set_style(border.right.color.clone());
              }
              if !end && br_x > 0. && br_y > 0. {
                let radius = Vector::new(br_x, br_y);
                let center = max - radius;
                painter.line_to(Point::new(x, max.y - br_y)).ellipse_to(
                  center,
                  radius + Vector::new(half_right, half_bottom),
                  Angle::degrees(0.),
                  Angle::degrees(90.),
                );
              } else {
                painter.line_to(Point::new(x, max.y));
              }
            }
            BorderPosition::Bottom => {
              let y = max.y + half_bottom;
              if start {
                painter
                  .begin_path(Point::new(max.x + border.right.width, y))
                  .set_line_width(border.bottom.width)
                  .set_style(border.bottom.color.clone());
              }
              if !end && bl_x > 0. && bl_y > 0. {
                painter
                  .line_to(Point::new(rect.min_x() + bl_x, y))
                  .ellipse_to(
                    Point::new(rect.min_x() + bl_x, max.y - bl_y),
                    Vector::new(bl_x + half_left, bl_y + half_bottom),
                    Angle::degrees(90.),
                    Angle::degrees(180.),
                  );
              } else {
                painter.line_to(Point::new(rect.min_x(), y));
              }
            }
            BorderPosition::Left => {
              let x = rect.min_x() - half_left;
              if start {
                painter
                  .begin_path(Point::new(x, max.y + border.bottom.width))
                  .set_line_width(border.left.width)
                  .set_style(border.left.color.clone());
              }

              if !end && tl_x > 0. && tl_y > 0. {
                let radius = Vector::new(tl_x, tl_y);
                painter
                  .line_to(Point::new(x, rect.min_y() + tl_y))
                  .ellipse_to(
                    rect.min() + radius,
                    radius + Vector::new(half_left, half_top),
                    Angle::degrees(180.),
                    Angle::degrees(270.),
                  );
              } else {
                painter.line_to(Point::new(x, rect.min_y()));
              }
            }
          }
        });
        painter.close_path().stroke();
      })
    }
  }

  fn continues_border(&self) -> Vec<Vec<BorderPosition>> {
    let mut path_to_paint = vec![];
    if let Some(border) = &self.border {
      let mut continues_border = vec![];

      if border.top.is_visible() {
        continues_border.push(BorderPosition::Top);
      }
      if border.right.is_visible() {
        if border.right != border.top && !continues_border.is_empty() {
          path_to_paint.push(continues_border.clone());
          continues_border.clear();
        }
        continues_border.push(BorderPosition::Right);
      }
      if border.bottom.is_visible() {
        if border.bottom != border.right && !continues_border.is_empty() {
          path_to_paint.push(continues_border.clone());
          continues_border.clear();
        }
        continues_border.push(BorderPosition::Bottom);
      }
      if border.left.is_visible() {
        if border.left != border.bottom && !continues_border.is_empty() {
          path_to_paint.push(continues_border.clone());
          continues_border.clear();
        }
        continues_border.push(BorderPosition::Left);
        if border.left == border.top {
          if let Some(first) = path_to_paint.first_mut() {
            continues_border.append(first);
            std::mem::swap(first, &mut continues_border);
          }
        }
      }
      if !continues_border.is_empty() {
        path_to_paint.push(continues_border);
      }
    }
    path_to_paint
  }
}

impl BorderSide {
  fn is_visible(&self) -> bool { self.width > 0. && self.color.alpha != 0. }
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
}
#[cfg(test)]
mod tests {
  use super::*;
  use crate::test::widget_and_its_children_box_rect;

  #[test]
  fn layout() {
    let size = Size::new(100., 100.);
    let sized_box = SizedBox::empty_box(size).with_border(Border {
      left: BorderSide {
        width: 1.,
        color: Color::BLACK,
      },
      right: BorderSide {
        width: 2.,
        color: Color::BLACK,
      },
      top: BorderSide {
        width: 3.,
        color: Color::BLACK,
      },
      bottom: BorderSide {
        width: 4.,
        color: Color::BLACK,
      },
    });
    let (rect, child) = widget_and_its_children_box_rect(sized_box, Size::new(500., 500.));
    assert_eq!(rect, Rect::from_size(Size::new(103., 107.)));
    assert_eq!(
      child,
      vec![Rect::new(Point::new(1., 3.), Size::new(100., 100.))]
    );
  }

  #[test]
  #[ignore = "gpu need"]
  fn paint() {
    let radius = Vector::new(20., 10.);
    let radius_cases = vec![
      BorderRadius::all(Vector::zero()),
      BorderRadius::all(Vector::new(10., 10.)),
      BorderRadius {
        top_left: radius,
        ..Default::default()
      },
      BorderRadius {
        top_right: radius,
        ..Default::default()
      },
      BorderRadius {
        bottom_right: radius,
        ..Default::default()
      },
      BorderRadius {
        bottom_left: radius,
        ..Default::default()
      },
      BorderRadius {
        top_left: Vector::new(50., 50.),
        bottom_right: Vector::new(50., 50.),
        ..Default::default()
      },
    ];
    let row = radius_cases
      .into_iter()
      .map(|radius| {
        SizedBox::empty_box(Size::new(60., 40.))
          .with_background(Color::RED.into())
          .with_border_radius(radius)
          .with_border(Border::all(BorderSide {
            width: 5.,
            color: Color::BLACK,
          }))
          .with_margin(EdgeInsets::all(2.))
          .box_it()
      })
      .collect::<Row>()
      .push(
        SizedBox::empty_box(Size::new(60., 40.))
          .with_background(Color::PINK.into())
          .with_border(Border {
            left: BorderSide {
              width: 1.,
              color: Color::BLACK,
            },
            right: BorderSide {
              width: 2.,
              color: Color::RED,
            },
            top: BorderSide {
              width: 3.,
              color: Color::GREEN,
            },
            bottom: BorderSide {
              width: 4.,
              color: Color::YELLOW,
            },
          })
          .with_margin(EdgeInsets::all(2.))
          .box_it(),
      )
      .with_wrap(true);

    let mut window = window::Window::headless(row.box_it(), DeviceSize::new(400, 600));
    window.render_ready();
    window.draw_frame();

    unit_test::assert_canvas_eq!(window.render(), "../test/test_imgs/box_decoration.png");
  }
}
