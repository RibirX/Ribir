//! Theme use to share visual config or style compose logic. It can be defined
//! to app-wide or particular part of the application.

use crate::{fill_svgs, prelude::*, widget::WidgetBuilder};
use ahash::{HashMap, HashSet};
use ribir_algo::Sc;
pub use ribir_algo::{CowArc, ShareResource};
use ribir_geom::Size;
use ribir_macros::Declare;
use ribir_text::{font_db::ID, TextStyle};
use std::{rc::Rc, vec};

mod palette;
pub use palette::*;
mod icon_theme;
pub use icon_theme::*;
mod typography_theme;
pub use typography_theme::*;
mod transition_theme;
pub use transition_theme::*;
mod compose_decorators;
pub use compose_decorators::*;
mod custom_styles;
pub use custom_styles::*;

pub use ribir_painter::*;
pub use ribir_text::{FontFace, FontFamily, FontSize, FontWeight, Pixel};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Brightness {
  Dark,
  Light,
}

/// A full theme means all config have be defined in it. Everything of parent
/// theme are overriding here, if anything that you can't find means it be
/// override as undefine, should not continue find in parent theme.
pub struct FullTheme {
  pub palette: Rc<Palette>,
  pub typography_theme: TypographyTheme,
  pub icon_theme: IconTheme,
  pub transitions_theme: TransitionTheme,
  pub compose_decorators: ComposeDecorators,
  pub custom_styles: CustomStyles,
  pub font_bytes: Option<Vec<Vec<u8>>>,
  pub font_files: Option<Vec<String>>,
}

/// Inherit theme override part of parent theme, if anything not found in here,
/// should query in parent theme until meet a `FullTheme`.
#[derive(Default)]
pub struct InheritTheme {
  pub palette: Option<Rc<Palette>>,
  pub typography_theme: Option<TypographyTheme>,
  /// icon size standard
  pub icon_size: Option<IconSize>,
  /// a collection of icons.
  pub icons: Option<HashMap<NamedSvg, ShareResource<Svg>>>,
  pub transitions_theme: Option<TransitionTheme>,
  pub compose_decorators: Option<ComposeDecorators>,
  pub custom_styles: Option<CustomStyles>,
  pub font_bytes: Option<Vec<Vec<u8>>>,
  pub font_files: Option<Vec<String>>,
}

#[derive(Query)]
pub enum Theme {
  Full(FullTheme),
  Inherit(InheritTheme),
}

#[derive(Declare)]
pub struct ThemeWidget {
  pub theme: Sc<Theme>,
}

impl ComposeChild for ThemeWidget {
  type Child = GenWidget;
  #[inline]
  fn compose_child(
    this: impl StateWriter<Value = Self>,
    mut child: Self::Child,
  ) -> impl WidgetBuilder {
    use crate::prelude::*;
    fn_widget! {
      let theme = this.read().theme.clone();
      AppCtx::load_font_from_theme(&theme);

      let mut themes = ctx!().themes().clone();
      themes.push(theme.clone());

      let p = ThemeRender{ theme: theme.clone() }.widget_build(ctx!()).attach_data(theme, ctx!());
      // shadow the context with the theme.
      let ctx = BuildCtx::new_with_data(Some(p.id()), ctx!().tree, themes);
      let child = child.gen_widget(&ctx);
      ctx.append_child(p.id(), child);

      p
    }
  }
}

// ThemeRender will install default font for the subtree in the
// perform_layout.
#[derive(Query, SingleChild)]
struct ThemeRender {
  theme: Sc<Theme>,
}

impl Render for ThemeRender {
  #[inline]
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    let mut fonts = vec![];
    let font_db = AppCtx::font_db().clone();
    let font_families = match self.theme.deref() {
      Theme::Full(f) => Some(f.typography_theme.default_font_family.clone()),
      Theme::Inherit(i) => i
        .typography_theme
        .as_ref()
        .map(|typography_theme| typography_theme.default_font_family.clone()),
    };

