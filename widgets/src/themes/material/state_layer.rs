use crate::path::PathPaintKit;
use ribir_core::prelude::*;

#[derive(Declare)]
pub struct StateLayer {
  pub color: Color,
  pub path: Path,
  pub role: StateRole,
}

impl Compose for StateLayer {
  fn compose(this: StateWidget<Self>) -> Widget {
    widget_try_track!(
      try_track { this }
      PathPaintKit {
        path: this.path.clone(),
        brush: {
          let color = this.color;
          let alpha = this.role.value();
          color.apply_alpha(alpha)
        }
      }
    )
  }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct StateRole(f32);

impl StateRole {
  pub const fn hover() -> Self { Self(0.08) }

  pub const fn focus() -> Self { Self(0.12) }

  pub const fn pressed() -> Self { Self(0.12) }

  pub const fn dragged() -> Self { Self(0.16) }

  #[inline]
  pub const fn custom(opacity: f32) -> Self { Self(opacity) }

  #[inline]
  pub const fn value(self) -> f32 { self.0 }
}
