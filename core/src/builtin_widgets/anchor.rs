use crate::{prelude::*, wrap_render::*};

/// Specifies a horizontal anchor position for a widget relative to a target.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HAnchor {
  /// positions the widget's left edge x pixels to the right of the target's
  /// left edge.
  Left(Measure),

  /// positions the widget's right edge x pixels to the left of the target's
  /// right edge.
  Right(Measure),
}

/// Specifies a vertical anchor position for a widget relative to a target.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VAnchor {
  /// positions the widget's top edge x pixels bellow the target's top edge.
  Top(Measure),

  /// positions the widget's bottom edge x pixels above the target's bottom
  /// edge.
  Bottom(Measure),
}

impl HAnchor {
  fn map(self, f: impl FnOnce(Measure) -> Measure) -> Self {
    match self {
      HAnchor::Left(x) => HAnchor::Left(f(x)),
      HAnchor::Right(x) => HAnchor::Right(f(x)),
    }
  }
}

impl VAnchor {
  fn map(self, f: impl FnOnce(Measure) -> Measure) -> Self {
    match self {
      VAnchor::Top(x) => VAnchor::Top(f(x)),
      VAnchor::Bottom(x) => VAnchor::Bottom(f(x)),
    }
  }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
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
      _ => *other,
    }
  }
}

impl Lerp for VAnchor {
  fn lerp(&self, other: &Self, t: f32) -> Self {
    match (self, other) {
      (VAnchor::Top(y1), VAnchor::Top(y2)) => VAnchor::Top(y1.lerp(y2, t)),
      (VAnchor::Bottom(y1), VAnchor::Bottom(y2)) => VAnchor::Bottom(y1.lerp(y2, t)),
      _ => *other,
    }
  }
}

impl Lerp for Anchor {
  fn lerp(&self, other: &Self, t: f32) -> Self {
    let x = match (self.x, other.x) {
      (Some(x1), Some(x2)) => Some(x1.lerp(&x2, t)),
      (Some(x1), None) => Some(x1.map(|x| x.lerp(&Measure::default(), t))),
      (None, Some(x1)) => Some(x1.map(|x| Measure::default().lerp(&x, t))),
      _ => None,
    };

    let y = match (self.y, other.y) {
      (Some(y1), Some(y2)) => Some(y1.lerp(&y2, t)),
      (Some(y1), None) => Some(y1.map(|y| y.lerp(&Measure::default(), t))),
      (None, Some(y1)) => Some(y1.map(|y| Measure::default().lerp(&y, t))),
      _ => None,
    };
    Self { x, y }
  }
}

impl Anchor {
  pub fn new(x: HAnchor, y: VAnchor) -> Self { Self { x: Some(x), y: Some(y) } }

  /// Return Anchor that positions the widget's left top corner to the position
  pub fn from_point(pos: Point) -> Self { pos.into() }

  /// Return Anchor that positions the widget's left edge x pixels to the right
  /// of the target's left edge.
  pub fn left(x: impl Into<Measure>) -> Self { Self { x: Some(HAnchor::Left(x.into())), y: None } }

  /// Return Anchor that positions the widget's right edge x pixels to the left
  /// of the target's right edge.
  pub fn right(x: impl Into<Measure>) -> Self {
    Self { x: Some(HAnchor::Right(x.into())), y: None }
  }

  /// Return Anchor that positions the widget's top edge x pixels bellow the
  /// target's top edge.
  pub fn top(y: impl Into<Measure>) -> Self { Self { x: None, y: Some(VAnchor::Top(y.into())) } }

  /// Return Anchor that positions the widget's bottom edge x pixels above the
  /// parent's bottom edge.
  pub fn bottom(y: impl Into<Measure>) -> Self {
    Self { x: None, y: Some(VAnchor::Bottom(y.into())) }
  }

  /// Return Anchor that positions the widget's left top corner to the position
  /// x pixel right, y pixel bellow relative to the left top corner of
  /// the target
  pub fn left_top(x: impl Into<Measure>, y: impl Into<Measure>) -> Self {
    Self::new(HAnchor::Left(x.into()), VAnchor::Top(y.into()))
  }

  /// Return Anchor that positions the widget's right top corner to the position
  /// x pixel left, y pixel bellow relative to the right top corner of
  /// the target
  pub fn right_top(x: impl Into<Measure>, y: impl Into<Measure>) -> Self {
    Self::new(HAnchor::Right(x.into()), VAnchor::Top(y.into()))
  }

