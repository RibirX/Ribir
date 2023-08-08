use ribir_core::{impl_query_self_only, prelude::*};

/// Widget just use as a paint kit for a path and not care about its size. Use
/// `[PathWidget]!` instead of.
#[derive(Declare, Declare2, Clone)]
pub struct PathPaintKit {
  pub path: Path,
  #[declare(convert=into)]
  pub brush: Brush,
  #[declare(default)]
  pub style: PathPaintStyle,
}

macro_rules! paint_method {
  () => {
    fn paint(&self, ctx: &mut PaintingCtx) {
      let painter = ctx.painter();
      painter.set_brush(self.brush.clone());
      match &self.style {
        PathPaintStyle::Fill => painter.fill_path(self.path.clone()),
        PathPaintStyle::Stroke(strokes) => painter
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

impl_query_self_only!(PathPaintKit);

#[derive(Declare)]
/// A path widget which size careful and can process events only if user hit at
/// the path self, not its size cover area.
pub struct PathWidget {
  pub path: Path,
  #[declare(convert=into)]
  pub brush: Brush,
  #[declare(default)]
  pub style: PathPaintStyle,
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

impl_query_self_only!(PathWidget);
