use ribir_core::prelude::*;
mod classes;
pub mod slim;

pub fn purple() -> Theme {
  let p = Palette {
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
  };

  with_palette(p)
}

pub fn with_palette(palette: Palette) -> Theme {
  let classes = classes::initd_classes();
  Theme { palette, classes, typography_theme: typography_theme(), ..Default::default() }
}

fn typography_theme() -> TypographyTheme {
  let regular_family = Box::new([FontFamily::Name(std::borrow::Cow::Borrowed("Lato"))]);
  let medium_family = Box::new([FontFamily::Name(std::borrow::Cow::Borrowed("Lato"))]);

  let regular_face =
    FontFace { families: regular_family.clone(), weight: FontWeight::NORMAL, ..<_>::default() };
  let medium_face =
    FontFace { families: medium_family, weight: FontWeight::MEDIUM, ..<_>::default() };

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
    display_medium: text_theme(52., 45., 0., regular_face.clone()),
    display_small: text_theme(44., 36., 0., regular_face.clone()),
    headline_large: text_theme(40., 32., 0., regular_face.clone()),
    headline_medium: text_theme(36., 28., 0., regular_face.clone()),
    headline_small: text_theme(32., 24., 0., regular_face.clone()),
    title_large: text_theme(28., 22., 0., medium_face.clone()),
    title_medium: text_theme(24., 16., 0.15, medium_face.clone()),
    title_small: text_theme(20., 14., 0.1, medium_face.clone()),
    label_large: text_theme(20., 14., 0.1, medium_face.clone()),
    label_medium: text_theme(16., 12., 0.5, medium_face.clone()),
    label_small: text_theme(16., 11., 0.5, medium_face),
    body_large: text_theme(24., 16., 0.5, regular_face.clone()),
    body_medium: text_theme(20., 14., 0.25, regular_face.clone()),
    body_small: text_theme(16., 12., 0.4, regular_face),
  }
}
