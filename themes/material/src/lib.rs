use ribir_core::{fill_svgs, prelude::*};
use ribir_widgets::prelude::*;
pub mod ripple;
pub mod state_layer;
pub use ripple::*;
pub use state_layer::*;
mod styles_sheet;
pub use styles_sheet::*;
mod classes;
mod interactive_layers;
pub mod md;
pub use interactive_layers::*;
mod focus_indicator;
pub use focus_indicator::*;

/// A provider used to hint widgets in the subtree to disable the ripple effect.
pub struct DisabledRipple(pub bool);

impl DisabledRipple {
  pub fn get(ctx: &impl AsRef<ProviderCtx>) -> bool {
    Provider::of::<Self>(ctx).is_some_and(|d| d.0)
  }
}

/// Crate a material theme with palette.
fn new(palette: Palette) -> Theme {
  let classes = classes::initd_classes();
  let mut theme = Theme {
    palette,
    typography_theme: typography_theme(),
    classes,
    icon_theme: icon_theme(),
    font_bytes: vec![
      include_bytes!("./fonts/Roboto-Regular.ttf").to_vec(),
      include_bytes!("./fonts/Roboto-Medium.ttf").to_vec(),
    ],
    ..Default::default()
  };

  fill_svgs! { theme.icon_theme,
    svgs::ADD: "./icons/add_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::ARROW_BACK: "./icons/arrow_back_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::ARROW_DROP_DOWN: "./icons/arrow_drop_down_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::ARROW_FORWARD: "./icons/arrow_forward_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::CANCEL: "./icons/cancel_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::CHEVRON_RIGHT: "./icons/chevron_right_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::CLOSE: "./icons/close_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::DELETE: "./icons/delete_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::DONE: "./icons/done_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::EXPAND_MORE: "./icons/expand_more_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::FAVORITE: "./icons/favorite_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::HOME: "./icons/home_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::LOGIN: "./icons/login_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::LOGOUT: "./icons/logout_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::MENU: "./icons/menu_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::MORE_VERT: "./icons/more_vert_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::SEARCH: "./icons/search_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::SETTINGS: "./icons/settings_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::STAR: "./icons/star_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::TEXT_CARET: "./icons/text_caret.svg",
    svgs::MORE_HORIZ: "./icons/more_horiz_FILL0_wght400_GRAD0_opsz48.svg"
  };

  fill_svgs! {
    theme.icon_theme,
    material_svgs::ADD_CIRCLE: "./icons/add_circle_FILL0_wght400_GRAD0_opsz48.svg",
    material_svgs::CHECK_CIRCLE: "./icons/check_circle_FILL0_wght400_GRAD0_opsz48.svg",
    material_svgs::CHECK: "./icons/check_FILL0_wght400_GRAD0_opsz48.svg",
    material_svgs::FILE_DOWNLOAD: "./icons/file_download_FILL0_wght400_GRAD0_opsz48.svg",
    material_svgs::GRADE: "./icons/grade_FILL0_wght400_GRAD0_opsz48.svg",
    material_svgs::REFRESH: "./icons/refresh_FILL0_wght400_GRAD0_opsz48.svg",
    material_svgs::SMS: "./icons/sms_FILL0_wght400_GRAD0_opsz48.svg",
    material_svgs::ACCOUNT_CIRCLE: "./icons/account_circle_FILL0_wght400_GRAD0_opsz48.svg",
    material_svgs::INFO: "./icons/info_FILL0_wght400_GRAD0_opsz48.svg"
  }

  theme
}

pub mod purple {
  use super::*;

  fn palette(brightness: Brightness) -> Palette {
    Palette {
      primary: Color::from_u32(0x6750A4FF),
      secondary: Color::from_u32(0x625B71FF),
      tertiary: Color::from_u32(0x7D5260FF),
      neutral: Color::from_u32(0xFFFBFEFF),
      neutral_variant: Color::from_u32(0xE7E0ECFF),
      error: Color::from_u32(0xB3261EFF),
      warning: Color::from_u32(0xFFB74DFF),
      success: Color::from_u32(0x81C784FF),
      brightness,
      light: LightnessCfg::light_theme_default(),
      dark: LightnessCfg::dark_theme_default(),
    }
  }

  /// A default light blue theme. Colors from <https://material.io/design/color/dark-theme.html#ui-application>
  pub fn light() -> Theme { new(palette(Brightness::Light)) }

  /// A default dark theme with a teal accent color. Colors from <https://material.io/design/color/dark-theme.html#ui-application>
  pub fn dark() -> Theme { new(palette(Brightness::Dark)) }
}

fn icon_theme() -> IconTheme {
  let icon_size = IconSize {
    tiny: md::SIZE_18,
    small: md::SIZE_24,
    medium: md::SIZE_36,
    large: md::SIZE_48,
    huge: md::SIZE_64,
  };
  IconTheme::new(icon_size)
}

/// Create a TypographyTheme which implement the typography styles base on the
/// material design specification.
///
/// The `titles_family` applied to headlines and subtitles and `body_family`
/// applied to body and caption. The `display_style` is applied to
/// headline4, headline3, headline2, headline1, and caption. The
/// `body_style` is applied to the remaining text styles.
pub fn typography_theme() -> TypographyTheme {
  let mut families = vec!["Roboto".into()];
  families.extend(
    fallback_font_families()
      .iter()
      .map(|f| (*f).into()),
  );
  families.push(FontFamily::SansSerif);
  let families = families.into_boxed_slice();

  let regular_face =
    FontFace { families: families.clone(), weight: FontWeight::NORMAL, ..<_>::default() };
  let medium_face = FontFace { families, weight: FontWeight::MEDIUM, ..<_>::default() };

  fn text_theme(
    line_height: f32, font_size: f32, letter_space: f32, font_face: FontFace,
  ) -> TextTheme {
    TextTheme {
      text: TextStyle {
        line_height,
        font_size,
        letter_space,
        font_face,
        overflow: TextOverflow::Overflow,
      },
      decoration: TextDecorationStyle {
        decoration: TextDecoration::NONE,
        decoration_color: Color::BLACK.with_alpha(0.87).into(),
      },
    }
  }

  TypographyTheme {
    display_large: text_theme(64., 57., 0., regular_face.clone()),
    display_medium: text_theme(52.0, 45.0, 0., regular_face.clone()),
    display_small: text_theme(44.0, 36.0, 0., regular_face.clone()),
    headline_large: text_theme(40.0, 32.0, 0., regular_face.clone()),
    headline_medium: text_theme(36.0, 28.0, 0., regular_face.clone()),
    headline_small: text_theme(32.0, 24.0, 0., regular_face.clone()),
    title_large: text_theme(28.0, 22.0, 0., medium_face.clone()),
    title_medium: text_theme(24.0, 16.0, 0.15, medium_face.clone()),
    title_small: text_theme(20.0, 14.0, 0.1, medium_face.clone()),
    label_large: text_theme(20.0, 14.0, 0.1, medium_face.clone()),
    label_medium: text_theme(16.0, 12.0, 0.5, medium_face.clone()),
    label_small: text_theme(16.0, 11.0, 0.5, medium_face),
    body_large: text_theme(24.0, 16.0, 0.5, regular_face.clone()),
    body_medium: text_theme(20.0, 14.0, 0.25, regular_face.clone()),
    body_small: text_theme(16.0, 12.0, 0.4, regular_face),
  }
}
