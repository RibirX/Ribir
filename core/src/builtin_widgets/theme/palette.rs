use ribir_painter::{Color, LightnessTone};

use super::{Brightness, Theme};
use crate::prelude::BuildCtx;

/// The palette enables you to modify the color of your application to suit
/// your brand. `Palette` provide colors base on the 8 key colors with different
/// light tone.
///
/// Note: `Palette` mainly learn from Material design color system
/// Reference https://m3.material.io/styles/color/
#[derive(Clone, Debug)]
pub struct Palette {
  // Accent colors: primary, secondary, and tertiary
  /// The primary key color is used to derive roles for key components across
  /// the UI, such as the FAB, prominent buttons, active states, as well as the
  /// tint of elevated surfaces.
  pub primary: Color,

  /// The secondary key color is used for less prominent components in the UI
  /// such as filter chips, while expanding the opportunity for color
  /// expression.
  pub secondary: Color,

  /// The tertiary key color is used to derive the roles of contrasting accents
  /// that can be used to balance primary and secondary colors or bring
  /// heightened attention to an element. The tertiary color role is left for
  /// teams to use at their discretion and is intended to support broader color
  /// expression in products.
  pub tertiary: Color,

  // Neutral colors: roles are used for surfaces and backgrounds, as well as high emphasis
  // text and icons.
  /// The neutral key color is used to derive the roles of surface and
  /// background, as well as high emphasis text and icons.
  pub neutral: Color,

  /// The neutral variant key color is used to derive medium emphasis text and
  /// icons, surface variants, and component outlines.
  pub neutral_variant: Color,

  // Functional Colors
  /// A semantic color role for error, used to represent interface elements that
  /// the user should be made aware of.
  pub error: Color,

  /// A semantic color role for warning, used to represent potentially dangerous
  /// actions or important messages.
  pub warning: Color,

  /// A semantic color role for success,  used to indicate the successful
  /// completion of an action that user triggered.
  pub success: Color,

  /// Dark or light theme.
  pub brightness: Brightness,

  /// Config the key color light theme lightness policy
  pub light: LightnessCfg,

  /// Config the key color dark theme lightness policy
  pub dark: LightnessCfg,
}

/// The four light tone to generate compatible colors group for color.
#[derive(Clone, Debug)]
pub struct LightnessGroup {
  /// The light amount of base color.
  pub base: LightnessTone,
  /// The light amount of the color sits on base color.
  pub on: LightnessTone,
  /// The light amount of this color as container.
  pub container: LightnessTone,
  /// The light amount of the color sit on the container color.
  pub on_container: LightnessTone,
}

/// Config the light tones of color
#[derive(Clone, Debug)]
pub struct LightnessCfg {
  /// The light tone group of color.
  pub color_group: LightnessGroup,
  pub surface: LightnessTone,
  pub surface_dim: LightnessTone,
  pub surface_bright: LightnessTone,
  pub surface_container_lowest: LightnessTone,
  pub surface_container_low: LightnessTone,
  pub surface_container: LightnessTone,
  pub surface_container_high: LightnessTone,
  pub surface_container_highest: LightnessTone,
  pub surface_variant: LightnessTone,
  pub on_surface: LightnessTone,
  pub on_surface_variant: LightnessTone,
  pub inverse_surface: LightnessTone,
  pub inverse_on_surface: LightnessTone,
  pub outline: LightnessTone,
  pub outline_variant: LightnessTone,
  pub shadow: LightnessTone,
}

