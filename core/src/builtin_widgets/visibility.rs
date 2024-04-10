use crate::prelude::*;

#[derive(Default)]
pub struct Visibility {
  pub visible: bool,
}

impl Declare for Visibility {
  type Builder = FatObj<()>;
  #[inline]
  fn declarer() -> Self::Builder { FatObj::new(()) }
}

impl ComposeChild for Visibility {
  type Child = Widget;
  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> impl WidgetBuilder {
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
  }
}

#[derive(SingleChild, Query, Declare, Clone)]
struct VisibilityRender {
  display: bool,
}

impl Render for VisibilityRender {
  #[inline]
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    if self.display { ctx.assert_perform_single_child_layout(clamp) } else { ZERO_SIZE }
  }

  #[inline]
  fn paint(&self, ctx: &mut PaintingCtx) {
    if !self.display {
      ctx.painter().apply_alpha(0.);
    }
  }

  fn hit_test(&self, _: &HitTestCtx, _: Point) -> HitTest {
    HitTest { hit: false, can_hit_child: self.display }
  }
}

impl Visibility {
  #[inline]
  pub fn new(visible: bool) -> Self { Self { visible } }

  #[inline]
  fn get_visible(&self) -> bool { self.visible }
}
