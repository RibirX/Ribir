use std::cell::Ref;

use painter::{Color, LightnessTone};

use crate::prelude::BuildCtx;

use super::Theme;

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

  /// Config the key color lightness policy
  pub lightness_cfg: LightnessCfg,
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
  /// The light tone of neutral
  pub neutral: LightnessTone,
  /// The light tone of on neutral
  pub on_neutral: LightnessTone,
  /// The light tone of inverse surface.
  pub inverse_surface: LightnessTone,
  /// The light tone of on inverse surface.
  pub on_inverse_surface: LightnessTone,
  /// The light tone of variant neutral
  pub variant_neutral: LightnessTone,
  /// The light tone of on variant neutral
  pub on_variant_neutral: LightnessTone,
  /// The light tone of outline
  pub outline: LightnessTone,
}

impl Palette {
  #[inline]
  pub fn of<'a>(ctx: &'a BuildCtx) -> Ref<'a, Self> {
    ctx
      .find_cfg(|t| match t {
        Theme::Full(f) => Some(&f.palette),
        Theme::Inherit(i) => i.palette.as_ref(),
      })
      .unwrap()
  }

  #[inline]
  pub fn primary(&self) -> Color { self.base_of(self.primary) }

  #[inline]
  pub fn on_primary(&self) -> Color { self.on_of(self.primary) }

  #[inline]
  pub fn primary_container(&self) -> Color { self.container_of(self.primary) }

  #[inline]
  pub fn on_primary_container(&self) -> Color { self.on_container_of(self.primary) }

  #[inline]
  pub fn secondary(&self) -> Color { self.base_of(self.secondary) }

  #[inline]
  pub fn on_secondary(&self) -> Color { self.on_of(self.secondary) }

  #[inline]
  pub fn secondary_container(&self) -> Color { self.container_of(self.secondary) }

  #[inline]
  pub fn on_secondary_container(&self) -> Color { self.on_container_of(self.secondary) }

  #[inline]
  pub fn tertiary(&self) -> Color { self.base_of(self.tertiary) }

  #[inline]
  pub fn on_tertiary(&self) -> Color { self.on_of(self.tertiary) }

  #[inline]
  pub fn tertiary_container(&self) -> Color { self.container_of(self.tertiary) }

  #[inline]
  pub fn on_tertiary_container(&self) -> Color { self.on_container_of(self.tertiary) }

  #[inline]
  pub fn success(&self) -> Color { self.base_of(self.success) }

  #[inline]
  pub fn on_success(&self) -> Color { self.on_of(self.success) }

  #[inline]
  pub fn success_container(&self) -> Color { self.container_of(self.success) }

  #[inline]
  pub fn on_success_container(&self) -> Color { self.on_container_of(self.success) }

  #[inline]
  pub fn warning(&self) -> Color { self.base_of(self.warning) }

  #[inline]
  pub fn on_warning(&self) -> Color { self.on_of(self.warning) }

  #[inline]
  pub fn warning_container(&self) -> Color { self.container_of(self.warning) }

  #[inline]
  pub fn on_warning_container(&self) -> Color { self.on_container_of(self.warning) }

  #[inline]
  pub fn error(&self) -> Color { self.base_of(self.error) }

  #[inline]
  pub fn on_error(&self) -> Color { self.on_of(self.error) }

  #[inline]
  pub fn error_container(&self) -> Color { self.container_of(self.error) }

  #[inline]
  pub fn on_error_container(&self) -> Color { self.on_container_of(self.error) }

  #[inline]
  pub fn background(&self) -> Color { self.neutral.with_lightness(self.lightness_cfg.neutral) }

  #[inline]
  pub fn on_background(&self) -> Color {
    self.neutral.with_lightness(self.lightness_cfg.on_neutral)
  }

  #[inline]
  pub fn surface(&self) -> Color { self.background() }

  #[inline]
  pub fn on_surface(&self) -> Color { self.on_background() }

  #[inline]
  pub fn surface_variant(&self) -> Color {
    self
      .neutral_variant
      .with_lightness(self.lightness_cfg.variant_neutral)
  }

  #[inline]
  pub fn on_surface_variant(&self) -> Color {
    self
      .neutral_variant
      .with_lightness(self.lightness_cfg.on_variant_neutral)
  }

  #[inline]
  pub fn outline(&self) -> Color {
    self
      .neutral_variant
      .with_lightness(self.lightness_cfg.outline)
  }

  #[inline]
  pub fn inverse_surface(&self) -> Color {
    self
      .neutral
      .with_lightness(self.lightness_cfg.inverse_surface)
  }

  #[inline]
  pub fn on_inverse_surface(&self) -> Color {
    self
      .neutral
      .with_lightness(self.lightness_cfg.on_inverse_surface)
  }

  /// change color to the `base` light tone of the palette.
  #[inline]
  pub fn base_of(&self, color: Color) -> Color {
    color.with_lightness(self.lightness_cfg.color_group.base)
  }

  /// change color to the `container` light tone of the palette.
  #[inline]
  pub fn container_of(&self, color: Color) -> Color {
    color.with_lightness(self.lightness_cfg.color_group.container)
  }

  /// change color to the `on`  light tone of the palette.
  #[inline]
  pub fn on_of(&self, color: Color) -> Color {
    color.with_lightness(self.lightness_cfg.color_group.on)
  }

  /// change color to the `on`  light tone of the palette.
  #[inline]
  pub fn on_container_of(&self, color: Color) -> Color {
    color.with_lightness(self.lightness_cfg.color_group.on_container)
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
      neutral: LightnessTone::new(0.99),
      on_neutral: LightnessTone::new(0.1),
      variant_neutral: LightnessTone::new(0.9),
      on_variant_neutral: LightnessTone::new(0.3),
      outline: LightnessTone::new(0.5),
      inverse_surface: LightnessTone::new(0.2),
      on_inverse_surface: LightnessTone::new(0.95),
    }
  }

  #[inline]
  pub fn dark_theme_default() -> Self {
    Self {
      color_group: LightnessGroup::dark_theme_default(),
      neutral: LightnessTone::new(0.1),
      on_neutral: LightnessTone::new(0.9),
      variant_neutral: LightnessTone::new(0.3),
      on_variant_neutral: LightnessTone::new(0.8),
      outline: LightnessTone::new(0.6),
      inverse_surface: LightnessTone::new(0.9),
      on_inverse_surface: LightnessTone::new(0.2),
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
      warning: Color::from_u32(0xffb74dFF),
      success: Color::from_u32(0x81c784FF),
      lightness_cfg: LightnessCfg::light_theme_default(),
    }
  }
}
