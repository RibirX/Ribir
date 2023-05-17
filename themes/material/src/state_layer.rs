use ribir_core::prelude::*;
use ribir_widgets::prelude::{Container, StackFit};
use ribir_widgets::{layout::Stack, path::PathPaintKit};

/// Widget that as an visual indicator of material design used to present the
/// interactive status of its child.
#[derive(Declare)]
pub struct StateLayer {
  pub color: Color,
  pub path: Path,
  pub role: StateRole,
}
/// Widget that as visual indicator of material design used to communicate the
/// status of interactive widget, its visual state will reactive to its child
/// interactive state.
#[derive(Declare)]
pub struct InteractiveLayer {
  /// the color of the state layer, will apply a fixed opacity in different
  /// state.
  pub color: Color,
  /// The border radii
  pub border_radii: Radius,
}

impl Compose for StateLayer {
  fn compose(this: State<Self>) -> Widget {
    widget!(
      states { this: this.into_readonly() }
      PathPaintKit {
        path: this.path.clone(),
        brush: this.role.calc_color(this.color),
      }
    )
  }
}

impl ComposeChild for InteractiveLayer {
  type Child = Widget;

  fn compose_child(this: State<Self>, child: Self::Child) -> Widget {
    widget! {
      states { this: this.into_readonly() }
      Stack {
        fit: StackFit::Passthrough,
        DynWidget { id: host, dyns: child }
        IgnorePointer {
          Container {
            size: host.layout_size(),
            StateLayer {
              color: this.color,
              path: Path::rect_round(&host.layout_rect(), &this.border_radii),
              role: if host.pointer_pressed() {
                StateRole::pressed()
              } else if host.has_focus() {
                StateRole::focus()
              } else if host.mouse_hover() {
                StateRole::hover()
              } else {
                // todo: not support drag & drop now
                StateRole::custom(0.)
              }
            }
          }
        }
      }
    }
  }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct StateRole(f32);

impl StateRole {
  pub const fn hover() -> Self { Self(0.08) }

  pub const fn focus() -> Self { Self(0.12) }

  pub const fn pressed() -> Self { Self(0.12) }

  pub const fn dragged() -> Self { Self(0.16) }

  pub const fn custom(opacity: f32) -> Self { Self(opacity) }

  #[inline]
  pub fn calc_color(self, color: Color) -> Color { color.with_alpha(self.0) }
}
