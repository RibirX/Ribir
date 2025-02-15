use crate::{prelude::*, wrap_render::*};

/// Widget use to ignore pointer events
#[derive(Declare, Clone)]
pub struct IgnorePointer {
  #[declare(default)]
  pub ignore: IgnoreScope,
}

/// Specify the scope for ignoring events.
#[derive(Debug, Clone, Copy, Default)]
pub enum IgnoreScope {
  /// Not ignore
  None,
  /// Ignore only the event of the current widget.
  OnlySelf,
  /// Ignored the event within the subtree, including the current widget.
  #[default]
  Subtree,
}

impl_compose_child_for_wrap_render!(IgnorePointer, DirtyPhase::Paint);

impl WrapRender for IgnorePointer {
  fn hit_test(&self, host: &dyn Render, ctx: &mut HitTestCtx, pos: Point) -> HitTest {
    match self.ignore {
      IgnoreScope::Subtree => HitTest { hit: false, can_hit_child: false },
      IgnoreScope::OnlySelf => {
        let hit = host.hit_test(ctx, pos);
        HitTest { hit: false, can_hit_child: hit.can_hit_child }
      }
      IgnoreScope::None => host.hit_test(ctx, pos),
    }
  }
}