impl Palette {
  #[inline]
  pub fn of<'a>(ctx: &'a BuildCtx) -> &'a Self {
    ctx
      .find_cfg(|t| match t {
        Theme::Full(f) => Some(&f.palette),
        Theme::Inherit(i) => i.palette.as_ref(),
      })
      .unwrap()
  }

  #[inline]
  pub fn primary(&self) -> Color { self.base_of(&self.primary) }

  #[inline]
  pub fn on_primary(&self) -> Color { self.on_of(&self.primary) }

  #[inline]
  pub fn primary_container(&self) -> Color { self.container_of(&self.primary) }

  #[inline]
  pub fn on_primary_container(&self) -> Color { self.on_container_of(&self.primary) }

  #[inline]
  pub fn secondary(&self) -> Color { self.base_of(&self.secondary) }

  #[inline]
  pub fn on_secondary(&self) -> Color { self.on_of(&self.secondary) }

  #[inline]
  pub fn secondary_container(&self) -> Color { self.container_of(&self.secondary) }

  #[inline]
  pub fn on_secondary_container(&self) -> Color { self.on_container_of(&self.secondary) }

  #[inline]
  pub fn tertiary(&self) -> Color { self.base_of(&self.tertiary) }

  #[inline]
  pub fn on_tertiary(&self) -> Color { self.on_of(&self.tertiary) }

  #[inline]
  pub fn tertiary_container(&self) -> Color { self.container_of(&self.tertiary) }

  #[inline]
  pub fn on_tertiary_container(&self) -> Color { self.on_container_of(&self.tertiary) }

  #[inline]
  pub fn success(&self) -> Color { self.base_of(&self.success) }

  #[inline]
  pub fn on_success(&self) -> Color { self.on_of(&self.success) }

  #[inline]
  pub fn success_container(&self) -> Color { self.container_of(&self.success) }

  #[inline]
  pub fn on_success_container(&self) -> Color { self.on_container_of(&self.success) }

  #[inline]
  pub fn warning(&self) -> Color { self.base_of(&self.warning) }

  #[inline]
  pub fn on_warning(&self) -> Color { self.on_of(&self.warning) }

  #[inline]
  pub fn warning_container(&self) -> Color { self.container_of(&self.warning) }

  #[inline]
  pub fn on_warning_container(&self) -> Color { self.on_container_of(&self.warning) }

  #[inline]
  pub fn error(&self) -> Color { self.base_of(&self.error) }

  #[inline]
  pub fn on_error(&self) -> Color { self.on_of(&self.error) }

  #[inline]
  pub fn error_container(&self) -> Color { self.container_of(&self.error) }

  #[inline]
  pub fn on_error_container(&self) -> Color { self.on_container_of(&self.error) }

  #[inline]
  pub fn background(&self) -> Color {
    self
      .neutral
      .with_lightness(self.lightness_cfg().surface)
  }

  #[inline]
  pub fn on_background(&self) -> Color {
    self
      .neutral
      .with_lightness(self.lightness_cfg().on_surface)
  }

  #[inline]
  pub fn surface(&self) -> Color { self.background() }

  #[inline]
  pub fn surface_dim(&self) -> Color {
    self
      .neutral
      .with_lightness(self.lightness_cfg().surface_dim)
  }

  #[inline]
  pub fn surface_bright(&self) -> Color {
    self
      .neutral
      .with_lightness(self.lightness_cfg().surface_bright)
  }

  #[inline]
  pub fn surface_container_lowest(&self) -> Color {
    self
      .neutral
      .with_lightness(self.lightness_cfg().surface_container_lowest)
  }

  #[inline]
  pub fn surface_container_low(&self) -> Color {
    self
      .neutral
      .with_lightness(self.lightness_cfg().surface_container_low)
  }

  #[inline]
  pub fn surface_container(&self) -> Color {
    self
      .neutral
      .with_lightness(self.lightness_cfg().surface_container)
  }

  #[inline]
  pub fn surface_container_high(&self) -> Color {
    self
      .neutral
      .with_lightness(self.lightness_cfg().surface_container_high)
  }

  #[inline]
  pub fn surface_container_highest(&self) -> Color {
    self
      .neutral
      .with_lightness(self.lightness_cfg().surface_container_highest)
  }

  #[inline]
  pub fn on_surface(&self) -> Color { self.on_background() }

  #[inline]
  pub fn surface_variant(&self) -> Color {
    self
      .neutral_variant
      .with_lightness(self.lightness_cfg().surface_variant)
  }

  #[inline]
  pub fn on_surface_variant(&self) -> Color {
    self
      .neutral_variant
      .with_lightness(self.lightness_cfg().on_surface_variant)
  }

  #[inline]
  pub fn outline(&self) -> Color {
    self
      .neutral_variant
      .with_lightness(self.lightness_cfg().outline)
  }

  pub fn outline_variant(&self) -> Color {
    self
      .neutral_variant
      .with_lightness(self.lightness_cfg().outline_variant)
  }

  #[inline]
  pub fn inverse_surface(&self) -> Color {
    self
      .neutral
      .with_lightness(self.lightness_cfg().inverse_surface)
  }

  #[inline]
  pub fn inverse_on_surface(&self) -> Color {
    self
      .neutral
      .with_lightness(self.lightness_cfg().inverse_on_surface)
  }

  #[inline]
  pub fn shadow(&self) -> Color {
    self
      .neutral
      .with_lightness(self.lightness_cfg().shadow)
  }

  #[inline]
  pub fn scrim(&self) -> Color { self.shadow() }

  /// change color to the `base` light tone of the palette.
  #[inline]
  pub fn base_of(&self, color: &Color) -> Color {
    color.with_lightness(self.lightness_cfg().color_group.base)
  }

  /// change color to the `container` light tone of the palette.
  #[inline]
  pub fn container_of(&self, color: &Color) -> Color {
    color.with_lightness(self.lightness_cfg().color_group.container)
  }

  /// change color to the `on`  light tone of the palette.
  #[inline]
  pub fn on_of(&self, color: &Color) -> Color {
    color.with_lightness(self.lightness_cfg().color_group.on)
  }

  /// change color to the `on`  light tone of the palette.
  #[inline]
  pub fn on_container_of(&self, color: &Color) -> Color {
    color.with_lightness(self.lightness_cfg().color_group.on_container)
  }

  #[inline]
  fn lightness_cfg(&self) -> &LightnessCfg {
    match self.brightness {
      Brightness::Dark => &self.dark,
      Brightness::Light => &self.light,
    }
  }
}