  /// Return Anchor that positions the widget's left bottom corner to the
  /// position x pixel right, y pixel above relative to the left bottom corner
  /// of the target
  pub fn left_bottom(x: impl Into<Measure>, y: impl Into<Measure>) -> Self {
    Self::new(HAnchor::Left(x.into()), VAnchor::Bottom(y.into()))
  }

  /// Return Anchor that positions the widget's right bottom corner to the
  /// position x pixel left, y pixel above relative to the right bottom corner
  /// of the target
  pub fn right_bottom(x: impl Into<Measure>, y: impl Into<Measure>) -> Self {
    Self::new(HAnchor::Right(x.into()), VAnchor::Bottom(y.into()))
  }
}

/// A wrapper widget that anchors its child relative to its parent or a target.
///
/// This is a built-in field of `FatObj`. Setting the `anchor` field attaches
/// an `Anchor` to the host, allowing precise placement relative to the parent
/// bounds.
///
/// # Example, the text will be anchored to the bottom right corner of the
/// container with 10 pixels offset from the right and bottom edges.
///
/// ```rust
/// use ribir::prelude::*;
///
/// container! {
///   size: Size::new(200., 200.),
///   @Text {
///     text: "Bottom Right",
///     anchor: Anchor::right_bottom(10., 10.),
///   }
/// };
/// ```
///
/// ## Note
///
/// For percentage-based or right/bottom-relative anchors we compute offsets
/// from a container derived from the child size and local constraints, not
/// from the parent's unconstrained sizes. The container is chosen by:
///
/// 1. Use the maximum constraint when it is finite.
/// 2. Otherwise, fall back to the child's clamped size (respecting min/max).
///
/// ## Usage guidelines
///
/// For reliable placement, prefer using anchors inside parents with fixed
/// sizes or predictable layout behavior.

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

    let container =
      Size::new(clamp.container_width(child_size.width), clamp.container_height(child_size.height));

    let offset = self.anchor.into_pixel(child_size, container);
    let pos = ctx.box_pos().unwrap_or_default();
    ctx.update_position(ctx.widget_id(), pos + Size::new(offset.x, offset.y));
    child_size
  }

  #[inline]
  fn wrapper_dirty_phase(&self) -> DirtyPhase { DirtyPhase::Layout }
}

impl HAnchor {
  pub fn into_pixel(self, width: f32, parent: f32) -> f32 {
    match self {
      HAnchor::Left(x) => x.into_pixel(parent),
      HAnchor::Right(x) => parent - width - x.into_pixel(parent),
    }
  }
}

impl VAnchor {
  pub fn into_pixel(self, height: f32, parent: f32) -> f32 {
    match self {
      VAnchor::Top(y) => y.into_pixel(parent),
      VAnchor::Bottom(y) => parent - height - y.into_pixel(parent),
    }
  }
}

impl Anchor {
  pub fn into_pixel(self, size: Size, parent: Size) -> Point {
    let Self { x, y } = self;
    Point::new(
      x.map(|x| x.into_pixel(size.width, parent.width))
        .unwrap_or_default(),
      y.map(|y| y.into_pixel(size.height, parent.height))
        .unwrap_or_default(),
    )
  }
}

impl From<Point> for Anchor {
  fn from(value: Point) -> Self {
    Self::new(HAnchor::Left(value.x.into()), VAnchor::Top(value.y.into()))
  }
}

impl Default for HAnchor {
  fn default() -> Self { Self::Left(Measure::default()) }
}

impl Default for VAnchor {
  fn default() -> Self { Self::Top(Measure::default()) }
}

impl From<f32> for HAnchor {
  fn from(value: f32) -> Self { Self::Left(value.into()) }
}

impl From<f32> for VAnchor {
  fn from(value: f32) -> Self { Self::Top(value.into()) }
}

impl From<Measure> for HAnchor {
  fn from(value: Measure) -> Self { Self::Left(value) }
}

impl From<Measure> for VAnchor {
  fn from(value: Measure) -> Self { Self::Top(value) }
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

      let mut w = FatObj::new(w);
      @(w) {
        anchor: Anchor::top(30.)
      }
    })
    .with_wnd_size(Size::new(500., 500.)),
    LayoutCase::new(&[0]).with_rect(ribir_geom::rect(40., 30., 100., 100.))
  );
}
