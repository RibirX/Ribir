use painter::Color;

use crate::prelude::BuildCtx;

/// The palette enables you to modify the color of your application to suit
/// your brand. `Palette` provide colors base on the 8 key colors with different
/// light tone.
///
/// Note: `Palette` mainly learn from Material design color system, because its
/// clear logic.
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

/// Describe the light tone of a color, should between [0, 1.0], 0.0 gives
/// absolute black and 1.0 give the brightest white.
#[derive(Clone, Debug)]
pub struct LightnessTone(f32);

/// The four light tone to generate compatible colors group for accent color or
/// functional color
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
  /// The light tone group of accent color.
  pub accent_group: LightnessGroup,
  /// The light tone group of functional color.
  pub functional_group: LightnessGroup,
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
  pub fn of<'a>(ctx: &'a mut BuildCtx) -> &'a Self { &&ctx.theme().palette }

  #[inline]
  pub fn primary(&self) -> Color {
    self
      .primary
      .with_lightness(self.lightness_cfg.accent_group.base.0)
  }

  #[inline]
  pub fn on_primary(&self) -> Color {
    self
      .primary
      .with_lightness(self.lightness_cfg.accent_group.on.0)
  }

  #[inline]
  pub fn primary_container(&self) -> Color {
    self
      .primary
      .with_lightness(self.lightness_cfg.accent_group.container.0)
  }

  #[inline]
  pub fn on_primary_container(&self) -> Color {
    self
      .primary
      .with_lightness(self.lightness_cfg.accent_group.on_container.0)
  }

  #[inline]
  pub fn secondary(&self) -> Color {
    self
      .secondary
      .with_lightness(self.lightness_cfg.accent_group.base.0)
  }

  #[inline]
  pub fn on_secondary(&self) -> Color {
    self
      .secondary
      .with_lightness(self.lightness_cfg.accent_group.on.0)
  }

  #[inline]
  pub fn secondary_container(&self) -> Color {
    self
      .secondary
      .with_lightness(self.lightness_cfg.accent_group.container.0)
  }

  #[inline]
  pub fn on_secondary_container(&self) -> Color {
    self
      .secondary
      .with_lightness(self.lightness_cfg.accent_group.on_container.0)
  }

  #[inline]
  pub fn tertiary(&self) -> Color {
    self
      .tertiary
      .with_lightness(self.lightness_cfg.accent_group.base.0)
  }

  #[inline]
  pub fn on_tertiary(&self) -> Color {
    self
      .tertiary
      .with_lightness(self.lightness_cfg.accent_group.on.0)
  }

  #[inline]
  pub fn tertiary_container(&self) -> Color {
    self
      .tertiary
      .with_lightness(self.lightness_cfg.accent_group.container.0)
  }

  #[inline]
  pub fn on_tertiary_container(&self) -> Color {
    self
      .tertiary
      .with_lightness(self.lightness_cfg.accent_group.on_container.0)
  }

  #[inline]
  pub fn success(&self) -> Color {
    self
      .success
      .with_lightness(self.lightness_cfg.functional_group.base.0)
  }

  #[inline]
  pub fn on_succuss(&self) -> Color {
    self
      .success
      .with_lightness(self.lightness_cfg.functional_group.on.0)
  }

  #[inline]
  pub fn success_container(&self) -> Color {
    self
      .success
      .with_lightness(self.lightness_cfg.functional_group.container.0)
  }

  #[inline]
  pub fn on_success_container(&self) -> Color {
    self
      .success
      .with_lightness(self.lightness_cfg.functional_group.on_container.0)
  }

  #[inline]
  pub fn warning(&self) -> Color {
    self
      .warning
      .with_lightness(self.lightness_cfg.functional_group.base.0)
  }

  #[inline]
  pub fn on_warning(&self) -> Color {
    self
      .warning
      .with_lightness(self.lightness_cfg.functional_group.on.0)
  }

  #[inline]
  pub fn warning_container(&self) -> Color {
    self
      .warning
      .with_lightness(self.lightness_cfg.functional_group.container.0)
  }

  #[inline]
  pub fn on_warning_container(&self) -> Color {
    self
      .warning
      .with_lightness(self.lightness_cfg.functional_group.container.0)
  }

  #[inline]
  pub fn error(&self) -> Color {
    self
      .error
      .with_lightness(self.lightness_cfg.functional_group.on.0)
  }

  #[inline]
  pub fn on_error(&self) -> Color {
    self
      .error
      .with_lightness(self.lightness_cfg.functional_group.on.0)
  }

  #[inline]
  pub fn error_container(&self) -> Color {
    self
      .error
      .with_lightness(self.lightness_cfg.functional_group.container.0)
  }

  #[inline]
  pub fn on_error_container(&self) -> Color {
    self
      .error
      .with_lightness(self.lightness_cfg.functional_group.on_container.0)
  }

  #[inline]
  pub fn background(&self) -> Color { self.neutral.with_lightness(self.lightness_cfg.neutral.0) }

  #[inline]
  pub fn on_background(&self) -> Color {
    self.neutral.with_lightness(self.lightness_cfg.on_neutral.0)
  }

  #[inline]
  pub fn surface(&self) -> Color { self.background() }

  #[inline]
  pub fn on_surface(&self) -> Color { self.on_background() }

  #[inline]
  pub fn surface_variant(&self) -> Color {
    self
      .neutral_variant
      .with_lightness(self.lightness_cfg.variant_neutral.0)
  }

  #[inline]
  pub fn on_surface_variant(&self) -> Color {
    self
      .neutral_variant
      .with_lightness(self.lightness_cfg.on_variant_neutral.0)
  }

  #[inline]
  pub fn outline(&self) -> Color {
    self
      .neutral_variant
      .with_lightness(self.lightness_cfg.outline.0)
  }

  #[inline]
  pub fn inverse_surface(&self) -> Color {
    self
      .neutral
      .with_lightness(self.lightness_cfg.inverse_surface.0)
  }

  #[inline]
  pub fn on_inverse_surface(&self) -> Color {
    self
      .neutral
      .with_lightness(self.lightness_cfg.on_inverse_surface.0)
  }
}

