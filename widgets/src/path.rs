use ribir_core::{impl_query_self_only, prelude::*};

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

  #[inline]
  fn hit_test(&self, _: &TreeCtx, _: Point) -> HitTest {
    HitTest { hit: false, can_hit_child: false }
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

// fixme: hit test directly used path box rect, no path hit test do.
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
}

impl Query for PathsPaintKit {
  impl_query_self_only!();
}
