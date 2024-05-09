use ribir_core::prelude::*;

use crate::{
  common_widget::{Leading, Trailing},
  prelude::{Icon, Label, Row, Text},
};

/// Represents a control that a user can select and clear.
#[derive(Clone, Declare)]
pub struct Checkbox {
  #[declare(default)]
  pub checked: bool,
  #[declare(default)]
  pub indeterminate: bool,
  #[declare(default=Palette::of(ctx!()).primary())]
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
  #[declare(default=Palette::of(ctx!()).primary())]
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
  Before(Pair<FatObj<Leading>, State<Label>>),
  After(Pair<FatObj<Trailing>, State<Label>>),
}

impl ComposeDecorator for CheckBoxDecorator {
  fn compose_decorator(_: State<Self>, host: Widget) -> impl WidgetBuilder { fn_widget!(host) }
}

impl ComposeChild for Checkbox {
  type Child = Option<CheckboxTemplate>;

  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> impl WidgetBuilder {
    fn_widget! {
      let CheckBoxStyle {
        icon_size,
        label_style,
        label_color,
      } = CheckBoxStyle::of(ctx!());

      let icon = @CheckBoxDecorator {
        color: pipe!($this.color),
        @Icon { size: icon_size,
          @ { pipe!{
            if $this.indeterminate {
              svgs::INDETERMINATE_CHECK_BOX
            } else if $this.checked {
              svgs::CHECK_BOX
            } else {
              svgs::CHECK_BOX_OUTLINE_BLANK
            }
          }}
        }
      }.build(ctx!());

      let checkbox = if let Some(child) = child  {
        let label = |label: State<Label>| @Text {
          text: $label.0.clone(),
          foreground: label_color,
          text_style: label_style,
        };

        @Row {
          @ {
            match child {
              CheckboxTemplate::Before(w) => {
                [w.child_replace_host().map(label).build(ctx!()), icon]
              },
              CheckboxTemplate::After(w) => {
                [icon, w.child_replace_host().map(label).build(ctx!())]
              },
            }
          }
        }.build(ctx!())
      } else {
        icon
      };

      @ $checkbox {
        cursor: CursorIcon::Pointer,
        on_tap: move |_| $this.write().switch_check(),
        on_key_up: move |k| if *k.key() == VirtualKey::Named(NamedKey::Space) {
          $this.write().switch_check()
        }
      }
    }
  }
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
  use ribir_core::test_helper::*;
  use ribir_dev_helper::*;

  use super::*;

  fn checked() -> impl WidgetBuilder {
    fn_widget! { @Checkbox { checked: true } }
  }
  widget_test_suit!(
    checked,
    wnd_size = Size::new(48., 48.),
    width == 24.,
    height == 24.,
    comparison = 0.001
  );

  fn unchecked() -> impl WidgetBuilder {
    fn_widget! { @Checkbox {} }
  }
  widget_test_suit!(
    unchecked,
    wnd_size = Size::new(48., 48.),
    width == 24.,
    height == 24.,
    comparison = 0.001
  );

  fn indeterminate() -> impl WidgetBuilder {
    fn_widget! {
      @Checkbox {
        checked: true,
        indeterminate: true,
      }
    }
  }

  widget_test_suit!(
    indeterminate,
    wnd_size = Size::new(48., 48.),
    width == 24.,
    height == 24.,
    comparison = 0.001
  );
}
