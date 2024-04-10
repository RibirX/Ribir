use ribir_core::prelude::*;
use ribir_widgets::{layout::Stack, path::PathPaintKit, prelude::StackFit};

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
  fn compose(this: impl StateWriter<Value = Self>) -> impl WidgetBuilder {
    fn_widget! {
      @PathPaintKit {
        path: pipe!($this.path.clone()),
        brush: pipe!($this.role.calc_color($this.color)),
      }
    }
  }
}

impl ComposeChild for InteractiveLayer {
  type Child = Widget;

  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> impl WidgetBuilder {
    fn_widget! {
      let mut host = @$child { };
      let layer = @IgnorePointer {
        @Container {
          size: pipe!($host.layout_size()),
          @StateLayer {
            color: pipe!($this.color),
            path: pipe!(Path::rect_round(&$host.layout_rect(), &$this.border_radii)),
            role: pipe!(if $host.pointer_pressed() {
              StateRole::pressed()
            } else if $host.has_focus() {
              StateRole::focus()
            } else if $host.mouse_hover() {
              StateRole::hover()
            } else {
              // todo: not support drag & drop now
              StateRole::custom(0.)
            })
          }
        }
      };

      @Stack {
        fit: StackFit::Passthrough,
        @{ host }
        @{ layer }
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
