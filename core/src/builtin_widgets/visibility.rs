use crate::{impl_query_self_only, prelude::*};

#[derive(Declare, Declare2)]
pub struct Visibility {
  #[declare(builtin)]
  pub visible: bool,
}

impl ComposeChild for Visibility {
  type Child = Widget;
  fn compose_child(this: State<Self>, child: Self::Child) -> Widget {
    widget! {
      states { this: this.into_readonly(), }
      FocusScope {
        skip_descendants: !this.visible,
        can_focus: this.visible,
        VisibilityRender {
          display: this.visible,
          widget::from(child)
        }
      }
    }
    .into()
  }
}

#[derive(SingleChild, Declare, Clone)]
struct VisibilityRender {
  display: bool,
}

impl Render for VisibilityRender {
  #[inline]
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    if self.display {
      ctx.assert_perform_single_child_layout(clamp)
    } else {
      ZERO_SIZE
    }
  }

  #[inline]
  fn paint(&self, ctx: &mut PaintingCtx) {
    if !self.display {
      ctx.painter().apply_alpha(0.);
    }
  }

  fn hit_test(&self, _: &HitTestCtx, _: Point) -> HitTest {
    HitTest {
      hit: false,
      can_hit_child: self.display,
    }
  }
}

impl_query_self_only!(VisibilityRender);

impl Visibility {
  #[inline]
  pub fn new(visible: bool) -> Self { Self { visible } }
}
