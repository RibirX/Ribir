use self::palette::LightnessCfg;
pub use super::*;
use crate::prelude::include_svg;
pub use painter::{Brush, Color};

/// Crate a material theme with palette.
pub fn new(brightness: Brightness, palette: Palette) -> Theme {
  let family = Box::new([FontFamily::Name(std::borrow::Cow::Borrowed("Roboto"))]);
  let typography_theme = TypographyTheme::new(
    family.clone(),
    family.clone(),
    palette.on_background().into(),
    palette.on_surface_variant().into(),
    TextDecoration::NONE,
    Color::BLACK.with_alpha(0.87).into(),
  );

  let scrollbar = ScrollBarTheme {
    track: ScrollBoxDecorationStyle {
      background: Color::SILVER.into(),
      radius: None,
      thickness: 12.,
    },

    thumb: ScrollBoxDecorationStyle {
      background: Color::GRAY.into(),
      radius: Some(Radius::all(4.)),
      thickness: 12.,
    },
    thumb_min_size: 12.,
  };
  Theme {
    brightness,
    palette,
    typography_theme,
    default_font_family: family,
    scrollbar,
    icon_theme: icon_theme(),
  }
}
pub mod purple {
  use super::*;

  fn palette(lightness_cfg: LightnessCfg) -> Palette {
    Palette {
      primary: Color::from_u32(0x6750A4FF),
      secondary: Color::from_u32(0x625B71FF),
      tertiary: Color::from_u32(0x7D5260FF),
      neutral: Color::from_u32(0xFFFBFEFF),
      neutral_variant: Color::from_u32(0xE7E0ECFF),
      error: Color::from_u32(0xB3261EFF),
      warning: Color::from_u32(0xffb74dFF),
      success: Color::from_u32(0x81c784FF),
      lightness_cfg,
    }
  }

  /// A default light blue theme. Colors from <https://material.io/design/color/dark-theme.html#ui-application>
  pub fn light() -> Theme {
    let palette = palette(LightnessCfg::light_theme_default());
    new(Brightness::Light, palette)
  }

  /// A default dark theme with a teal accent color. Colors from <https://material.io/design/color/dark-theme.html#ui-application>
  pub fn dark() -> Theme {
    let palette = palette(LightnessCfg::dark_theme_default());
    new(Brightness::Dark, palette)
  }
}

fn icon_theme() -> IconTheme {
  macro_rules! include_icon {
    ($path: literal) => {
      ShareResource::new(include_svg!($path))
    };
  }

  IconTheme {
    icon_size: IconSize {
      tiny: Size::new(18., 18.),
      small: Size::new(24., 24.),
      medium: Size::new(36., 36.),
      large: Size::new(48., 48.),
      huge: Size::new(64., 64.),
    },
    builtin_icons: SvgIcons {
      checked: include_icon!("./material/checked.svg"),
      unchecked: include_icon!("./material/unchecked_box.svg"),
      indeterminate: include_icon!("./material/indeterminate_check_box.svg"),
    },
  }
}
