use crate::{prelude::*, wrap_render::*};

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
        skip_descendants: pipe!(!$this.get_visible()),
        can_focus: pipe!($this.get_visible()),
        @VisibilityRender {
          display: pipe!($this.get_visible()),
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

  fn paint(&self, host: &dyn Render, ctx: &mut PaintingCtx) {
    if self.display {
      host.paint(ctx)
    }
  }

  fn hit_test(&self, host: &dyn Render, ctx: &mut HitTestCtx, pos: Point) -> HitTest {
    if self.display {
      host.hit_test(ctx, pos)
    } else {
      HitTest { hit: false, can_hit_child: false }
    }
  }
}

impl Visibility {
  #[inline]
  pub fn new(visible: bool) -> Self { Self { visible } }

  #[inline]
  fn get_visible(&self) -> bool { self.visible }
}