    if let Some(families) = font_families {
      fonts = font_db
        .borrow_mut()
        .select_all_match(&FontFace { families, ..<_>::default() });
    }
    if !fonts.is_empty() {
      // install default font for the subtree.
      let old_fonts = font_db.borrow().default_fonts().to_vec();
      let mut set: HashSet<ID> = HashSet::from_iter(fonts.iter().cloned());
      for font in old_fonts {
        if set.insert(font) {
          fonts.push(font);
        }
      }
      font_db.borrow_mut().set_default_fonts(fonts);
    }
    if let Some(mut l) = ctx.single_child_layouter() {
      l.perform_widget_layout(clamp)
    } else {
      Size::zero()
    }
  }

  #[inline]
  fn paint(&self, _: &mut PaintingCtx) {}
}

impl Default for Theme {
  fn default() -> Self { Theme::Full(<_>::default()) }
}

impl Default for FullTheme {
  fn default() -> Self {
    let icon_size = IconSize {
      tiny: Size::new(18., 18.),
      small: Size::new(24., 24.),
      medium: Size::new(36., 36.),
      large: Size::new(48., 48.),
      huge: Size::new(64., 64.),
    };

    let regular_family = Box::new([FontFamily::Name(std::borrow::Cow::Borrowed("Lato"))]);
    let medium_family = Box::new([FontFamily::Name(std::borrow::Cow::Borrowed("Lato"))]);

    let typography_theme = typography_theme(
      regular_family,
      medium_family,
      TextDecoration::NONE,
      Color::BLACK.with_alpha(0.87).into(),
    );

    let mut icon_theme = IconTheme::new(icon_size);
    fill_svgs! {
      icon_theme,
      svgs::ADD: "./theme/icons/add_FILL0_wght400_GRAD0_opsz48.svg",
      svgs::ARROW_BACK: "./theme/icons/arrow_back_FILL0_wght400_GRAD0_opsz48.svg",
      svgs::ARROW_DROP_DOWN: "./theme/icons/arrow_drop_down_FILL0_wght400_GRAD0_opsz48.svg",
      svgs::ARROW_FORWARD: "./theme/icons/arrow_forward_FILL0_wght400_GRAD0_opsz48.svg",
      svgs::CANCEL: "./theme/icons/cancel_FILL0_wght400_GRAD0_opsz48.svg",
      svgs::CHECK_BOX: "./theme/icons/check_box_FILL0_wght400_GRAD0_opsz48.svg",
      svgs::CHECK_BOX_OUTLINE_BLANK: "./theme/icons/check_box_outline_blank_FILL0_wght400_GRAD0_opsz48.svg",
      svgs::CHEVRON_RIGHT: "./theme/icons/chevron_right_FILL0_wght400_GRAD0_opsz48.svg",
      svgs::CLOSE: "./theme/icons/close_FILL0_wght400_GRAD0_opsz48.svg",
      svgs::DELETE: "./theme/icons/delete_FILL0_wght400_GRAD0_opsz48.svg",
      svgs::DONE: "./theme/icons/done_FILL0_wght400_GRAD0_opsz48.svg",
      svgs::EXPAND_MORE: "./theme/icons/expand_more_FILL0_wght400_GRAD0_opsz48.svg",
      svgs::FAVORITE: "./theme/icons/favorite_FILL0_wght400_GRAD0_opsz48.svg",
      svgs::HOME: "./theme/icons/home_FILL0_wght400_GRAD0_opsz48.svg",
      svgs::INDETERMINATE_CHECK_BOX: "./theme/icons/indeterminate_check_box_FILL0_wght400_GRAD0_opsz48.svg",
      svgs::LOGIN: "./theme/icons/login_FILL0_wght400_GRAD0_opsz48.svg",
      svgs::LOGOUT: "./theme/icons/logout_FILL0_wght400_GRAD0_opsz48.svg",
      svgs::MENU: "./theme/icons/menu_FILL0_wght400_GRAD0_opsz48.svg",
      svgs::MORE_HORIZ: "./theme/icons/more_horiz_FILL0_wght400_GRAD0_opsz48.svg",
      svgs::MORE_VERT: "./theme/icons/more_vert_FILL0_wght400_GRAD0_opsz48.svg",
      svgs::OPEN_IN_NEW: "./theme/icons/open_in_new_FILL0_wght400_GRAD0_opsz48.svg",
      svgs::SEARCH: "./theme/icons/search_FILL0_wght400_GRAD0_opsz48.svg",
      svgs::SETTINGS: "./theme/icons/settings_FILL0_wght400_GRAD0_opsz48.svg",
      svgs::STAR: "./theme/icons/star_FILL0_wght400_GRAD0_opsz48.svg",
      svgs::TEXT_CARET: "./theme/icons/text_caret.svg"
    };

    FullTheme {
      palette: Default::default(),
      typography_theme,
      icon_theme,
      transitions_theme: Default::default(),
      compose_decorators: Default::default(),
      custom_styles: Default::default(),
      font_bytes: None,
      font_files: None,
    }
  }
}

