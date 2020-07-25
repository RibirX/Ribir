pub use super::theme_data::*;
pub use canvas::{Color, FillStyle, FontStyle, FontWeight};

impl ThemeData {
  /// A default light blue theme.
  pub fn light() -> Self {
    let family = "".to_string();
    let dark_text = TypographyTheme::new(
      family.clone(),
      family,
      Color::BLACK.with_alpha(0.54).into(),
      Color::BLACK.with_alpha(0.87).into(),
      TextDecoration::NONE,
      Color::TRANSPARENT,
    );
    unimplemented!()
  }

  /// A default dark theme with a teal accent color.
  pub fn dark() -> Self {
    let family = "".to_string();
    let light_text = TypographyTheme::new(
      family.clone(),
      family,
      Color::WHITE.with_alpha(0.70).into(),
      Color::WHITE.into(),
      TextDecoration::NONE,
      Color::TRANSPARENT,
    );
    unimplemented!()
  }
}