impl LightnessGroup {
  #[inline]
  fn light_theme_default() -> Self {
    Self {
      base: LightnessTone::new(0.4),
      on: LightnessTone::new(1.),
      container: LightnessTone::new(0.9),
      on_container: LightnessTone::new(0.1),
    }
  }

  #[inline]
  fn dark_theme_default() -> Self {
    Self {
      base: LightnessTone::new(0.8),
      on: LightnessTone::new(0.2),
      container: LightnessTone::new(0.3),
      on_container: LightnessTone::new(0.9),
    }
  }
}

impl LightnessCfg {
  #[inline]
  pub fn light_theme_default() -> Self {
    Self {
      color_group: LightnessGroup::light_theme_default(),
      outline: LightnessTone::new(0.5),
      outline_variant: LightnessTone::new(0.8),
      inverse_surface: LightnessTone::new(0.2),
      surface: LightnessTone::new(0.98),
      surface_dim: LightnessTone::new(0.87),
      surface_bright: LightnessTone::new(0.98),
      surface_container_lowest: LightnessTone::new(1.),
      surface_container_low: LightnessTone::new(0.96),
      surface_container: LightnessTone::new(0.94),
      surface_container_high: LightnessTone::new(0.92),
      surface_container_highest: LightnessTone::new(0.9),
      surface_variant: LightnessTone::new(0.9),
      on_surface: LightnessTone::new(0.1),
      on_surface_variant: LightnessTone::new(0.3),
      inverse_on_surface: LightnessTone::new(0.95),
      shadow: LightnessTone::new(0.),
    }
  }

  #[inline]
  pub fn dark_theme_default() -> Self {
    Self {
      color_group: LightnessGroup::dark_theme_default(),
      outline: LightnessTone::new(0.6),
      outline_variant: LightnessTone::new(0.3),
      inverse_surface: LightnessTone::new(0.9),
      surface: LightnessTone::new(0.06),
      surface_dim: LightnessTone::new(0.06),
      surface_bright: LightnessTone::new(0.24),
      surface_container_lowest: LightnessTone::new(0.04),
      surface_container_low: LightnessTone::new(0.1),
      surface_container: LightnessTone::new(0.12),
      surface_container_high: LightnessTone::new(0.17),
      surface_container_highest: LightnessTone::new(0.22),
      surface_variant: LightnessTone::new(0.3),
      on_surface: LightnessTone::new(0.9),
      on_surface_variant: LightnessTone::new(0.8),
      inverse_on_surface: LightnessTone::new(0.2),
      shadow: LightnessTone::new(0.),
    }
  }
}

impl Default for Palette {
  fn default() -> Self {
    Palette {
      primary: Color::from_u32(0x6750A4FF),
      secondary: Color::from_u32(0x625B71FF),
      tertiary: Color::from_u32(0x7D5260FF),
      neutral: Color::from_u32(0xFFFBFEFF),
      neutral_variant: Color::from_u32(0xE7E0ECFF),
      error: Color::from_u32(0xB3261EFF),
      warning: Color::from_u32(0xFFB74DFF),
      success: Color::from_u32(0x81C784FF),
      brightness: Brightness::Light,
      light: LightnessCfg::light_theme_default(),
      dark: LightnessCfg::dark_theme_default(),
    }
  }
}
