use crate::prelude::*;

/// Specifies the horizontal position you want to anchor the widget.
#[derive(Debug, Clone, Copy)]
pub enum HAnchor {
  /// positions the widget's left edge x pixels to the right of the target's
  /// left edge.
  Left(f32),

  /// positions the widget's right edge x pixels to the left of the target's
  /// right edge.
  Right(f32),
}

/// Specifies the vertical position you want to anchor the widget.
#[derive(Debug, Clone, Copy)]
pub enum VAnchor {
  /// positions the widget's top edge x pixels bellow the target's top edge.
  Top(f32),

  /// positions the widget's bottom edge x pixels above the target's bottom
  /// edge.
  Bottom(f32),
}

impl HAnchor {
  pub fn map(self, f: impl FnOnce(f32) -> f32) -> Self {
    match self {
      HAnchor::Left(x) => HAnchor::Left(f(x)),
      HAnchor::Right(x) => HAnchor::Right(f(x)),
    }
  }
}

impl VAnchor {
  pub fn map(self, f: impl FnOnce(f32) -> f32) -> Self {
    match self {
      VAnchor::Top(x) => VAnchor::Top(f(x)),
      VAnchor::Bottom(x) => VAnchor::Bottom(f(x)),
    }
  }
}

impl PartialEq for HAnchor {
  fn eq(&self, other: &Self) -> bool {
    match (self, other) {
      (HAnchor::Left(x1), HAnchor::Left(x2)) => (x1 - x2).abs() < f32::EPSILON,
      (HAnchor::Right(x1), HAnchor::Right(x2)) => (x1 - x2).abs() < f32::EPSILON,
      _ => false,
    }
  }
}

impl PartialEq for VAnchor {
  fn eq(&self, other: &Self) -> bool {
    match (self, other) {
      (VAnchor::Top(y1), VAnchor::Top(y2)) => (y1 - y2).abs() < f32::EPSILON,
      (VAnchor::Bottom(y1), VAnchor::Bottom(y2)) => (y1 - y2).abs() < f32::EPSILON,
      _ => false,
    }
  }
}

#[derive(Clone, Copy, Default, PartialEq)]
pub struct Anchor {
  /// Specifies the horizontal position you want to anchor the widget, See
  /// [`HAnchor`]!. if None, the widget is anchored by the parent
  pub x: Option<HAnchor>,

  /// Specifies the vertical position you want to anchor the widget, See
  /// [`VAnchor`]! if None, the widget is anchored by the parent
  pub y: Option<VAnchor>,
}

impl Lerp for HAnchor {
  fn lerp(&self, other: &Self, t: f32) -> Self {
    match (self, other) {
      (HAnchor::Left(x1), HAnchor::Left(x2)) => HAnchor::Left(x1.lerp(x2, t)),
      (HAnchor::Right(x1), HAnchor::Right(x2)) => HAnchor::Right(x1.lerp(x2, t)),
      _ => unreachable!(),
    }
  }
}

impl Lerp for VAnchor {
  fn lerp(&self, other: &Self, t: f32) -> Self {
    match (self, other) {
      (VAnchor::Top(y1), VAnchor::Top(y2)) => VAnchor::Top(y1.lerp(y2, t)),
      (VAnchor::Bottom(y1), VAnchor::Bottom(y2)) => VAnchor::Bottom(y1.lerp(y2, t)),
      _ => unreachable!(),
    }
  }
}

impl Lerp for Anchor {
  fn lerp(&self, other: &Self, t: f32) -> Self {
    let x = match (self.x, other.x) {
      (Some(x1), Some(x2)) => Some(x1.lerp(&x2, t)),
      (Some(x1), None) => Some(x1.map(|x| x.lerp(&0., t))),
      (None, Some(x1)) => Some(x1.map(|x| 0_f32.lerp(&x, t))),
      _ => None,
    };

    let y = match (self.y, other.y) {
      (Some(y1), Some(y2)) => Some(y1.lerp(&y2, t)),
      (Some(y1), None) => Some(y1.map(|y| y.lerp(&0., t))),
      (None, Some(y1)) => Some(y1.map(|y| 0_f32.lerp(&y, t))),
      _ => None,
    };
    Self { x, y }
  }
}

impl Anchor {
  pub fn new(x: HAnchor, y: VAnchor) -> Self { Self { x: Some(x), y: Some(y) } }

  /// Return Anchor that positions the widget's left top corner to the position
  pub fn from_point(pos: Point) -> Self { Self::new(HAnchor::Left(pos.x), VAnchor::Top(pos.y)) }

  /// Return Anchor that positions the widget's left edge x pixels to the right
  /// of the target's left edge.
  pub fn left(x: f32) -> Self { Self { x: Some(HAnchor::Left(x)), y: None } }

