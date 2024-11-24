use crate::{prelude::*, wrap_render::*};

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

/// This widget is used to anchor child constraints relative to the parent
/// widget.
///
/// It's important to note that if you anchor the child widget outside of its
/// parent, it may become unable to click, so ensure there is ample space within
/// the parent.
#[derive(Default)]
pub struct RelativeAnchor {
  pub anchor: Anchor,
}

impl Declare for RelativeAnchor {
  type Builder = FatObj<()>;
  #[inline]
  fn declarer() -> Self::Builder { FatObj::new(()) }
}

impl_compose_child_for_wrap_render!(RelativeAnchor);

impl WrapRender for RelativeAnchor {
  fn perform_layout(&self, clamp: BoxClamp, host: &dyn Render, ctx: &mut LayoutCtx) -> Size {
    ctx.update_position(ctx.widget_id(), Point::zero());
    let child_size = host.perform_layout(clamp, ctx);

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

    let pos = ctx.box_pos().unwrap_or_default();
    ctx.update_position(ctx.widget_id(), pos + Size::new(x, y));
    child_size
  }
}

impl From<Point> for Anchor {
  fn from(value: Point) -> Self {
    Anchor { x: Some(HAnchor::Left(value.x)), y: Some(VAnchor::Top(value.y)) }
  }
}

#[cfg(test)]
mod test {
  use ribir_dev_helper::*;

  use super::*;
  use crate::test_helper::*;
  const CHILD_SIZE: Size = Size::new(50., 50.);
  const WND_SIZE: Size = Size::new(100., 100.);

  fn widget_tester(anchor: Anchor) -> WidgetTester {
    WidgetTester::new(fn_widget! {
      @MockBox { size: CHILD_SIZE, anchor }
    })
    .with_wnd_size(WND_SIZE)
  }
  widget_layout_test!(
    pixel_left_top,
    widget_tester(Anchor::left_top(1., 1.)),
    LayoutCase::default().with_pos(Point::new(1., 1.))
  );

  widget_layout_test!(
    pixel_left_bottom,
    widget_tester(Anchor::left_bottom(1., 1.)),
    LayoutCase::default().with_pos((1., 49.).into())
  );

  widget_layout_test!(
    pixel_top_right,
    widget_tester(Anchor::right_top(1., 1.)),
    LayoutCase::default().with_pos((49., 1.).into())
  );

  widget_layout_test!(
    pixel_bottom_right,
    widget_tester(Anchor::right_bottom(1., 1.)),
    LayoutCase::default().with_pos((49., 49.).into())
  );

  widget_layout_test!(
    multi_anchor,
    WidgetTester::new(fn_widget! {
      let w = @Container {
        size: Size::new(100., 100.),
        anchor: Anchor::left(40.),
      }.into_widget();

      let w = FatObj::new(w);
      @$ w {
        anchor: Anchor::top(30.)
      }
    })
    .with_wnd_size(Size::new(500., 500.)),
    LayoutCase::new(&[0]).with_rect(ribir_geom::rect(40., 30., 100., 100.))
  );
}
