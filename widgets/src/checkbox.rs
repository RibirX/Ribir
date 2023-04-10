use crate::{
  common_widget::{Leading, Trailing},
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
  #[declare(default=Palette::of(ctx).primary())]
  pub color: Color,
}

#[derive(Clone)]
pub struct CheckBoxStyle {
  /// The size of the checkbox icon.
  pub icon_size: Size,
  /// The text style of the checkbox label.
  pub label_style: CowArc<TextStyle>,
  /// The checkbox foreground
  pub label_color: Brush,
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
pub enum CheckboxTemplate {
  Before(WidgetPair<Leading, State<Label>>),
  After(WidgetPair<Trailing, State<Label>>),
}

impl ComposeDecorator for CheckBoxDecorator {
  type Host = Widget;
  #[inline]
  fn compose_decorator(_: Stateful<Self>, style: Self::Host) -> Widget { style }
}

impl Checkbox {
  fn icon(this: Stateful<Self>, size: Size) -> Widget {
    widget! {
      states { this }
      CheckBoxDecorator {
        color: this.color,
        Icon {
          size,
          widget::from(
            if this.indeterminate {
              svgs::INDETERMINATE_CHECK_BOX
            } else if this.checked {
              svgs::CHECK_BOX
            } else {
              svgs::CHECK_BOX_OUTLINE_BLANK
            }
          )
        }
      }
    }
  }

  fn label(label: &State<Label>, label_color: Brush, style: CowArc<TextStyle>) -> Widget {
    let label = label.clone();
    widget! {
      states { label: label.into_readonly() }
      Text {
        text: label.0.clone(),
        foreground: label_color,
        style,
      }
    }
  }
}

impl ComposeChild for Checkbox {
  type Child = Option<CheckboxTemplate>;

  fn compose_child(this: State<Self>, child: Self::Child) -> Widget {
    widget! {
      states { this: this.into_writable() }
      init ctx => {
        let CheckBoxStyle {
          icon_size,
          label_style,
          label_color,
        } = CheckBoxStyle::of(ctx).clone();
      }
      DynWidget {
        cursor: CursorIcon::Hand,
        on_tap: move |_| this.switch_check(),
        on_key_up: move |k| if k.key == VirtualKeyCode::Space {
          this.switch_check()
        },
        dyns: {
          let label_style = label_style.clone();
          let label_color = label_color.clone();
          child.map_or(
            Checkbox::icon(no_watch!(this.clone_stateful()), icon_size),
            |child| widget! {
              Row {
                widget::from(match &child {
                  CheckboxTemplate::Before(w) => [
                    Checkbox::label(&w.child, label_color.clone(), label_style.clone()),
                    Checkbox::icon(this.clone_stateful(), icon_size),
                  ],
                  CheckboxTemplate::After(w) => [
                    Checkbox::icon(this.clone_stateful(), icon_size),
                    Checkbox::label(&w.child, label_color.clone(), label_style.clone()),
                  ],
                })
              }
          })
        },
      }
    }
  }
}

pub fn add_to_system_theme(theme: &mut SystemTheme) {
  theme.set_custom_style(CheckBoxStyle {
    icon_size: Size::splat(24.),
    label_style: theme.typography_theme().body_large.text.clone(),
    label_color: theme.palette().on_surface().into(),
  });
}

impl CustomStyle for CheckBoxStyle {}
#[cfg(test)]
mod tests {

  use super::*;
  use ribir_core::test::{expect_layout_result_with_theme, ExpectRect, LayoutTestItem};

  #[test]
  fn layout() {
    let w = widget! { Checkbox {} };
    let mut system_theme = SystemTheme::new(FullTheme::default());
    super::add_to_system_theme(&mut system_theme);

    expect_layout_result_with_theme(
      w,
      None,
      system_theme.theme(),
      &[LayoutTestItem {
        path: &[0],
        expect: ExpectRect {
          x: Some(0.),
          y: Some(0.),
          width: Some(24.),
          height: Some(24.),
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