impl LightnessTone {
  #[inline]
  pub fn new(tone: f32) -> Self { Self(tone.clamp(0., 1.0)) }
}

impl LightnessGroup {
  #[inline]
  fn light_theme_default() -> Self {
    Self {
      base: LightnessTone(0.4),
      on: LightnessTone(1.0),
      container: LightnessTone(0.9),
      on_container: LightnessTone(0.1),
    }
  }

  #[inline]
  fn dark_theme_default() -> Self {
    Self {
      base: LightnessTone(0.8),
      on: LightnessTone(0.2),
      container: LightnessTone(0.3),
      on_container: LightnessTone(0.9),
    }
  }
}

impl LightnessCfg {
  #[inline]
  pub fn light_theme_default() -> Self {
    Self {
      accent_group: LightnessGroup::light_theme_default(),
      functional_group: LightnessGroup::light_theme_default(),
      neutral: LightnessTone(0.99),
      on_neutral: LightnessTone(0.1),
      variant_neutral: LightnessTone(0.9),
      on_variant_neutral: LightnessTone(0.3),
      outline: LightnessTone(0.5),
      inverse_surface: LightnessTone(0.2),
      on_inverse_surface: LightnessTone(0.95),
    }
  }

  #[inline]
  pub fn dark_theme_default() -> Self {
    Self {
      accent_group: LightnessGroup::dark_theme_default(),
      functional_group: LightnessGroup::dark_theme_default(),
      neutral: LightnessTone(0.1),
      on_neutral: LightnessTone(0.9),
      variant_neutral: LightnessTone(0.3),
      on_variant_neutral: LightnessTone(0.8),
      outline: LightnessTone(0.6),
      inverse_surface: LightnessTone(0.9),
      on_inverse_surface: LightnessTone(0.2),
    }
  }
}
