use ribir_core::prelude::*;

/// Widget just use as a paint kit for a path and not care about its size. Use
/// `[PathWidget]!` instead of.
#[derive(Declare, Clone)]
pub struct PathPaintKit {
  pub path: Path,
  #[declare(default)]
  pub style: PathStyle,
}

macro_rules! paint_method {
  () => {
    fn paint(&self, ctx: &mut PaintingCtx) {
      let painter = ctx.painter();
      match &self.style {
        PathStyle::Fill => painter.fill_path(self.path.clone().into()),
        PathStyle::Stroke(strokes) => painter
          .set_strokes(strokes.clone())
          .stroke_path(self.path.clone().into()),
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

#[derive(Declare)]
/// A path widget which size careful and can process events only if user hit at
/// the path self, not its size cover area.
pub struct PathWidget {
  pub path: Path,
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

#[cfg(test)]
mod tests {
  use ribir_core::test_helper::*;
  use ribir_dev_helper::*;

  use super::*;

  fn circle40() -> Path { Path::circle(Point::new(20., 20.), 20.) }
  const WND_SIZE: Size = Size::new(48., 48.);
  widget_test_suit!(
    circle40_kit,
    WidgetTester::new(fn_widget! {
      @PathPaintKit {
        path: circle40(),
        foreground: Color::BLACK,
      }
    })
    .with_wnd_size(WND_SIZE)
    .with_comparison(0.000025),
    LayoutCase::default().with_size(WND_SIZE)
  );

  widget_test_suit!(
    circle40,
    WidgetTester::new(fn_widget! {
      @PathWidget {
        path: circle40(),
        foreground: Color::BLACK,
      }
    })
    .with_wnd_size(WND_SIZE)
    .with_comparison(0.000025),
    LayoutCase::default().with_size(Size::new(40., 40.))
  );
}
