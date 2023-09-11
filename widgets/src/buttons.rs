use crate::prelude::*;
use ribir_core::prelude::*;

#[derive(Declare, Declare2)]
pub struct ButtonImpl {
  #[declare(default = 48.)]
  pub min_width: f32,
  pub height: f32,
  pub icon_size: Size,
  pub label_gap: f32,
  #[allow(unused)]
  pub icon_pos: IconPosition,
  pub label_style: CowArc<TextStyle>,
  pub foreground_color: Brush,
  #[declare(convert=strip_option)]
  pub background_color: Option<Brush>,
  #[declare(convert=strip_option)]
  pub radius: Option<f32>,
  #[declare(convert=strip_option)]
  pub border_style: Option<Border>,
  #[declare(convert=strip_option)]
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
  pub label: Option<State<Label>>,
  pub icon: Option<NamedSvg>,
}

impl ComposeChild for ButtonImpl {
  type Child = ButtonTemplate;

  fn compose_child(this: State<Self>, child: Self::Child) -> Widget {
    let ButtonTemplate { icon, label } = child;
    fn_widget! {
      @BoxDecoration {
        border_radius: pipe!($this.radius.map(Radius::all)),
        background: pipe!($this.background_color.clone()),
        border: pipe!($this.border_style.clone()),
        @ConstrainedBox {
          clamp: pipe!(BoxClamp::min_width($this.min_width)
            .with_fixed_height($this.height)),
          @{
            let padding = pipe!($this.padding_style.map(|padding| Padding { padding }));
            let icon = icon.map(|icon| @Icon {
              size: pipe!($this.icon_size),
              @{ icon }
            });
            let label = label.map(|mut label| @Text {
              margin: pipe!(EdgeInsets::horizontal($this.label_gap)),
              text: pipe!($label.0.clone()),
              foreground: pipe!($this.foreground_color.clone()),
              text_style: pipe!($this.label_style.clone())
            });

            @$ padding {
              @Row {
                justify_content: JustifyContent::Center,
                align_items: Align::Center,
                @ { icon }
                @{ label }
              }
            }
          }
        }
      }
    }
    .into()
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
