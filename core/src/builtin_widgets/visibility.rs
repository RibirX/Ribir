use crate::{prelude::*, wrap_render::*};

/// A wrapper that controls whether its child is visible and participates in
/// layout, painting, and hit testing.
///
/// This is a built-in `FatObj` field. Setting the `visible` field attaches a
/// `Visibility` wrapper which can hide the child from layout, painting, and
/// hit testing when disabled.
///
/// # Example
///
/// Hide a container by setting `visible` to `false`.
///
/// ```rust no_run
/// use ribir::prelude::*;
///
/// container! {
///   size: Size::new(100., 100.),
///   background: Color::RED,
///   visible: false,
/// };
/// ```
#[derive(Default)]
pub struct Visibility {
  pub visible: bool,
}

impl Declare for Visibility {
  type Builder = FatObj<()>;
  #[inline]
  fn declarer() -> Self::Builder { FatObj::new(()) }
}

impl<'c> ComposeChild<'c> for Visibility {
  type Child = Widget<'c>;
  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
    fn_widget! {
      @FocusScope {
        skip_descendants: pipe!(!$read(this).get_visible()),
        skip_host: pipe!(!$read(this).get_visible()),
        @VisibilityRender {
          display: pipe!($read(this).get_visible()),
          @ { child }
        }
      }
    }
    .into_widget()
  }
}

#[derive(Declare, Clone)]
struct VisibilityRender {
  display: bool,
}

impl_compose_child_for_wrap_render!(VisibilityRender);

impl WrapRender for VisibilityRender {
  #[inline]
  fn perform_layout(&self, clamp: BoxClamp, host: &dyn Render, ctx: &mut LayoutCtx) -> Size {
    if self.display { host.perform_layout(clamp, ctx) } else { clamp.min }
  }

  fn visual_box(&self, host: &dyn Render, ctx: &mut VisualCtx) -> Option<Rect> {
    if self.display {
      host.visual_box(ctx)
    } else {
      ctx.clip(Rect::from_size(Size::zero()));
      None
    }
  }

  fn paint(&self, host: &dyn Render, ctx: &mut PaintingCtx) {
    if self.display {
      host.paint(ctx)
    } else {
      ctx.painter().apply_alpha(0.);
    }
  }

  fn hit_test(&self, host: &dyn Render, ctx: &mut HitTestCtx, pos: Point) -> HitTest {
    if self.display {
      host.hit_test(ctx, pos)
    } else {
      HitTest { hit: false, can_hit_child: false }
    }
  }

  fn wrapper_dirty_phase(&self) -> DirtyPhase { DirtyPhase::Layout }
}

impl Visibility {
  #[inline]
  pub fn new(visible: bool) -> Self { Self { visible } }

  #[inline]
  fn get_visible(&self) -> bool { self.visible }
}

#[cfg(test)]
mod tests {
  use test_helper::split_value;

  use super::*;
  use crate::test_helper::*;

  #[test]
  fn visible_children_not_paint() {
    reset_test_env!();

    struct PainterHit(Stateful<i32>);

    impl Render for PainterHit {
      fn perform_layout(&self, clamp: BoxClamp, _ctx: &mut LayoutCtx) -> Size { clamp.max }

      fn paint(&self, _ctx: &mut PaintingCtx) { *self.0.write() += 1; }
    }

    let hit = Stateful::new(0);
    let (visible, w_visible) = split_value(true);
    let hit2 = hit.clone_writer();
    let wnd = TestWindow::from_widget(container! {
      size: Size::splat(100.),
      visible: pipe!(*$read(visible)),
      @PainterHit(hit2.clone_writer())
    });

    wnd.draw_frame();
    assert_eq!(*hit.read(), 1);
    *w_visible.write() = false;
    wnd.draw_frame();
    assert_eq!(*hit.read(), 1);
  }
}
