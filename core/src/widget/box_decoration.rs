use crate::prelude::*;

/// The BoxDecoration provides a variety of ways to draw a box.
#[derive(SingleChildWidget, Default, Clone, Declare)]
pub struct BoxDecoration {
  /// The background of the box.
  #[declare(builtin, strip_option, default)]
  pub background: Option<Brush>,
  /// A border to draw above the background
  #[declare(builtin, strip_option, default)]
  pub border: Option<Border>,
  /// The corners of this box are rounded by this `BorderRadius`. The round
  /// corner only work if the two borders beside it are same style.
  #[declare(builtin, strip_option, default)]
  pub radius: Option<Radius>,
}

#[derive(Debug, Default, Clone, PartialEq)]
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

impl BorderSide {
  #[inline]
  pub fn new(width: f32, color: Color) -> Self { Self { width, color } }
}

fn single_child<C: WidgetCtx>(ctx: &C) -> WidgetId {
  ctx
    .single_child()
    .expect("BoxDecoration must have one child.")
}

impl RenderWidget for BoxDecoration {
  #[inline]
  fn only_sized_by_parent(&self) -> bool { false }

  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    let child = single_child(ctx);
    let mut size = ctx.perform_render_child_layout(child, clamp);
    if let Some(ref border) = self.border {
      size.width += border.left.width + border.right.width;
      size.height += border.top.width + border.bottom.width;
      ctx.update_position(child, Point::new(border.left.width, border.top.width));
    }
    size
  }

  fn paint(&self, ctx: &mut PaintingCtx) {
    let child = single_child(ctx);
    let content_rect = ctx.widget_box_rect(child).unwrap();

    let painter = ctx.painter();
    if let Some(ref background) = self.background {
      painter.set_brush(background.clone());
      if let Some(radius) = &self.radius {
        painter.rect_round(&content_rect, radius);
      } else {
        painter.rect(&content_rect);
      }
      painter.fill(None);
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
impl BoxDecoration {
  #[inline]
  pub fn is_empty(&self) -> bool {
    let Self { border, background, radius }: &BoxDecoration = self;
    border.is_none() && background.is_none() && radius.is_none()
  }

  fn paint_border(&self, painter: &mut Painter, rect: &Rect) {
    // return;
    // todo: refactor border paint, we should only support radius for uniform border
    // line.
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
        .set_brush(Brush::Color(border.left.color.clone()));

      let half_border = border_width / 2.;
      let rect = rect.inflate(half_border, half_border);
      if let Some(ref radius) = self.radius {
        painter.rect_round(&rect, radius);
      } else {
        painter.rect(&rect);
      };
      painter.stroke(None, None);
    } else {
      let w = rect.width();
      let h = rect.height();
      let mut tl = 0.;
      let mut tr = 0.;
      let mut bl = 0.;
      let mut br = 0.;
      if let Some(radius) = self.radius {
        tl = radius.top_left.abs().min(w).min(h);
        tr = radius.top_right.abs().min(w).min(h);
        bl = radius.bottom_left.abs().min(w).min(h);
        br = radius.bottom_right.abs().min(w).min(h);

        if tl + tr > w {
          let shrink = (tl + tr - w) / 2.;
          tl -= shrink;
          tr -= shrink;
        }
        if bl + br > w {
          let shrink = (bl + br - w) / 2.;
          bl -= shrink;
          br -= shrink;
        }
        if tl + bl > h {
          let shrink = (tl + bl - h) / 2.;
          tl -= shrink;
          bl -= shrink;
        }
        if tr + br > h {
          let shrink = (tr + br - h) / 2.;
          tr -= shrink;
          br -= shrink;
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
                  .set_line_width(border.top.width)
                  .set_brush(border.top.color.clone())
                  .begin_path(Point::new(rect.min_x() - border.left.width, y));
              }
              if !end && tr > 0. && tr > 0. {
                let center = Point::new(max.x - tr, rect.min_y() + tr);
                painter.line_to(Point::new(max.x - tr, y)).ellipse_to(
                  center,
                  Vector::new(tr + half_right, tr + half_top),
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
                  .set_line_width(border.right.width)
                  .set_brush(border.right.color.clone())
                  .begin_path(Point::new(x, rect.min_y() - border.top.width));
              }
              if !end && br > 0. && br > 0. {
                let radius = Vector::new(br, br);
                let center = max - radius;
                painter.line_to(Point::new(x, max.y - br)).ellipse_to(
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
                  .set_line_width(border.bottom.width)
                  .set_brush(border.bottom.color.clone())
                  .begin_path(Point::new(max.x + border.right.width, y));
              }
              if !end && bl > 0. && bl > 0. {
                painter
                  .line_to(Point::new(rect.min_x() + bl, y))
                  .ellipse_to(
                    Point::new(rect.min_x() + bl, max.y - bl),
                    Vector::new(bl + half_left, bl + half_bottom),
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
                  .set_line_width(border.left.width)
                  .set_brush(border.left.color.clone())
                  .begin_path(Point::new(x, max.y + border.bottom.width));
              }

              if !end && tl > 0. && tl > 0. {
                let radius = Vector::new(tl, tl);
                painter
                  .line_to(Point::new(x, rect.min_y() + tl))
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
        painter.close_path();
        painter.stroke(None, None);
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
  fn default_value_is_none() {
    let dummy = std::mem::MaybeUninit::uninit();
    // just for test, we know BoxDecoration not use `ctx` to build.
    let mut ctx: BuildCtx<'static> = unsafe { dummy.assume_init() };
    let w = BoxDecoration::builder().build(&mut ctx);

    assert_eq!(w.border, None);
    assert_eq!(w.radius, None);
    assert_eq!(w.background, None);

    std::mem::forget(ctx);
  }

  #[test]
  fn layout() {
    const SIZE: Size = Size::new(100., 100.);
    struct T;
    impl CombinationWidget for T {
      #[widget]
      fn build(&self, ctx: &mut BuildCtx) -> BoxedWidget {
        widget! {
          SizedBox {
            size: SIZE,
            border: Border {
              left: BorderSide::new(1., Color::BLACK),
              right: BorderSide::new(2., Color::BLACK),
              top: BorderSide::new(3., Color::BLACK),
              bottom: BorderSide::new(4., Color::BLACK),
            },
          }
        }
      }
    }

    let (rect, child) = widget_and_its_children_box_rect(T.box_it(), Size::new(500., 500.));
    assert_eq!(rect, Rect::from_size(Size::new(103., 107.)));
    assert_eq!(
      child,
      vec![Rect::new(Point::new(1., 3.), Size::new(100., 100.))]
    );
  }

  #[cfg(feature = "png")]
  #[test]
  fn paint() {
    struct Paint;
    impl CombinationWidget for Paint {
      #[widget]
      fn build(&self, ctx: &mut BuildCtx) -> BoxedWidget {
        let radius_cases = vec![
          Radius::all(0.),
          Radius::all(10.),
          Radius::top_left(20.),
          Radius::top_right(20.),
          Radius::bottom_right(20.),
          Radius::bottom_left(20.),
          Radius::top_left(50.),
        ];

        widget! {
          Row {
            wrap: true,
            margin: EdgeInsets::all(2.),
            SizedBox {
              size: Size::new(60., 40.),
              background: Color::PINK,
              border: Border {
                left: BorderSide { width: 1., color: Color::BLACK },
                right: BorderSide { width: 2., color: Color::RED },
                top: BorderSide { width: 3., color: Color::GREEN },
                bottom: BorderSide { width: 4., color: Color::YELLOW },
              },
            }
            radius_cases
            .into_iter()
            .map(|radius| {
              widget!{
                SizedBox {
                  size: Size::new(60., 40.),
                  background: Color::RED,
                  radius,
                  border: Border::all(BorderSide { width: 5., color: Color::BLACK }),
                  margin: EdgeInsets::all(2.)
                }
              }
            }),
          }
        }
      }
    }

    let mut window = Window::wgpu_headless(Paint.box_it(), DeviceSize::new(400, 600));
    window.render_ready();
    assert!(window.same_as_png("../test/test_imgs/box_decoration.png"));
  }
}
