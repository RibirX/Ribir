use lyon_algorithms::{hit_test::hit_test_path, math::point};
use lyon_path::FillRule;
use ribir_core::{impl_query_self_only, prelude::*};

const TOLERANCE: f32 = 0.1;

/// Widget just use as a paint kit for a path and not care about its size.
/// Use `[PathWidget]!` instead of.
#[derive(Declare)]
pub struct PathPaintKit {
  pub path: Path,
  #[declare(convert=into)]
  pub brush: Brush,
}

impl Render for PathPaintKit {
  #[inline]
  fn perform_layout(&self, _: BoxClamp, _: &mut LayoutCtx) -> Size { Size::zero() }

  #[inline]
  fn only_sized_by_parent(&self) -> bool { true }

  #[inline]
  fn paint(&self, ctx: &mut PaintingCtx) {
    ctx
      .painter()
      .set_brush(self.brush.clone())
      .paint_path(self.path.clone());
  }

  #[inline]
  fn can_overflow(&self) -> bool { true }

  fn hit_test(&self, _ctx: &TreeCtx, pos: Point) -> HitTest {
    let pt = point(pos.x, pos.y);
    // todo: support fillrule
    let is_hit = hit_test_path(
      &pt,
      self.path.path.into_iter(),
      FillRule::EvenOdd,
      TOLERANCE,
    );
    HitTest { hit: is_hit, can_hit_child: is_hit }
  }
}

impl Query for PathPaintKit {
  impl_query_self_only!();
}

#[derive(Declare)]
/// A path widget which size careful and can process events only if user hit at
/// the path self, not its size cover area.
pub struct PathWidget {
  pub path: Path,
  #[declare(convert=into)]
  pub brush: Brush,
}

/// Path widget just use as a paint kit for a path and not care about its size.
/// Use `[HitTesPath]!` instead of.
impl Render for PathWidget {
  #[inline]
  fn perform_layout(&self, _: BoxClamp, _: &mut LayoutCtx) -> Size { self.path.box_rect().size }

  #[inline]
  fn paint(&self, ctx: &mut PaintingCtx) {
    ctx
      .painter()
      .set_brush(self.brush.clone())
      .paint_path(self.path.clone());
  }
}

impl Query for PathWidget {
  impl_query_self_only!();
}

#[derive(Declare)]
/// Widget use to help directly paint dozens of paths, and not care about its
/// size.
pub struct PathsPaintKit {
  pub paths: Vec<PathPaintKit>,
}

impl Render for PathsPaintKit {
  #[inline]
  fn perform_layout(&self, _: BoxClamp, _: &mut LayoutCtx) -> Size { Size::zero() }

  #[inline]
  fn only_sized_by_parent(&self) -> bool { true }

  #[inline]
  fn paint(&self, ctx: &mut PaintingCtx) { self.paths.iter().for_each(|p| p.paint(ctx)); }

  fn hit_test(&self, _ctx: &TreeCtx, pos: Point) -> HitTest {
    let pt = point(pos.x, pos.y);
    let is_hit = self.paths.iter().any(|path| {
      hit_test_path(
        &pt,
        path.path.path.into_iter(),
        FillRule::EvenOdd,
        TOLERANCE,
      )
    });

    HitTest { hit: is_hit, can_hit_child: false }
  }
}

impl Query for PathsPaintKit {
  impl_query_self_only!();
}
