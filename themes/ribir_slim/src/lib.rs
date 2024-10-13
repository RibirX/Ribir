use ribir_core::{fill_svgs, prelude::*};

pub fn with_palette(palette: Palette) -> Theme {
  let icon_size = IconSize {
    tiny: Size::new(18., 18.),
    small: Size::new(24., 24.),
    medium: Size::new(36., 36.),
    large: Size::new(48., 48.),
    huge: Size::new(64., 64.),
  };

  let mut icon_theme = IconTheme::new(icon_size);
  fill_svgs! {
    icon_theme,
    svgs::ADD: "./icons/add_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::ARROW_BACK: "./icons/arrow_back_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::ARROW_DROP_DOWN: "./icons/arrow_drop_down_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::ARROW_FORWARD: "./icons/arrow_forward_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::CANCEL: "./icons/cancel_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::CHECK_BOX: "./icons/check_box_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::CHECK_BOX_OUTLINE_BLANK: "./icons/check_box_outline_blank_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::CHEVRON_RIGHT: "./icons/chevron_right_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::CLOSE: "./icons/close_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::DELETE: "./icons/delete_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::DONE: "./icons/done_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::EXPAND_MORE: "./icons/expand_more_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::FAVORITE: "./icons/favorite_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::HOME: "./icons/home_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::INDETERMINATE_CHECK_BOX: "./icons/indeterminate_check_box_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::LOGIN: "./icons/login_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::LOGOUT: "./icons/logout_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::MENU: "./icons/menu_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::MORE_HORIZ: "./icons/more_horiz_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::MORE_VERT: "./icons/more_vert_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::OPEN_IN_NEW: "./icons/open_in_new_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::SEARCH: "./icons/search_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::SETTINGS: "./icons/settings_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::STAR: "./icons/star_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::TEXT_CARET: "./icons/text_caret.svg"
  };

  Theme {
    palette,
    typography_theme: typography_theme(),
    classes: <_>::default(),
    icon_theme,
    transitions_theme: Default::default(),
    compose_decorators: Default::default(),
    custom_styles: Default::default(),
    font_bytes: None,
    font_files: None,
  }
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
      text: TextStyle { line_height, font_size, letter_space, font_face, overflow: Overflow::Clip },
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
