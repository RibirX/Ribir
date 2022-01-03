pub use super::*;
pub use painter::{Brush, Color, FontFamily, FontStyle, FontWeight};

/// A default light blue theme. Colors from <https://material.io/design/color/dark-theme.html#ui-application>
pub fn light(family: Box<[FontFamily]>) -> Theme {
  let dark_text = TypographyTheme::new(
    family.clone(),
    family.clone(),
    Color::BLACK.with_alpha(0.54).into(),
    Color::BLACK.with_alpha(0.87).into(),
    TextDecoration::NONE,
    Color::TRANSPARENT.into(),
  );
  let background = Color::from_u32(0xFFFF_FFFF);
  let secondary = Color::from_u32(0x03DA_C6FF);
  let unselected_widget_color = Color::BLACK.with_alpha(0.7);
  let mut checkbox = CheckboxTheme {
    check_background: secondary.clone(),
    border_color: unselected_widget_color.clone().with_alpha(0.65),
    ..Default::default()
  };
  checkbox.indeterminate_path.brush = background.clone().into();
  checkbox.checked_path.brush = background.clone().into();
  Theme {
    brightness: Brightness::Light,
    checkbox,
    primary: Color::from_u32(0x6200_EEFF),
    primary_variant: Color::from_u32(0x3700_B3FF),
    secondary,
    secondary_variant: Color::from_u32(0x0187_86FF),
    background,
    surface: Color::from_u32(0xFFFF_FFFF),
    error: Color::from_u32(0xB000_20FF),
    on_primary: Color::from_u32(0xFFFF_FFFF),
    on_secondary: Color::from_u32(0),
    on_background: Color::from_u32(0),
    on_surface: Color::from_u32(0),
    on_error: Color::from_u32(0xFFFF_FFFF),
    typography_theme: dark_text,
    default_font_family: family,
    unselected_widget_color,
  }
}

/// A default dark theme with a teal accent color. Colors from <https://material.io/design/color/dark-theme.html#ui-application>
pub fn dark(family: Box<[FontFamily]>) -> Theme {
  let unselected_widget_color = Color::WHITE.with_alpha(0.7);
  let background = Color::from_u32(0x1212_12FF);
  let secondary = Color::from_u32(0x03DA_C6FF);
  let light_text = TypographyTheme::new(
    family.clone(),
    family.clone(),
    Color::WHITE.with_alpha(0.70).into(),
    Color::WHITE.into(),
    TextDecoration::NONE,
    Color::TRANSPARENT.into(),
  );
  let mut checkbox = CheckboxTheme {
    check_background: secondary.clone(),
    border_color: unselected_widget_color.clone().with_alpha(0.65),
    ..Default::default()
  };
  checkbox.indeterminate_path.brush = background.clone().into();
  checkbox.checked_path.brush = background.clone().into();

  Theme {
    brightness: Brightness::Dark,
    checkbox,
    primary: Color::from_u32(0xBB86_FCFF),
    primary_variant: Color::from_u32(0x3700_B3FF),
    secondary,
    secondary_variant: Color::from_u32(0x1212_12FF),
    background,
    surface: Color::from_u32(0x1212_12FF),
    error: Color::from_u32(0xCF66_79FF),
    on_primary: Color::from_u32(0),
    on_secondary: Color::from_u32(0),
    on_background: Color::from_u32(0xFFFF_FFFF),
    on_surface: Color::from_u32(0xFFFF_FFFF),
    on_error: Color::from_u32(0),
    typography_theme: light_text,
    default_font_family: family,
    unselected_widget_color,
  }
}
