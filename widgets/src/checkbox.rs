use crate::{
  layout::Position,
  prelude::{svgs, Icon, Label, Row, Text},
};
use ribir_core::prelude::*;

/// Represents a control that a user can select and clear.
#[derive(Clone, Declare)]
pub struct Checkbox {
  #[declare(default)]
  pub checked: bool,
  #[declare(default)]
  pub indeterminate: bool,
  #[declare(default=Palette::of(ctx).primary(), convert=into)]
  pub foreground: Brush,
}

#[derive(Clone)]
pub struct CheckBoxStyle {
  /// The size of the checkbox icon.
  pub icon_size: Size,
  /// The text style of the checkbox label.
  pub label_style: CowArc<TextStyle>,
  /// The checkbox foreground
  pub label_foreground: Brush,
  /// The checkbox Label in the position of Icon
  pub position: Position,
}

#[derive(Clone, Declare)]
pub struct CheckBoxDecorator {
  #[declare(default=Palette::of(ctx).primary())]
  pub color: Color,
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

#[derive(Template)]
pub struct CheckboxTemplate {
  pub label: State<Label>,
}

impl ComposeStyle for CheckBoxDecorator {
  type Host = Widget;
  #[inline]
  fn compose_style(_: Stateful<Self>, style: Self::Host) -> Widget { style }
}

impl ComposeChild for Checkbox {
  type Child = Option<CheckboxTemplate>;

  fn compose_child(this: State<Self>, child: Self::Child) -> Widget {
    widget! {
      states { this: this.into_writable() }
      init ctx => {
        let CheckBoxStyle {
          icon_size: size,
          label_style,
          position,
          label_foreground,
        } = CheckBoxStyle::of(ctx).clone();
      }
      DynWidget {
        cursor: CursorIcon::Hand,
        on_tap: move |_| this.switch_check(),
        on_key_up: move |k| if k.key == VirtualKeyCode::Space {
          this.switch_check()
        },
        dyns: {
          let label = child.map(|child| widget! {
            states { label: child.label.into_readonly() }
            Text {
              text: label.0.clone(),
              foreground: label_foreground,
              style: label_style.clone(),
            }
          });
          let checkbox = widget! {
            CheckBoxDecorator {
              Icon {
                size,
                DynWidget {
                  dyns: if this.indeterminate {
                    svgs::INDETERMINATE_CHECK_BOX
                  } else if this.checked {
                    svgs::CHECK_BOX
                  } else {
                    svgs::CHECK_BOX_OUTLINE_BLANK
                  }
                }
              }
            }
          };
          match position {
            Position::Left => widget! {
              Row {
                DynWidget::from(label)
                DynWidget::from(checkbox)
              }
            },
            Position::Right => widget! {
              Row {
                DynWidget::from(checkbox)
                DynWidget::from(label)
              }
            },
            _ => unreachable!("don't have vertical checkbox"),
          }
        }
      }
    }
  }
}

impl CustomTheme for CheckBoxStyle {}
#[cfg(test)]
mod tests {
  use crate::prelude::material;

  use super::*;
  use ribir_core::test::{expect_layout_result_with_theme, ExpectRect, LayoutTestItem};

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
          width: Some(42.),
          height: Some(42.),
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
