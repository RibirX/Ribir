use ribir_core::prelude::*;

/// Widget just use as a paint kit for a path and not care about its size. Use
/// `[PathWidget]!` instead of.
#[derive(Declare, Query, Clone)]
pub struct PathPaintKit {
  pub path: Path,
  pub brush: Brush,
  #[declare(default)]
  pub style: PathStyle,
}

macro_rules! paint_method {
  () => {
    fn paint(&self, ctx: &mut PaintingCtx) {
      let painter = ctx.painter();
      painter.set_brush(self.brush.clone());
      match &self.style {
        PathStyle::Fill => painter.fill_path(self.path.clone()),
        PathStyle::Stroke(strokes) => painter
          .set_strokes(strokes.clone())
          .stroke_path(self.path.clone()),
      };
    }
  };
}

impl Render for PathPaintKit {
  #[inline]
  fn perform_layout(&self, clamp: BoxClamp, _: &mut LayoutCtx) -> Size { clamp.max }

  #[inline]
  fn only_sized_by_parent(&self) -> bool { true }

  paint_method!();

  fn hit_test(&self, _ctx: &HitTestCtx, _: Point) -> HitTest {
    HitTest { hit: false, can_hit_child: false }
  }
}

#[derive(Declare, Query)]
/// A path widget which size careful and can process events only if user hit at
/// the path self, not its size cover area.
pub struct PathWidget {
  pub path: Path,
  pub brush: Brush,
  #[declare(default)]
  pub style: PathStyle,
}

/// Path widget just use as a paint kit for a path and not care about its size.
/// Use `[HitTesPath]!` instead of.
impl Render for PathWidget {
  #[inline]
  fn perform_layout(&self, _: BoxClamp, _: &mut LayoutCtx) -> Size {
    self.path.bounds().max().to_vector().to_size()
  }

  paint_method!();
}
