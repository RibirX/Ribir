use ribir_core::prelude::*;

/// The widget serves as a painting kit for a path and does not concern itself
/// with size. Otherwise, it should use `Resource<Path>` instead.
#[derive(Declare, Clone)]
pub struct PathPaintKit {
  pub path: Resource<Path>,
}

impl Render for PathPaintKit {
  #[inline]
  fn perform_layout(&self, _: BoxClamp, _: &mut LayoutCtx) -> Size { Size::zero() }

  fn visual_box(&self, _: &mut VisualCtx) -> Option<Rect> { Some(self.path.bounds(None)) }

  #[inline]
  fn size_affected_by_child(&self) -> bool { false }

  fn paint(&self, ctx: &mut PaintingCtx) { self.path.paint(ctx); }

  fn hit_test(&self, _ctx: &mut HitTestCtx, _: Point) -> HitTest {
    HitTest { hit: false, can_hit_child: false }
  }
}

#[cfg(test)]
mod tests {
  use ribir_core::test_helper::*;
  use ribir_dev_helper::*;

  use super::*;

  fn circle40() -> Resource<Path> { Path::circle(Point::new(20., 20.), 20.).into() }
  const WND_SIZE: Size = Size::new(48., 48.);
  const SIZE_40: Size = Size::new(40., 40.);

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
    LayoutCase::default().with_size(Size::zero())
  );

  widget_test_suit!(
    circle40,
    WidgetTester::new(fat_obj! {
      foreground: Color::BLACK,
      @circle40()
    })
    .with_wnd_size(WND_SIZE)
    .with_comparison(0.000025),
    LayoutCase::default().with_size(SIZE_40)
  );

  widget_test_suit!(
    fill_circle40,
    WidgetTester::new(fn_widget! {
      let mut path = FatObj::new(circle40());
      @(path) {
        painting_style: PaintingStyle::Fill,
        foreground: Color::BLACK,
      }
    })
    .with_wnd_size(WND_SIZE)
    .with_comparison(0.000025),
    LayoutCase::default().with_size(SIZE_40)
  );

  widget_test_suit!(
    stroke_circle40,
    WidgetTester::new(fat_obj! {
      painting_style: PaintingStyle::Stroke(StrokeOptions::default()),
      foreground: Color::BLACK,
      @circle40()
    })
    .with_wnd_size(WND_SIZE)
    .with_comparison(0.00003),
    LayoutCase::default().with_size(Size::splat(40.5))
  );
}
