use ribir_core::prelude::*;

use crate::text::*;

/// An icon widget represents an icon.
///
/// It can accept either text or another widget as its child. If the child is
/// text, the widget uses the font ligature to display the text as an icon.
/// Therefore, an icon font must be provided.
///
///
/// # Example
///
/// ```
/// use ribir_core::prelude::*;
/// use ribir_widgets::prelude::*;
///
/// // To use an icon font, set the icon font before running the app.
/// let mut theme = AppCtx::app_theme().write();
/// theme
///   .font_files
///   .push("the font file path".to_string());
/// theme.icon_font = FontFace {
///   families: Box::new([FontFamily::Name("Your icon font family name".into())]),
///   // The rest of the face configuration depends on your font file
///   ..<_>::default()
/// };
///
/// // Using a named SVG as an icon
/// let _icon = icon! { @ { svgs::DELETE } };
/// // Using a font icon
/// let _icon = icon! { @ { "search" } };
/// // Using any widget you want
/// let _icon = icon! {
///   @Container {
///     size: Size::new(200., 200.),
///     background: Color::RED,
///   }
/// };
/// ```
///
/// The size of the icon is determined by the `ICON` class. If you need a
/// different size for the icon, you can override the `ICON` class to apply the
/// changes to the icon within its subtree.
///
///
/// ```
/// use ribir_core::prelude::*;
/// use ribir_widgets::prelude::*;
///
/// let w = fn_widget! {
///   @OverrideClass {
///     name: ICON,
///     class_impl: style_class! {
///       clamp: BoxClamp::fixed_size(Size::new(64., 64.)),
///       text_style: TextStyle {
///         font_face: Theme::of(BuildCtx::get()).icon_font.clone(),
///         line_height: 64.,
///         font_size: 64.,
///         ..<_>::default()
///       }
///     } as ClassImpl,
///     @icon! { @ { svgs::DELETE } }
///   }
/// };
/// ```
#[derive(Declare, Default, Clone, Copy)]
pub struct Icon;

class_names! {
  #[doc = "This class is used to specify the size of the icon and the text style for the icon."]
  ICON,
}

#[derive(Template)]
pub enum IconChild<'c> {
  /// The text to display as a icon.
  ///
  /// Use a `DeclareInit<CowArc<str>>` so that we can accept a pipe text.
  FontIcon(DeclareInit<CowArc<str>>),
  Widget(Widget<'c>),
}

impl<'c> ComposeChild<'c> for Icon {
  type Child = IconChild<'c>;
  fn compose_child(_: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
    let child = match child {
      IconChild::FontIcon(text) => text! { text }.into_widget(),
      IconChild::Widget(child) => child,
    };

    let icon = FatObj::new(child)
      .box_fit(BoxFit::Contain)
      .h_align(HAlign::Center)
      .v_align(VAlign::Center)
      .into_widget();

    // We need apply class after align and box_fit.
    Class { class: Some(ICON) }
      .with_child(icon)
      .into_widget()
  }
}

#[cfg(test)]
mod tests {
  use ribir_core::test_helper::*;
  use ribir_dev_helper::*;

  use super::*;
  use crate::prelude::*;

  widget_image_tests!(
    icons,
    WidgetTester::new(row! {
      @Icon { @ { svgs::DELETE }}
      @Icon { @ { "search" } }
      @Icon { @SpinnerProgress { value: Some(0.8) }}
    })
    .with_wnd_size(Size::new(300., 200.))
    .with_env_init(|| {
      let mut theme = AppCtx::app_theme().write();
      // Specify the icon font.
      theme
        .font_bytes
        .push(include_bytes!("../../fonts/material-search.ttf").to_vec());
      theme.icon_font = FontFace {
        families: Box::new([FontFamily::Name("Material Symbols Rounded 48pt".into())]),
        weight: FontWeight::NORMAL,
        ..<_>::default()
      };
    })
    .with_comparison(0.002)
  );
}
