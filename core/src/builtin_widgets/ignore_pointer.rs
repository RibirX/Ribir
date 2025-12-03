use crate::{prelude::*, wrap_render::*};

/// A wrapper that ignores pointer events according to a specified scope.
///
/// `IgnorePointer` can suppress pointer events for the host, only the host,
/// or the whole subtree depending on `IgnoreScope`.
///
/// # Example
///
/// The red container below will not respond to pointer events when the scope
/// is set to `IgnoreScope::Subtree`.
///
/// ```rust
/// use ribir::prelude::*;
///
/// fn_widget! {
///   @IgnorePointer {
///     ignore: IgnoreScope::Subtree,
///     @Container {
///       size: Size::new(100., 100.),
///       background: Color::RED,
///       on_tap: |_: &mut PointerEvent| println!("This will never be printed"),
///     }
///   }
/// };
/// ```
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

impl_compose_child_for_wrap_render!(IgnorePointer);

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

  #[inline]
  fn wrapper_dirty_phase(&self) -> DirtyPhase { DirtyPhase::Paint }
}
