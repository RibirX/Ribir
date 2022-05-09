use crate::prelude::*;
use crate::widget::theme::CheckboxTheme;

/// Represents a control that a user can select and clear.
#[derive(Default, Clone, Declare)]
pub struct Checkbox {
  #[declare(default)]
  pub checked: bool,
  #[declare(default)]
  pub indeterminate: bool,
  #[declare(default = "ctx.theme().checkbox.clone()")]
  pub style: CheckboxTheme,
}

impl Checkbox {
  pub fn switch_check(&mut self) {
    if self.indeterminate {
      self.indeterminate = false;
      self.checked = false;
    } else {
      self.checked = !self.checked;
    }
  }
}

impl Compose for Stateful<Checkbox> {
  fn compose(self, _: &mut BuildCtx) -> BoxedWidget {
    let CheckboxTheme { size, .. } = self.style.clone();
    let has_checked = self.indeterminate || self.checked;
    let mut state_ref = unsafe { self.state_ref() };

    // todo: track self
    widget! {
      declare Empty {
        margin: EdgeInsets::all(4.),
        cursor: CursorIcon::Hand,
        on_tap: {
          move |_| state_ref.switch_check()
        },
        on_key_up: {
          move |k| {
            if k.key == VirtualKeyCode::Space {
              state_ref.switch_check()
            }
          }
        },
        ExprChild {
          let size = Size::new(size, size);
          if has_checked {
            if self.indeterminate {
              Icon {
                src: "./core/src/widget/theme/checkbox/indeterminate.svg",
                size
              }
            } else {
              Icon {
                src: "./core/src/widget/theme/checkbox/checked.svg",
                size
              }
            }
          } else {
            Icon {
              src: "./core/src/widget/theme/checkbox/unchecked.svg",
              size
            }
          }
        }
      }
    }
  }
}

impl BoxWidget<ComposeMarker> for Checkbox {
  fn box_it(self) -> BoxedWidget { Stateful::new(self).box_it() }
}
#[cfg(test)]
mod tests {
  use super::*;
  use crate::test::widget_and_its_children_box_rect;

  #[test]
  fn layout() {
    let w = Checkbox::default();
    let (rect, child) = widget_and_its_children_box_rect(w.box_it(), Size::new(200., 200.));
    debug_assert_eq!(rect, Rect::new(Point::new(0., 0.), Size::new(24., 24.)));

    debug_assert_eq!(
      child,
      vec![Rect::new(Point::new(4., 4.), Size::new(16., 16.))]
    );
  }

  #[cfg(feature = "png")]
  #[test]
  fn checked_paint() {
    let c = Checkbox { checked: true, ..<_>::default() };
    let mut window = Window::wgpu_headless(c.box_it(), DeviceSize::new(100, 100));
    window.render_ready();

    assert!(window.same_as_png("../test/test_imgs/checkbox_checked.png"));
  }

  #[cfg(feature = "png")]
  #[test]
  fn unchecked_paint() {
    let mut window = Window::wgpu_headless(Checkbox::default().box_it(), DeviceSize::new(100, 100));
    window.render_ready();
    assert!(window.same_as_png("../test/test_imgs/checkbox_uncheck.png"));
  }

  #[cfg(feature = "png")]
  #[test]
  fn indeterminate_paint() {
    let c = Checkbox {
      checked: true,
      indeterminate: true,
      ..<_>::default()
    };
    let mut window = Window::wgpu_headless(c.box_it(), DeviceSize::new(100, 100));
    window.render_ready();

    assert!(window.same_as_png("../test/test_imgs/checkbox_indeterminate.png"));

    let c = Checkbox {
      checked: false,
      indeterminate: true,
      ..<_>::default()
    };
    let mut window = Window::wgpu_headless(c.box_it(), DeviceSize::new(100, 100));
    window.render_ready();

    assert!(window.same_as_png("../test/test_imgs/checkbox_indeterminate.png"));
  }
}
