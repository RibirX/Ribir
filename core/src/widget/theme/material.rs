pub use super::theme_data::*;
pub use canvas::{Color, FillStyle, FontStyle, FontWeight};

/// A default light blue theme. Colors from https://material.io/design/color/dark-theme.html#ui-application
pub fn light(family: String) -> ThemeData {
  let dark_text = TypographyTheme::new(
    family.clone(),
    family.clone(),
    Color::BLACK.with_alpha(0.54).into(),
    Color::BLACK.with_alpha(0.87).into(),
    TextDecoration::NONE,
    Color::TRANSPARENT,
  );
  ThemeData {
    brightness: Brightness::Light,
    primary: Color::from_u32(0x6200EEFF),
    primary_variant: Color::from_u32(0x3700B3FF),
    secondary: Color::from_u32(0x03DAC6FF),
    secondary_variant: Color::from_u32(0x018786FF),
    background: Color::from_u32(0xFFFFFFFF),
    surface: Color::from_u32(0xFFFFFFFF),
    error: Color::from_u32(0xB00020FF),
    on_primary: Color::from_u32(0xFFFFFFFF),
    on_secondary: Color::from_u32(0),
    on_background: Color::from_u32(0),
    on_surface: Color::from_u32(0),
    on_error: Color::from_u32(0xFFFFFFFF),
    typography_theme: dark_text,
    default_font_family: family,
  }
}

/// A default dark theme with a teal accent color. Colors from https://material.io/design/color/dark-theme.html#ui-application
pub fn dark(family: String) -> ThemeData {
  let light_text = TypographyTheme::new(
    family.clone(),
    family.clone(),
    Color::WHITE.with_alpha(0.70).into(),
    Color::WHITE.into(),
    TextDecoration::NONE,
    Color::TRANSPARENT,
  );
  ThemeData {
    brightness: Brightness::Light,
    primary: Color::from_u32(0xBB86FCFF),
    primary_variant: Color::from_u32(0x3700B3FF),
    secondary: Color::from_u32(0x03DAC6FF),
    secondary_variant: Color::from_u32(0x121212FF),
    background: Color::from_u32(0x121212FF),
    surface: Color::from_u32(0x121212FF),
    error: Color::from_u32(0xCF6679FF),
    on_primary: Color::from_u32(0),
    on_secondary: Color::from_u32(0),
    on_background: Color::from_u32(0xFFFFFFFF),
    on_surface: Color::from_u32(0xFFFFFFFF),
    on_error: Color::from_u32(0),
    typography_theme: light_text,
    default_font_family: family,
  }
}
