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

  #[inline]
  fn only_sized_by_parent(&self) -> bool { true }

  fn paint(&self, ctx: &mut PaintingCtx) {
    let path = PaintPath::Share(self.path.clone());
    ctx.painter().draw_path(path);
  }

  fn hit_test(&self, _ctx: &HitTestCtx, _: Point) -> HitTest {
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
    WidgetTester::new(fn_widget! {
      FatObj::new(circle40()).foreground(Color::BLACK)
    })
    .with_wnd_size(WND_SIZE)
    .with_comparison(0.000025),
    LayoutCase::default().with_size(SIZE_40)
  );

  widget_test_suit!(
    fill_circle40,
    WidgetTester::new(fn_widget! {
      let path = FatObj::new( circle40());
      @ $path {
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
    WidgetTester::new(fn_widget! {
      FatObj::new(circle40())
        .painting_style(PaintingStyle::Stroke(StrokeOptions::default()))
        .foreground(Color::BLACK)
    })
    .with_wnd_size(WND_SIZE)
    .with_comparison(0.000025),
    LayoutCase::default().with_size(Size::splat(40.5))
  );
}
