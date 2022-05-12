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

impl Compose for Checkbox {
  fn compose(this: Stateful<Self>, _: &mut BuildCtx) -> BoxedWidget {
    widget! {
      track { this }
      declare Empty {
        margin: EdgeInsets::all(4.),
        cursor: CursorIcon::Hand,
        on_tap: move |_| this.switch_check(),
        on_key_up: move |k| {
          if k.key == VirtualKeyCode::Space {
            this.switch_check()
          }
        },
        ExprWidget {
          let size = this.style.size;
          let size = Size::new(size, size);
            if this.indeterminate {
              Icon {
                src: "./core/src/widget/theme/checkbox/indeterminate.svg",
                size
              }
            } else if this.checked {
              Icon {
                src: "./core/src/widget/theme/checkbox/checked.svg",
                size
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
