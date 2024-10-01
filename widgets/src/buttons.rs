use ribir_core::prelude::*;

use crate::prelude::*;

#[derive(Declare)]
pub struct ButtonImpl {
  #[declare(default = 48.)]
  pub min_width: f32,
  pub height: f32,
  pub icon_size: Size,
  pub label_gap: f32,
  #[allow(unused)]
  pub icon_pos: IconPosition,
  pub label_style: TextStyle,
  pub foreground_color: Brush,
  pub background_color: Option<Brush>,
  pub radius: Option<f32>,
  pub border_style: Option<Border>,
  pub padding_style: Option<EdgeInsets>,
}

/// Indicate icon position where before label or after label
#[derive(Default, Clone, Copy)]
pub enum IconPosition {
  #[default]
  Before,
  After,
}

/// Indicate the composition of the internal icon and label of the button
#[derive(Clone, Copy, PartialEq)]
pub enum ButtonType {
  // only icon
  ICON,
  // only label
  LABEL,
  // both icon and label
  BOTH,
}

#[derive(Template)]
pub struct ButtonTemplate {
  pub label: Option<Label>,
  pub icon: Option<NamedSvg>,
}

impl ComposeChild<'static> for ButtonImpl {
  type Child = ButtonTemplate;

  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'static> {
    let ButtonTemplate { icon, label } = child;
    fn_widget! {
      @ConstrainedBox {
        border_radius: pipe!($this.radius.map(Radius::all)),
        background: pipe!($this.background_color.clone()),
        border: pipe!($this.border_style.clone()),
        clamp: pipe!(BoxClamp::min_width($this.min_width)
          .with_fixed_height($this.height)),
        @{
          let padding = $this.padding_style.map(Padding::new);
          let icon = icon.map(|icon| @Icon {
            size: pipe!($this.icon_size),
            @{ icon }
          });
          let label = label.map(|label| @Text {
            margin: pipe!(EdgeInsets::horizontal($this.label_gap)),
            text: label.0,
            foreground: pipe!($this.foreground_color.clone()),
            text_style: pipe!($this.label_style.clone())
          });

          @$ padding {
            @Row {
              justify_content: JustifyContent::Center,
              align_items: Align::Center,
              @ { icon }
              @ { label }
            }
          }
        }
      }
    }
    .into_widget()
  }
}

mod filled_button;
pub use filled_button::*;

mod outlined_button;
pub use outlined_button::*;

mod button;
pub use button::*;

mod fab_button;
pub use fab_button::*;
