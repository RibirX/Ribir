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
  Before(Leading<Label>),
  After(Trailing<Label>),
}

impl ComposeDecorator for CheckBoxDecorator {
  fn compose_decorator(_: State<Self>, host: Widget) -> Widget { host }
}

impl ComposeChild<'static> for Checkbox {
  type Child = Option<CheckboxTemplate>;

  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'static> {
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
      }.into_widget();

      let checkbox = if let Some(child) = child  {
        let label = |label: Label| @Text {
          text: label.0,
          foreground: label_color,
          text_style: label_style,
        };

        @Row {
          @ {
            match child {
              CheckboxTemplate::Before(w) => {
                [(label(w.0)).into_widget(), icon]
              },
              CheckboxTemplate::After(w) => {
                [icon, label(w.0).into_widget()]
              },
            }
          }
        }.into_widget()
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
    .into_widget()
  }
}

impl CustomStyle for CheckBoxStyle {
  fn default_style(ctx: &impl ProviderCtx) -> Self {
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

  widget_test_suit!(
    checked,
    WidgetTester::new(fn_widget! { @Checkbox { checked: true } })
      .with_wnd_size(Size::new(48., 48.))
      .with_comparison(0.001),
    LayoutCase::default().with_size(Size::new(24., 24.))
  );

  widget_test_suit!(
    unchecked,
    WidgetTester::new(fn_widget! { @Checkbox {} })
      .with_wnd_size(Size::new(48., 48.))
      .with_comparison(0.001),
    LayoutCase::default().with_size(Size::new(24., 24.))
  );

  widget_test_suit!(
    indeterminate,
    WidgetTester::new(fn_widget! {
      @Checkbox {
        checked: true,
        indeterminate: true,
      }
    })
    .with_wnd_size(Size::new(48., 48.))
    .with_comparison(0.001),
    LayoutCase::default().with_size(Size::new(24., 24.))
  );
}