fn typography_theme(
  regular_family: Box<[FontFamily]>,
  medium_family: Box<[FontFamily]>,
  decoration: TextDecoration,
  decoration_color: Brush,
) -> TypographyTheme {
  let decoration = TextDecorationStyle { decoration, decoration_color };
  let regular_face = FontFace {
    families: regular_family.clone(),
    weight: FontWeight::NORMAL,
    ..<_>::default()
  };
  let medium_face = FontFace {
    families: medium_family,
    weight: FontWeight::MEDIUM,
    ..<_>::default()
  };

  TypographyTheme {
    default_font_family: regular_family,
    display_large: TextTheme {
      text: CowArc::owned(TextStyle {
        line_height: Some(Pixel(64.0.into()).into()),
        font_size: FontSize::Pixel(57.0.into()),
        letter_space: Some(Pixel(0.0.into())),
        font_face: regular_face.clone(),
      }),
      decoration: decoration.clone(),
    },
    display_medium: TextTheme {
      text: CowArc::owned(TextStyle {
        line_height: Some(Pixel(52.0.into()).into()),
        font_size: FontSize::Pixel(45.0.into()),
        letter_space: Some(Pixel(0.0.into())),
        font_face: regular_face.clone(),
      }),
      decoration: decoration.clone(),
    },
    display_small: TextTheme {
      text: CowArc::owned(TextStyle {
        line_height: Some(Pixel(44.0.into()).into()),
        font_size: FontSize::Pixel(36.0.into()),
        letter_space: Some(Pixel(0.0.into())),
        font_face: regular_face.clone(),
      }),
      decoration: decoration.clone(),
    },
    headline_large: TextTheme {
      text: CowArc::owned(TextStyle {
        line_height: Some(Pixel(40.0.into()).into()),
        font_size: FontSize::Pixel(32.0.into()),
        letter_space: Some(Pixel(0.0.into())),
        font_face: regular_face.clone(),
      }),
      decoration: decoration.clone(),
    },
    headline_medium: TextTheme {
      text: CowArc::owned(TextStyle {
        line_height: Some(Pixel(36.0.into()).into()),
        font_size: FontSize::Pixel(28.0.into()),
        letter_space: Some(Pixel(0.0.into())),
        font_face: regular_face.clone(),
      }),
      decoration: decoration.clone(),
    },
    headline_small: TextTheme {
      text: CowArc::owned(TextStyle {
        line_height: Some(Pixel(32.0.into()).into()),
        font_size: FontSize::Pixel(24.0.into()),
        letter_space: Some(Pixel(0.0.into())),
        font_face: regular_face.clone(),
      }),
      decoration: decoration.clone(),
    },
    title_large: TextTheme {
      text: CowArc::owned(TextStyle {
        line_height: Some(Pixel(28.0.into()).into()),
        font_size: FontSize::Pixel(22.0.into()),
        letter_space: Some(Pixel(0.0.into())),
        font_face: medium_face.clone(),
      }),
      decoration: decoration.clone(),
    },
    title_medium: TextTheme {
      text: CowArc::owned(TextStyle {
        line_height: Some(Pixel(24.0.into()).into()),
        font_size: FontSize::Pixel(16.0.into()),
        letter_space: Some(Pixel(0.15.into())),
        font_face: medium_face.clone(),
      }),
      decoration: decoration.clone(),
    },
    title_small: TextTheme {
      text: CowArc::owned(TextStyle {
        line_height: Some(Pixel(20.0.into()).into()),
        font_size: FontSize::Pixel(14.0.into()),
        letter_space: Some(Pixel(0.1.into())),
        font_face: medium_face.clone(),
      }),
      decoration: decoration.clone(),
    },
    label_large: TextTheme {
      text: CowArc::owned(TextStyle {
        line_height: Some(Pixel(20.0.into()).into()),
        font_size: FontSize::Pixel(14.0.into()),
        letter_space: Some(Pixel(0.1.into())),
        font_face: medium_face.clone(),
      }),
      decoration: decoration.clone(),
    },
    label_medium: TextTheme {
      text: CowArc::owned(TextStyle {
        line_height: Some(Pixel(16.0.into()).into()),
        font_size: FontSize::Pixel(12.0.into()),
        letter_space: Some(Pixel(0.5.into())),
        font_face: medium_face.clone(),
      }),
      decoration: decoration.clone(),
    },
    label_small: TextTheme {
      text: CowArc::owned(TextStyle {
        line_height: Some(Pixel(16.0.into()).into()),
        font_size: FontSize::Pixel(11.0.into()),
        letter_space: Some(Pixel(0.5.into())),
        font_face: medium_face,
      }),
      decoration: decoration.clone(),
    },
    body_large: TextTheme {
      text: CowArc::owned(TextStyle {
        line_height: Some(Pixel(24.0.into()).into()),
        font_size: FontSize::Pixel(16.0.into()),
        letter_space: Some(Pixel(0.5.into())),
        font_face: regular_face.clone(),
      }),
      decoration: decoration.clone(),
    },
    body_medium: TextTheme {
      text: CowArc::owned(TextStyle {
        line_height: Some(Pixel(20.0.into()).into()),
        font_size: FontSize::Pixel(14.0.into()),
        letter_space: Some(Pixel(0.25.into())),
        font_face: regular_face.clone(),
      }),
      decoration: decoration.clone(),
    },
    body_small: TextTheme {
      text: CowArc::owned(TextStyle {
        line_height: Some(Pixel(16.0.into()).into()),
        font_size: FontSize::Pixel(12.0.into()),
        letter_space: Some(Pixel(0.4.into())),
        font_face: regular_face,
      }),
      decoration,
    },
  }
}

