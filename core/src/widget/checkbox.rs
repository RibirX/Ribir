use crate::prelude::*;

/// Represents a control that a user can select and clear.
#[derive(Clone, Declare)]
pub struct Checkbox {
  #[declare(default)]
  pub checked: bool,
  #[declare(default)]
  pub indeterminate: bool,
  #[declare(default = IconSize::of(ctx).tiny)]
  pub size: Size,
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
  fn compose(this: StateWidget<Self>) -> Widget {
    widget! {
      track { this: this.into_stateful() }
      Icon {
        size: this.size,
        cursor: CursorIcon::Hand,
        on_tap: move |_| this.switch_check(),
        on_key_up: move |k| {
          if k.key == VirtualKeyCode::Space {
            this.switch_check()
          }
        },
        ExprWidget {
          expr: {
            let icons = SvgIcons::of(ctx);
            let checked = icons.checked.clone();
            let unchecked = icons.unchecked.clone();
            let indeterminate = icons.indeterminate.clone();
            if this.indeterminate {
              indeterminate.clone()
            } else if this.checked {
              checked.clone()
            } else {
              unchecked.clone()
            }
        }}
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
    let w = widget! { Checkbox {} };
    let (rect, _) = widget_and_its_children_box_rect(w.into_widget(), Size::new(200., 200.));
    debug_assert_eq!(rect, Rect::new(Point::new(0., 0.), Size::new(18., 18.)));
  }

  #[cfg(feature = "png")]
  #[test]
  fn checked_paint() {
    let c = widget! { Checkbox { checked: true } };
    let mut window = Window::wgpu_headless(c, DeviceSize::new(100, 100));
    window.draw_frame();

    let mut expected = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    expected.push("src/test_imgs/checkbox_checked.png");
    assert!(window.same_as_png(expected));
  }

  #[cfg(feature = "png")]
  #[test]
  fn unchecked_paint() {
    let mut window = Window::wgpu_headless(widget! { Checkbox {} }, DeviceSize::new(100, 100));
    window.draw_frame();
    let mut unchecked_expect = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    unchecked_expect.push("src/test_imgs/checkbox_uncheck.png");
    assert!(window.same_as_png(unchecked_expect));
  }

  #[cfg(feature = "png")]
  #[test]
  fn indeterminate_paint() {
    let c = widget! {
      Checkbox {
        checked: true,
        indeterminate: true,
      }
    };
    let mut window = Window::wgpu_headless(c.into_widget(), DeviceSize::new(100, 100));
    window.draw_frame();

    let mut expected = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    expected.push("src/test_imgs/checkbox_indeterminate.png");
    assert!(window.same_as_png(expected.clone()));

    let c = widget! {
      Checkbox {
        checked: false,
        indeterminate: true,
      }
    };
    let mut window = Window::wgpu_headless(c.into_widget(), DeviceSize::new(100, 100));
    window.draw_frame();

    assert!(window.same_as_png(expected));
  }
}
