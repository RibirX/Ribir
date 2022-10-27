use crate::prelude::{icons, Icon, Label, Position, Row, Text};
use ribir_core::prelude::*;

/// Represents a control that a user can select and clear.
#[derive(Clone, Declare)]
pub struct Checkbox {
  #[declare(default)]
  pub checked: bool,
  #[declare(default)]
  pub indeterminate: bool,
}

#[derive(Clone)]
pub struct CheckBoxTheme {
  /// The size of the checkbox icon.
  pub size: Size,
  /// The text style of the checkbox label.
  pub label_style: TextStyle,
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

impl ComposeChild for Checkbox {
  type Child = Option<Label>;

  fn compose_child(this: StateWidget<Self>, label: Self::Child) -> Widget {
    let this = this.into_stateful();
    let mut checkbox = widget! {
      track { this: this.clone() }
      env { let theme = CheckBoxTheme::of(ctx.theme()); }
      Icon {
        size: theme.size,
        ExprWidget {
          expr: {
            let theme = ctx.theme();
            if this.indeterminate {
              icons::INDETERMINATE_CHECK_BOX.of_or_miss(theme)
            } else if this.checked {
              icons::CHECK_BOX.of_or_miss(theme)
            } else {
              icons::CHECK_BOX_OUTLINE_BLANK.of_or_miss(theme)
            }
        }}
      }
    };
    if let Some(Label { desc, position }) = label {
      let label = widget! {
        env { let theme = CheckBoxTheme::of(ctx.theme()); }
        Text { text: desc, style: theme.label_style.clone() }
      };
      checkbox = match position {
        Position::Before => {
          widget! { Row { ExprWidget { expr: [label, checkbox] } } }
        }
        Position::After => {
          widget! { Row { ExprWidget { expr: [checkbox, label] } } }
        }
      };
    }

    widget! {
      track { this }
      ExprWidget {
        cursor: CursorIcon::Hand,
        tap: move |_| this.switch_check(),
        key_up: move |k| {
          if k.key == VirtualKeyCode::Space {
            this.switch_check()
          }
        },
        expr: checkbox
      }
    }
  }
}

impl CustomTheme for CheckBoxTheme {}
#[cfg(test)]
mod tests {
  use crate::prelude::material;

  use super::*;
  use ribir_core::test::{
    expect_layout_result_with_theme, widget_and_its_children_box_rect, ExpectRect, LayoutTestItem,
  };

  #[test]
  fn layout() {
    let w = widget! { Checkbox {} };
    expect_layout_result_with_theme(
      w,
      None,
      material::purple::light(),
      &[LayoutTestItem {
        path: &[0],
        expect: ExpectRect {
          x: Some(0.),
          y: Some(0.),
          width: Some(18.),
          height: Some(18.),
        },
      }],
    );
  }

  #[cfg(feature = "png")]
  #[test]
  fn checked_paint() {
    use std::rc::Rc;

    let c = widget! { Checkbox { checked: true } };
    let theme = Rc::new(material::purple::light());
    let mut window = Window::wgpu_headless(c, theme, DeviceSize::new(100, 100));
    window.draw_frame();

    let mut expected = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    expected.push("src/test_imgs/checkbox_checked.png");
    assert!(window.same_as_png(expected));
  }

  #[cfg(feature = "png")]
  #[test]
  fn unchecked_paint() {
    use std::rc::Rc;

    let theme = Rc::new(material::purple::light());
    let mut window =
      Window::wgpu_headless(widget! { Checkbox {} }, theme, DeviceSize::new(100, 100));
    window.draw_frame();
    let mut unchecked_expect = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    unchecked_expect.push("src/test_imgs/checkbox_uncheck.png");
    assert!(window.same_as_png(unchecked_expect));
  }

  #[cfg(feature = "png")]
  #[test]
  fn indeterminate_paint() {
    use std::rc::Rc;

    let c = widget! {
      Checkbox {
        checked: true,
        indeterminate: true,
      }
    };
    let theme = Rc::new(material::purple::light());
    let mut window = Window::wgpu_headless(c.into_widget(), theme, DeviceSize::new(100, 100));
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
    let theme = Rc::new(material::purple::light());
    let mut window = Window::wgpu_headless(c.into_widget(), theme, DeviceSize::new(100, 100));
    window.draw_frame();

    assert!(window.same_as_png(expected));
  }
}
