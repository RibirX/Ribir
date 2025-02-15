use std::ops::Range;

use wrap_render::WrapRender;

use super::*;

/// This widget adds a border to the host widget based on the layout size and
/// utilizes the provided `Radius` to round the corners.
#[derive(Default, Clone)]
pub struct BorderWidget {
  pub border: Border,
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

impl Declare for BorderWidget {
  type Builder = FatObj<()>;
  #[inline]
  fn declarer() -> Self::Builder { FatObj::new(()) }
}

impl Border {
  #[inline]
  pub fn all(side: BorderSide) -> Self {
    Self { left: side.clone(), right: side.clone(), top: side.clone(), bottom: side }
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

impl BorderSide {
  #[inline]
  pub fn new(width: f32, color: Brush) -> Self { Self { width, color } }
}

impl_compose_child_for_wrap_render!(BorderWidget, DirtyPhase::Layout);

impl WrapRender for BorderWidget {
  fn perform_layout(&self, mut clamp: BoxClamp, host: &dyn Render, ctx: &mut LayoutCtx) -> Size {
    let border = &self.border;
    let min =
      Size::new(border.left.width + border.right.width, border.top.width + border.bottom.width);
    clamp.min = clamp.clamp(min);
    host.perform_layout(clamp, ctx)
  }

  fn visual_box(&self, host: &dyn Render, ctx: &mut VisualCtx) -> Option<Rect> {
    let visual_box = host.visual_box(ctx);
    let size = ctx.box_size().unwrap();
    if visual_box.is_none() {
      Some(Rect::from_size(size))
    } else {
      visual_box.map(|rect| rect.union(&Rect::from_size(size)))
    }
  }

  fn paint(&self, host: &dyn Render, ctx: &mut PaintingCtx) {
    let size = ctx.box_size().unwrap();

    if !size.is_empty() {
      let (provider_ctx, mut painter) = ctx.provider_ctx_and_box_painter();
      // Connecting adjacent borders implies that the styles of the neighboring
      // borders should match. If one of the adjacent borders is absent, the corner
      // radius will align with the existing border.
      let border = &self.border;
      let first = border
        .find_visible(SidePos::Top..SidePos::Top)
        .map(|side| border.expand_continuous(side));

      if let Some(rg) = first {
        let old_brush = painter.fill_brush().clone();
        let radius = if let Some(r) = Provider::of::<Radius>(provider_ctx) {
          limited_radius(&r, size)
        } else {
          Radius::all(0.)
        };
        border.paint_continuous_borders(size, &rg, &radius, &mut painter);

        // if the first continuous border only has one side, there maybe existing
        // another border on its opposite side
        if rg.start.next() == rg.end {
          let opposite = rg.end.next();
          if let Some(side) = border.find_visible(opposite..opposite.next()) {
            border.paint_continuous_borders(size, &(side..side.next()), &radius, &mut painter);
          }
        }

        painter.set_fill_brush(old_brush);
      }
    }

    host.paint(ctx);
  }
}
fn limited_radius(radius: &Radius, size: Size) -> Radius {
  let max = size.height.min(size.width) / 2.;
  let Radius { top_left, top_right, bottom_left, bottom_right } = radius;
  Radius {
    top_left: top_left.min(max),
    top_right: top_right.min(max),
    bottom_left: bottom_left.min(max),
    bottom_right: bottom_right.min(max),
  }
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
enum SidePos {
  Top,
  Right,
  Bottom,
  Left,
}

impl SidePos {
  fn next(&self) -> Self {
    match self {
      SidePos::Top => SidePos::Right,
      SidePos::Right => SidePos::Bottom,
      SidePos::Bottom => SidePos::Left,
      SidePos::Left => SidePos::Top,
    }
  }

  fn prev(&self) -> Self {
    match self {
      SidePos::Top => SidePos::Left,
      SidePos::Right => SidePos::Top,
      SidePos::Bottom => SidePos::Right,
      SidePos::Left => SidePos::Bottom,
    }
  }
}

impl Border {
  fn paint_continuous_borders(
    &self, size: Size, rg: &Range<SidePos>, radius: &Radius, painter: &mut Painter,
  ) {
    let Range { start, end } = *rg;
    #[cfg(debug_assertions)]
    {
      let color = &self.get_side(start).color;
      let mut pos = start.next();
      while pos != end {
        assert_eq!(
          &self.get_side(start).color,
          color,
          "The continuous border should have the same color."
        );
        pos = pos.next();
      }
    }

    painter.set_fill_brush(self.get_side(start).color.clone());
    self.begin_outside_path(size, radius, start, start == end, painter);
    let mut pos = start;
    loop {
      self.paint_outside_path(size, radius, pos, painter);
      pos = pos.next();
      if pos == end {
        break;
      }
    }

    if start != end {
      // This is not a full border; the ending side needs to draw the end corner.
      self.end_corner_for_outside_path(size, radius, end.prev(), painter);
    } else {
      // To create a complete border, we first close the outside path before moving to
      // the inner path to begin.
      painter.end_path(true);
      self.begin_inner_path(size, radius, end.prev(), painter);
    }

    let mut rev_start = end.prev();
    let rev_end = start.prev();
    loop {
      self.paint_inner_path(size, radius, rev_start, painter);
      rev_start = rev_start.prev();
      if rev_start == rev_end {
        break;
      }
    }

    if start != end {
      self.end_corner_for_inner_path(size, radius, start, painter);
    }

    painter.end_path(true).fill();
  }

  fn begin_outside_path(
    &self, size: Size, radius: &Radius, pos: SidePos, full_border: bool, painter: &mut Painter,
  ) {
    match pos {
      SidePos::Top => {
        let radius = radius.top_left;
        if !full_border && self.top.width > radius {
          painter
            .begin_path(Point::new(0., self.top.width))
            .line_to(Point::new(0., radius));
        } else {
          painter.begin_path(Point::new(0., radius));
        }
      }
      SidePos::Right => {
        let radius = radius.top_right;
        if !full_border && self.right.width > radius {
          painter
            .begin_path(Point::new(size.width - self.right.width, 0.))
            .line_to(Point::new(size.width - radius, 0.));
        } else {
          painter.begin_path(Point::new(size.width - radius, 0.));
        }
      }
      SidePos::Bottom => {
        let radius = radius.bottom_right;
        if !full_border && self.bottom.width > radius {
          painter
            .begin_path(Point::new(size.width, size.height - self.bottom.width))
            .line_to(Point::new(size.width, size.height - radius));
        } else {
          painter.begin_path(Point::new(size.width, size.height - radius));
        }
      }
      SidePos::Left => {
        let radius = radius.bottom_left;
        if !full_border && self.left.width > radius {
          painter
            .begin_path(Point::new(self.left.width, size.height))
            .line_to(Point::new(radius, size.height));
        } else {
          painter.begin_path(Point::new(radius, size.height));
        }
      }
    }
  }

  fn begin_inner_path(&self, size: Size, radius: &Radius, pos: SidePos, painter: &mut Painter) {
    let Self { left, right, top, bottom } = self;
    let Radius { top_left, top_right, bottom_left, bottom_right } = *radius;
    let inner_start = match pos {
      SidePos::Top => Point::new(size.width - right.width, right.width.max(top_right)),
      SidePos::Right => {
        Point::new(size.width - right.width.max(bottom_right), size.height - bottom.width)
      }
      SidePos::Bottom => Point::new(left.width, size.height - bottom.width.max(bottom_left)),
      SidePos::Left => Point::new(left.width.max(top_left), top.width),
    };
    painter.begin_path(inner_start);
  }
  fn paint_outside_path(&self, size: Size, radius: &Radius, pos: SidePos, painter: &mut Painter) {
    match pos {
      SidePos::Top => {
        self.paint_top_left_outside_corner(radius.top_left, painter);
        painter.line_to(Point::new(size.width - radius.top_right, 0.));
      }
      SidePos::Right => {
        self.paint_top_right_outside_corner(radius.top_right, size.width, painter);
        painter.line_to(Point::new(size.width, size.height - radius.bottom_right));
      }
      SidePos::Bottom => {
        self.paint_bottom_right_outside_corner(radius.bottom_right, size, painter);
        painter.line_to(Point::new(radius.bottom_left, size.height));
      }
      SidePos::Left => {
        self.paint_bottom_left_outside_corner(radius.bottom_left, size.height, painter);
        painter.line_to(Point::new(0., radius.top_left));
      }
    };
  }

  fn paint_inner_path(&self, size: Size, radius: &Radius, pos: SidePos, painter: &mut Painter) {
    let Border { left, top, right, bottom } = self;
    let Radius { top_left, top_right, bottom_left, bottom_right } = *radius;
    match pos {
      SidePos::Top => {
        self.paint_top_right_inner_corner(top_right, size.width, painter);
        painter.line_to(Point::new(left.width.max(top_left), top.width));
      }
      SidePos::Right => {
        self.paint_bottom_right_inner_corner(bottom_right, size, painter);
        painter.line_to(Point::new(size.width - right.width, top.width.max(top_right)));
      }
      SidePos::Bottom => {
        self.paint_bottom_left_inner_corner(bottom_right, size.height, painter);
        painter.line_to(Point::new(
          size.width - right.width.max(bottom_left),
          size.height - bottom.width,
        ));
      }
      SidePos::Left => {
        self.paint_top_left_inner_corner(bottom_left, painter);
        painter.line_to(Point::new(left.width, size.height - bottom.width.max(bottom_left)));
      }
    }
  }

  fn end_corner_for_outside_path(
    &self, size: Size, radius: &Radius, pos: SidePos, painter: &mut Painter,
  ) {
    match pos {
      SidePos::Top => {
        self.paint_top_right_outside_corner(radius.top_right, size.width, painter);
        if self.top.width > radius.top_right {
          painter.line_to(Point::new(size.width, self.top.width));
        }
      }
      SidePos::Right => {
        self.paint_bottom_right_outside_corner(radius.bottom_right, size, painter);
        if self.right.width > radius.bottom_right {
          painter.line_to(Point::new(size.width - self.right.width, size.height));
        }
      }
      SidePos::Bottom => {
        self.paint_bottom_left_outside_corner(radius.bottom_left, size.height, painter);
        if self.bottom.width > radius.bottom_left {
          painter.line_to(Point::new(0., size.height - self.bottom.width));
        }
      }
      SidePos::Left => {
        self.paint_top_left_outside_corner(radius.top_left, painter);
        if self.left.width > radius.top_left {
          painter.line_to(Point::new(self.left.width, 0.));
        }
      }
    }
  }

  fn end_corner_for_inner_path(
    &self, size: Size, radius: &Radius, pos: SidePos, painter: &mut Painter,
  ) {
    let Radius { top_left, top_right, bottom_left, bottom_right } = *radius;
    let Border { left, top, right, bottom } = self;
    match pos {
      SidePos::Top => {
        self.paint_top_left_inner_corner(top_left, painter);
        if top_left > 0. && (top_left < left.width || top_left < top.width) {
          painter.line_to(Point::new(0., top.width));
        }
      }
      SidePos::Right => {
        self.paint_top_right_inner_corner(top_right, size.width, painter);
        if top_right > 0. && (top_right < right.width || top_right < top.width) {
          painter.line_to(Point::new(size.width - right.width, 0.));
        }
      }
      SidePos::Bottom => {
        self.paint_bottom_right_inner_corner(bottom_right, size, painter);
        if bottom_right > 0. && (bottom_right < right.width || bottom_right < bottom.width) {
          painter.line_to(Point::new(size.width, size.height - bottom.width));
        }
      }
      SidePos::Left => {
        self.paint_bottom_left_inner_corner(bottom_left, size.height, painter);
        if bottom_left > 0. && (bottom_left < left.width || bottom_left < bottom.width) {
          painter.line_to(Point::new(left.width, size.height));
        }
      }
    }
  }

  fn paint_top_left_outside_corner(&self, radius: f32, painter: &mut Painter) {
    if radius > 0. {
      let start_angle = Angle::pi();
      let end_angle = start_angle + Angle::frac_pi_2();
      painter.ellipse_to(Point::splat(radius), Vector::splat(radius), start_angle, end_angle);
    }
  }

  fn paint_top_right_outside_corner(&self, radius: f32, width: f32, painter: &mut Painter) {
    if radius > 0. {
      let start_angle = -Angle::frac_pi_2();
      let center = Point::new(width - radius, radius);
      painter.ellipse_to(center, Vector::splat(radius), start_angle, Angle::zero());
    }
  }

  fn paint_bottom_right_outside_corner(&self, radius: f32, size: Size, painter: &mut Painter) {
    if radius > 0. {
      let center = Point::new(size.width - radius, size.height - radius);
      painter.ellipse_to(center, Vector::splat(radius), Angle::zero(), Angle::frac_pi_2());
    }
  }

  fn paint_bottom_left_outside_corner(&self, radius: f32, height: f32, painter: &mut Painter) {
    if radius > 0. {
      let center = Point::new(radius, height - radius);
      painter.ellipse_to(center, Vector::splat(radius), Angle::frac_pi_2(), Angle::pi());
    }
  }

  fn paint_top_right_inner_corner(&self, radius: f32, width: f32, painter: &mut Painter) {
    let Border { right, top, .. } = self;
    if radius > right.width && radius > top.width {
      let center = Point::new(width - radius, radius);
      let radius = Vector::new(radius - right.width, radius - top.width);
      painter.ellipse_to(center, radius, Angle::zero(), -Angle::frac_pi_2());
    }
  }

  fn paint_top_left_inner_corner(&self, radius: f32, painter: &mut Painter) {
    let Border { left, top, .. } = self;
    if radius > left.width && radius > top.width {
      let center = Point::splat(radius);
      let radius = Vector::new(radius - left.width, radius - top.width);
      painter.ellipse_to(center, radius, -Angle::frac_pi_2(), -Angle::pi());
    }
  }

  fn paint_bottom_left_inner_corner(&self, radius: f32, height: f32, painter: &mut Painter) {
    let Border { left, bottom, .. } = self;
    if radius > left.width && radius > bottom.width {
      let center = Point::new(radius, height - radius);
      let radius = Vector::new(radius - left.width, radius - bottom.width);
      painter.ellipse_to(center, radius, Angle::pi(), Angle::frac_pi_2());
    }
  }

  fn paint_bottom_right_inner_corner(&self, radius: f32, size: Size, painter: &mut Painter) {
    let Border { right, bottom, .. } = self;
    if radius > right.width && radius > bottom.width {
      let center = Point::new(size.width - radius, size.height - radius);
      let radius = Vector::new(radius - right.width, radius - bottom.width);
      painter.ellipse_to(center, radius, Angle::frac_pi_2(), Angle::zero());
    }
  }
  fn find_visible(&self, rg: Range<SidePos>) -> Option<SidePos> {
    let Range { mut start, end } = rg;
    loop {
      if self.get_side(start).width > 0. {
        return Some(start);
      } else {
        start = start.next();
      }
      if start == end {
        return None;
      }
    }
  }

  fn expand_continuous(&self, pos: SidePos) -> Range<SidePos> {
    let (mut start, mut end) = (pos, pos);

    loop {
      let prev = start.prev();
      if prev != end && self.get_side(prev).width > 0. {
        start = prev;
      } else {
        break;
      }
    }
    loop {
      let next = end.next();
      if next != start && self.get_side(next).width > 0. {
        end = next;
      } else {
        break;
      }
    }

    start..end.next()
  }

  fn get_side(&self, pos: SidePos) -> &BorderSide {
    match pos {
      SidePos::Top => &self.top,
      SidePos::Right => &self.right,
      SidePos::Bottom => &self.bottom,
      SidePos::Left => &self.left,
    }
  }
}

#[cfg(test)]
mod tests {
  use ribir_dev_helper::*;

  use super::*;
  use crate::{reset_test_env, test_helper::*};

  const SIZE: Size = Size::new(100., 100.);

  widget_layout_test!(
    with_border,
    WidgetTester::new(fn_widget! {
      @MockBox {
        size: SIZE,
        border: Border {
          left: BorderSide::new(1., Color::BLACK.into()),
          right: BorderSide::new(2., Color::BLACK.into()),
          top: BorderSide::new(3., Color::BLACK.into()),
          bottom: BorderSide::new(4., Color::BLACK.into()),
        },
      }
    }),
    LayoutCase::default().with_size(Size::new(100., 100.)),
    LayoutCase::new(&[0]).with_rect(ribir_geom::rect(0., 0., 100., 100.))
  );

  fn border_100_50_box(
    top: f32, right: f32, bottom: f32, left: f32, radius: Option<Radius>,
  ) -> Widget<'static> {
    let brush: Brush = Color::RED.with_alpha(0.5).into();
    fn_widget! {
      let mut mock_box = @MockBox { size: Size::new(100., 50.) };
      if let Some(radius) = radius {
        mock_box = mock_box.radius(radius);
      }
      @ $mock_box {
        margin: EdgeInsets::all(10.),
        background: Color::GRAY.with_alpha(0.5),
        border: Border {
          left: BorderSide::new(left, brush.clone()),
          right: BorderSide::new(right, brush.clone()),
          top: BorderSide::new(top, brush.clone()),
          bottom: BorderSide::new(bottom, brush),
        },
      }
    }
    .into_widget()
  }

  #[test]
  #[cfg(not(target_arch = "wasm32"))]
  fn single_borders() {
    reset_test_env!();

    assert_widget_eq_image!(
      WidgetTester::new(mock_multi! {
        // only top
        @ { border_100_50_box(10., 0., 0., 0., None) }
        // top with large radius
        @ { border_100_50_box(10., 0., 0., 0., Some(Radius::all(100.))) }
        // top with small radius
        @ { border_100_50_box(10., 0., 0., 0., Some(Radius::all(5.))) }
      })
      .with_wnd_size(Size::new(400., 80.))
      .with_comparison(0.000065),
      "top_borders"
    );

    assert_widget_eq_image!(
      WidgetTester::new(mock_multi! {
        // only right
        @ { border_100_50_box(0., 10., 0., 0., None) }
        // right with large radius
        @ { border_100_50_box(0., 10., 0., 0., Some(Radius::all(100.))) }
        // right with small radius
        @ { border_100_50_box(0., 10., 0., 0., Some(Radius::all(5.))) }
      })
      .with_wnd_size(Size::new(400., 80.))
      .with_comparison(0.000065),
      "right_borders"
    );

    assert_widget_eq_image!(
      WidgetTester::new(mock_multi! {
        // only bottom
        @ { border_100_50_box(0., 0., 10., 0., None) }
        // bottom with large radius
        @ { border_100_50_box(0., 0., 10., 0., Some(Radius::all(100.))) }
        // bottom with small radius
        @ { border_100_50_box(0., 0., 10., 0., Some(Radius::all(5.))) }
      })
      .with_wnd_size(Size::new(400., 80.))
      .with_comparison(0.000065),
      "bottom_borders"
    );

    assert_widget_eq_image!(
      WidgetTester::new(mock_multi! {
        // only left
        @ { border_100_50_box(0., 0., 0., 10., None) }
        // left with large radius
        @ { border_100_50_box(0., 0., 0., 10., Some(Radius::all(100.))) }
        // left with small radius
        @ { border_100_50_box(0., 0., 0., 10., Some(Radius::all(5.))) }
      })
      .with_wnd_size(Size::new(400., 80.))
      .with_comparison(0.000065),
      "left_borders"
    );
  }

  #[test]
  #[cfg(not(target_arch = "wasm32"))]
  fn two_borders() {
    reset_test_env!();

    assert_widget_eq_image!(
      WidgetTester::new(mock_multi! {
        // top and bottom
        @ { border_100_50_box(10., 0., 10., 0., None) }
        // top and bottom with large radius
        @ { border_100_50_box(10., 0., 10., 0., Some(Radius::all(100.))) }
        // top and bottom with small radius
        @ { border_100_50_box(10., 0., 10., 0., Some(Radius::all(5.))) }
      })
      .with_wnd_size(Size::new(400., 80.))
      .with_comparison(0.000065),
      "top_and_bottom_borders"
    );

    assert_widget_eq_image!(
      WidgetTester::new(mock_multi! {
        // left and right
        @ { border_100_50_box(0., 10., 0., 10., None) }
        // left and right with large radius
        @ { border_100_50_box(0., 10., 0., 10., Some(Radius::all(100.))) }
        // left and right with small radius
        @ { border_100_50_box(0., 10., 0., 10., Some(Radius::all(5.))) }
      })
      .with_wnd_size(Size::new(400., 80.))
      .with_comparison(0.000065),
      "left_and_right_borders"
    );

    assert_widget_eq_image!(
      WidgetTester::new(mock_multi! {
        // top left
        @ { border_100_50_box(10., 0., 0., 10., None) }
        // top left with large radius
        @ { border_100_50_box(10., 0., 0., 10., Some(Radius::all(100.))) }
        // top left with small radius
        @ { border_100_50_box(10., 0., 0., 10., Some(Radius::all(5.))) }
      })
      .with_wnd_size(Size::new(400., 80.))
      .with_comparison(0.000065),
      "top_left_borders"
    );

    assert_widget_eq_image!(
      WidgetTester::new(mock_multi! {
        // top right
        @ { border_100_50_box(10., 10., 0., 0., None) }
        // top right with large radius
        @ { border_100_50_box(10., 10., 0., 0., Some(Radius::all(100.))) }
        // top right with small radius
        @ { border_100_50_box(10., 10., 0., 0., Some(Radius::all(5.))) }
      })
      .with_wnd_size(Size::new(400., 80.))
      .with_comparison(0.000065),
      "top_right_borders"
    );

    assert_widget_eq_image!(
      WidgetTester::new(mock_multi! {
        // right bottom
        @ { border_100_50_box(0., 10., 10., 0., None) }
        // right bottom with large radius
        @ { border_100_50_box(0., 10., 10., 0., Some(Radius::all(100.))) }
        // right bottom with small radius
        @ { border_100_50_box(0., 10., 10., 0., Some(Radius::all(5.))) }
      })
      .with_wnd_size(Size::new(400., 80.))
      .with_comparison(0.000065),
      "right_bottom_borders"
    );

    assert_widget_eq_image!(
      WidgetTester::new(mock_multi! {
        // bottom left
        @ { border_100_50_box(0., 0., 10., 10., None) }
        // bottom left with large radius
        @ { border_100_50_box(0., 0., 10., 10., Some(Radius::all(100.))) }
        // bottom left with small radius
        @ { border_100_50_box(0., 0., 10., 10., Some(Radius::all(5.))) }
      })
      .with_wnd_size(Size::new(400., 80.))
      .with_comparison(0.000065),
      "bottom_left_borders"
    );
  }

  #[test]
  #[cfg(not(target_arch = "wasm32"))]
  fn triple_borders() {
    reset_test_env!();

    assert_widget_eq_image!(
      WidgetTester::new(mock_multi! {
       // top left and right
        @ { border_100_50_box(10., 10., 10., 0., None) }
        // top left and right with large radius
        @ { border_100_50_box(10., 10., 10., 0., Some(Radius::all(100.))) }
        // top left and right with small radius
        @ { border_100_50_box(10., 10., 10., 0., Some(Radius::all(5.))) }
      })
      .with_wnd_size(Size::new(400., 80.))
      .with_comparison(0.000065),
      "top_left_and_right_borders"
    );

    assert_widget_eq_image!(
      WidgetTester::new(mock_multi! {
        // right bottom and left
        @ { border_100_50_box(0., 10., 10., 10., None) }
        // right bottom and left with large radius
        @ { border_100_50_box(0., 10., 10., 10., Some(Radius::all(100.))) }
        // right bottom and left with small radius
        @ { border_100_50_box(0., 10., 10., 10., Some(Radius::all(5.))) }
      })
      .with_wnd_size(Size::new(400., 80.))
      .with_comparison(0.000065),
      "right_bottom_and_left_borders"
    );

    assert_widget_eq_image!(
      WidgetTester::new(mock_multi! {
        // bottom left and top
        @ { border_100_50_box(10., 0., 10., 10., None) }
        // bottom left and top with large radius
        @ { border_100_50_box(10., 0., 10., 10., Some(Radius::all(100.))) }
        // bottom left and top with small radius
        @ { border_100_50_box(10., 0., 10., 10., Some(Radius::all(5.))) }
      })
      .with_wnd_size(Size::new(400., 80.))
      .with_comparison(0.000065),
      "bottom_left_and_top_borders"
    );
  }

  #[test]
  #[cfg(not(target_arch = "wasm32"))]
  fn all_borders() {
    reset_test_env!();

    assert_widget_eq_image!(
      WidgetTester::new(mock_multi! {
        // all
        @ { border_100_50_box(10., 10., 10., 10., None) }
        // all with large radius
        @ { border_100_50_box(10., 10., 10., 10., Some(Radius::all(100.))) }
        // all with small radius
        @ { border_100_50_box(10., 10., 10., 10., Some(Radius::all(5.))) }
      })
      .with_wnd_size(Size::new(400., 80.))
      .with_comparison(0.000065),
      "all_borders"
    );
  }
}