  /// Return Anchor that positions the widget's right edge x pixels to the left
  /// of the target's right edge.
  pub fn right(x: f32) -> Self { Self { x: Some(HAnchor::Right(x)), y: None } }

  /// Return Anchor that positions the widget's top edge x pixels bellow the
  /// target's top edge.
  pub fn top(y: f32) -> Self { Self { x: None, y: Some(VAnchor::Top(y)) } }

  /// Return Anchor that positions the widget's bottom edge x pixels above the
  /// parent's bottom edge.
  pub fn bottom(y: f32) -> Self { Self { x: None, y: Some(VAnchor::Bottom(y)) } }

  /// Return Anchor that positions the widget's left top corner to the position
  /// x pixel right, y pixel bellow relative to the left top corner of
  /// the target
  pub fn left_top(x: f32, y: f32) -> Self { Self::new(HAnchor::Left(x), VAnchor::Top(y)) }

  /// Return Anchor that positions the widget's right top corner to the position
  /// x pixel left, y pixel bellow relative to the right top corner of
  /// the target
  pub fn right_top(x: f32, y: f32) -> Self { Self::new(HAnchor::Right(x), VAnchor::Top(y)) }

  /// Return Anchor that positions the widget's left bottom corner to the
  /// position x pixel right, y pixel above relative to the left bottom corner
  /// of the target
  pub fn left_bottom(x: f32, y: f32) -> Self { Self::new(HAnchor::Left(x), VAnchor::Bottom(y)) }

  /// Return Anchor that positions the widget's right bottom corner to the
  /// position x pixel left, y pixel above relative to the right bottom corner
  /// of the target
  pub fn right_bottom(x: f32, y: f32) -> Self { Self::new(HAnchor::Right(x), VAnchor::Bottom(y)) }
}

/// Widget use to anchor child constraints relative to parent widget.
#[derive(Query, SingleChild, Default)]
pub struct RelativeAnchor {
  pub anchor: Anchor,
}

impl Declare for RelativeAnchor {
  type Builder = FatObj<()>;
  #[inline]
  fn declarer() -> Self::Builder { FatObj::new(()) }
}

impl Render for RelativeAnchor {
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    let mut layouter = ctx.assert_single_child_layouter();
    let child_size = layouter.perform_widget_layout(clamp);

    let Anchor { x, y } = self.anchor;
    let x = x
      .map(|x| match x {
        HAnchor::Left(x) => x,
        HAnchor::Right(x) => clamp.max.width - child_size.width - x,
      })
      .unwrap_or_default();
    let y = y
      .map(|y| match y {
        VAnchor::Top(y) => y,
        VAnchor::Bottom(y) => clamp.max.height - child_size.height - y,
      })
      .unwrap_or_default();

    layouter.update_position(Point::new(x, y));
    child_size
  }

  fn paint(&self, _: &mut PaintingCtx) {}

  fn hit_test(&self, _: &HitTestCtx, _: Point) -> HitTest {
    HitTest { hit: false, can_hit_child: true }
  }
}

#[cfg(test)]
mod test {
  use ribir_dev_helper::*;

  use super::*;
  use crate::test_helper::*;
  const CHILD_SIZE: Size = Size::new(50., 50.);
  const WND_SIZE: Size = Size::new(100., 100.);

  fn pixel_left_top() -> impl WidgetBuilder {
    fn_widget! {
      @MockBox {
        size: CHILD_SIZE,
        anchor: Anchor::left_top(1., 1.),
      }
    }
  }
  widget_layout_test!(
    pixel_left_top,
    wnd_size = WND_SIZE,
    { path = [0, 0], y == 1., }
    { path = [0, 0], x == 1., }
  );

  fn pixel_left_bottom() -> impl WidgetBuilder {
    fn_widget! {
      @MockBox {
        size: CHILD_SIZE,
        anchor: Anchor::left_bottom(1., 1.),
      }
    }
  }
  widget_layout_test!(
    pixel_left_bottom,
    wnd_size = WND_SIZE,
    { path = [0, 0], y == 49.,}
    { path = [0, 0], x == 1., }
  );

  fn pixel_top_right() -> impl WidgetBuilder {
    fn_widget! {
      @MockBox {
        size: CHILD_SIZE,
        anchor: Anchor::right_top(1., 1.),
      }
    }
  }
  widget_layout_test!(
    pixel_top_right,
    wnd_size = WND_SIZE,
    { path = [0, 0], y == 1.,}
    { path = [0, 0], x == 49.,}
  );

  fn pixel_bottom_right() -> impl WidgetBuilder {
    fn_widget! {
      @MockBox {
        size: CHILD_SIZE,
        anchor: Anchor::right_bottom(1., 1.)
      }
    }
  }
  widget_layout_test!(
    pixel_bottom_right,
    wnd_size = WND_SIZE,
    { path = [0, 0], y == 49.,}
    { path = [0, 0], x == 49.,}
  );
}