impl From<FullTheme> for Theme {
  #[inline]
  fn from(value: FullTheme) -> Self { Theme::Full(value) }
}

impl From<InheritTheme> for Theme {
  #[inline]
  fn from(value: InheritTheme) -> Self { Theme::Inherit(value) }
}

#[cfg(test)]
mod tests {
  use std::cell::RefCell;

  use super::*;
  use crate::{reset_test_env, test_helper::*};

  #[derive(Query, Declare)]
  struct QueryFont {
    ids: Rc<RefCell<Vec<ID>>>,
  }

  impl Render for QueryFont {
    fn perform_layout(&self, _: BoxClamp, _: &mut LayoutCtx) -> Size {
      *self.ids.borrow_mut() = AppCtx::font_db().borrow().default_fonts().to_vec();
      Size::zero()
    }
    fn paint(&self, _: &mut PaintingCtx) {}
  }
  #[test]
  fn theme_font() {
    reset_test_env!();

    let font_db = AppCtx::font_db().clone();
    let font_ids = Rc::new(RefCell::new(vec![]));
    let font_ids2 = font_ids.clone();
    let w = fn_widget! {
      let mut typography_theme = TypographyTheme::of(ctx!()).clone();
      typography_theme.default_font_family = Box::new([FontFamily::Name("DejaVu Sans".into())]);
      @ThemeWidget {
        theme: Sc::new(Theme::Inherit(InheritTheme {
          typography_theme: Some(typography_theme),
          font_files: Some(vec![env!("CARGO_MANIFEST_DIR").to_owned() + "/../fonts/DejaVuSans.ttf"]),
          ..<_>::default()
        })),
        @{
          Box::new(fn_widget!{
            @QueryFont {
              ids: font_ids2.clone(),
            }
         })
        }
      }
    };

    let old_path = font_db.borrow().default_fonts().to_vec();
    let mut wnd = TestWindow::new(w);
    wnd.draw_frame();

    let ids = font_db.borrow_mut().select_all_match(&FontFace {
      families: Box::new([
        FontFamily::Name("DejaVu Sans".into()),
        FontFamily::Name("Lato".into()),
      ]),
      ..<_>::default()
    });

    assert_eq!(*font_ids.borrow(), ids);
    assert_eq!(&old_path, font_db.borrow().default_fonts());
  }
}
