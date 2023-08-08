use crate::{
  common_widget::{Leading, Trailing},
  prelude::{Icon, Label, Row, Text},
};
use ribir_core::prelude::*;

/// Represents a control that a user can select and clear.
#[derive(Clone, Declare, Declare2)]
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
  Before(SinglePair<Leading, State<Label>>),
  After(SinglePair<Trailing, State<Label>>),
}

impl ComposeDecorator for CheckBoxDecorator {
  type Host = Widget;

  fn compose_decorator(_: State<Self>, host: Self::Host) -> Widget { host }
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
    .into()
  }

  fn label(label: Stateful<Label>, label_color: Brush, text_style: CowArc<TextStyle>) -> Widget {
    widget! {
      states { label }
      Text {
        text: label.0.clone(),
        foreground: label_color,
        text_style,
      }
    }
    .into()
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
            |mut child| widget! {
              Row {
                Multi::new(match &mut child {
                  CheckboxTemplate::Before(w) => [
                    Checkbox::label(w.child.clone_state(), label_color.clone(), label_style.clone()),
                    Checkbox::icon(this.clone_stateful(), icon_size),
                  ],
                  CheckboxTemplate::After(w) => [
                    Checkbox::icon(this.clone_stateful(), icon_size),
                    Checkbox::label(w.child.clone_state(), label_color.clone(), label_style.clone()),
                  ],
                })
              }
          }.into())
        },
      }
    }
    .into()
  }
}

// A Checkbox can be a widget even if it has no children
impl Compose for Checkbox {
  #[inline]
  fn compose(this: State<Self>) -> Widget { ComposeChild::compose_child(this, None) }
}

impl CustomStyle for CheckBoxStyle {
  fn default_style(ctx: &BuildCtx) -> Self {
    CheckBoxStyle {
      icon_size: Size::splat(24.),
      label_style: TypographyTheme::of(ctx).body_large.text.clone(),
      label_color: Palette::of(ctx).on_surface().into(),
    }
  }
}
#[cfg(test)]
mod tests {
  use super::*;
  use ribir_core::test_helper::*;
  use ribir_dev_helper::*;
  extern crate test;
  use test::Bencher;

  fn checked() -> Widget { widget! { Checkbox { checked: true } }.into() }
  widget_test_suit!(
    checked,
    wnd_size = Size::new(48., 48.),
    width == 24.,
    height == 24.,
  );

  fn unchecked() -> Widget { widget! { Checkbox {  } }.into() }
  widget_test_suit!(
    unchecked,
    wnd_size = Size::new(48., 48.),
    width == 24.,
    height == 24.,
  );

  fn indeterminate() -> Widget {
    widget! {
      Checkbox {
        checked: true,
        indeterminate: true,
      }
    }
    .into()
  }

  widget_test_suit!(
    indeterminate,
    wnd_size = Size::new(48., 48.),
    width == 24.,
    height == 24.,
  );
}
