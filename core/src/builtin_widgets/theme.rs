//! Theme use to share visual config or style compose logic. It can be defined
//! to app-wide or particular part of the application.

use std::collections::HashMap;

pub use ribir_algo::{CowArc, Resource};
use smallvec::SmallVec;

use crate::{fill_svgs, prelude::*};

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

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Brightness {
  Dark,
  Light,
}

/// A `Theme` widget is used to share design configurations among its
/// descendants.
///
/// This includes palettes, font styles, animation transitions, and icons. An
/// app theme is always present, but you can also use a different
/// theme for parts of your sub-tree. You can customize parts of the theme using
/// `Palette`, `TypographyTheme`, and `IconTheme`.
///
/// # Examples
///
/// Every descendant widget of the theme can query it or its parts.
///
/// ```no_run
/// use ribir::prelude::*;
///
/// let w = fn_widget! {
///   @Text {
///     on_tap: |e| {
///       // Query the `Palette` of the application theme.
///       let mut p = Palette::write_of(e);
///        if p.brightness == Brightness::Light {
///           p.brightness = Brightness::Dark;
///        } else {
///           p.brightness = Brightness::Light;
///        }
///     },
///     text : "Click me!"
///   }
/// };
///
/// App::run(w);
/// ```
///
/// You can provide a theme for a widget:
///
/// ```
/// use ribir::prelude::*;
///
/// let w = Theme::default().with_child(fn_widget! {
///   // Feel free to use a different theme here.
///   Void
/// });
/// ```
///
/// # Todo
///
/// Simplify the theme by eliminating the need for `TransitionTheme`,
// `CustomStyles`, and `ComposeDecorators` if we can find a more elegant way to
// handle widget theme styles.
pub struct Theme {
  pub palette: Palette,
  pub typography_theme: TypographyTheme,
  pub classes: Classes,
  pub icon_theme: IconTheme,
  pub transitions_theme: TransitionTheme,
  pub compose_decorators: ComposeDecorators,
  pub custom_styles: CustomStyles,
  pub font_bytes: Option<Vec<Vec<u8>>>,
  pub font_files: Option<Vec<String>>,
}

impl Theme {
  /// Retrieve the nearest `Theme` from the context among its ancestors
  pub fn of(ctx: &impl ProviderCtx) -> QueryRef<Theme> {
    // At least one application theme exists
    Provider::of(ctx).unwrap()
  }

  /// Retrieve the nearest `Theme` from the context among its ancestors and
  /// return a write reference to the theme.
  pub fn write_of(ctx: &impl ProviderCtx) -> WriteRef<Theme> {
    // At least one application theme exists
    Provider::write_of(ctx).unwrap()
  }

  /// Loads the fonts specified in the theme configuration.
  fn load_fonts(&mut self) {
    let mut font_db = AppCtx::font_db().borrow_mut();
    let Theme { font_bytes, font_files, .. } = self;
    if let Some(font_bytes) = font_bytes {
      font_bytes
        .iter()
        .for_each(|data| font_db.load_from_bytes(data.clone()));
    }
    if let Some(font_files) = font_files {
      font_files.iter().for_each(|path| {
        let _ = font_db.load_font_file(path);
      });
    }
  }
}

impl ComposeChild<'static> for Theme {
  /// The child should be a `GenWidget` so that when the theme is modified, we
  /// can regenerate its sub-tree.
  type Child = GenWidget;
  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'static> {
    use crate::prelude::*;

    this.silent().load_fonts();
    let w = this.clone_watcher();
    let theme = ThemeQuerier(this.clone_writer());

    Provider::new(Box::new(theme))
      .with_child(fn_widget! {
        pipe!($w;).map(move |_| child.gen_widget())
      })
      .into_widget()
  }
}

struct ThemeQuerier<T: StateWriter<Value = Theme>>(T);

impl<T: StateWriter<Value = Theme>> Query for ThemeQuerier<T> {
  fn query_all<'q>(&'q self, type_id: TypeId, out: &mut SmallVec<[QueryHandle<'q>; 1]>) {
    // The value of the writer and the writer itself cannot be queried
    // at the same time.
    if let Some(h) = self.query(type_id) {
      out.push(h)
    }
  }

  fn query(&self, type_id: TypeId) -> Option<QueryHandle> {
    ReadRef::filter_map(self.0.read(), |v: &Theme| {
      let w: Option<&dyn Any> = if TypeId::of::<Theme>() == type_id {
        Some(v)
      } else if TypeId::of::<Palette>() == type_id {
        Some(&v.palette)
      } else if TypeId::of::<TypographyTheme>() == type_id {
        Some(&v.typography_theme)
      } else if TypeId::of::<TextStyle>() == type_id {
        Some(&v.typography_theme.body_medium.text)
      } else if TypeId::of::<Classes>() == type_id {
        Some(&v.classes)
      } else if TypeId::of::<IconTheme>() == type_id {
        Some(&v.icon_theme)
      } else if TypeId::of::<TransitionTheme>() == type_id {
        Some(&v.transitions_theme)
      } else if TypeId::of::<ComposeDecorators>() == type_id {
        Some(&v.compose_decorators)
      } else if TypeId::of::<CustomStyles>() == type_id {
        Some(&v.custom_styles)
      } else {
        None
      };
      w.map(PartData::from_ref)
    })
    .ok()
    .map(QueryHandle::from_read_ref)
  }

  fn query_write(&self, type_id: TypeId) -> Option<QueryHandle> {
    WriteRef::filter_map(self.0.write(), |v: &mut Theme| {
      let w: Option<&mut dyn Any> = if TypeId::of::<Theme>() == type_id {
        Some(v)
      } else if TypeId::of::<Palette>() == type_id {
        Some(&mut v.palette)
      } else if TypeId::of::<TypographyTheme>() == type_id {
        Some(&mut v.typography_theme)
      } else if TypeId::of::<IconTheme>() == type_id {
        Some(&mut v.icon_theme)
      } else if TypeId::of::<TransitionTheme>() == type_id {
        Some(&mut v.transitions_theme)
      } else if TypeId::of::<ComposeDecorators>() == type_id {
        Some(&mut v.compose_decorators)
      } else if TypeId::of::<CustomStyles>() == type_id {
        Some(&mut v.custom_styles)
      } else {
        None
      };
      w.map(PartData::from_ref_mut)
    })
    .ok()
    .map(QueryHandle::from_write_ref)
  }

  fn queryable(&self) -> bool { true }
}

impl Default for Theme {
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
      palette: Default::default(),
      typography_theme,
      classes: <_>::default(),
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
  regular_family: Box<[FontFamily]>, medium_family: Box<[FontFamily]>, decoration: TextDecoration,
  decoration_color: Brush,
) -> TypographyTheme {
  let decoration = TextDecorationStyle { decoration, decoration_color };
  let regular_face =
    FontFace { families: regular_family.clone(), weight: FontWeight::NORMAL, ..<_>::default() };
  let medium_face =
    FontFace { families: medium_family, weight: FontWeight::MEDIUM, ..<_>::default() };

  TypographyTheme {
    display_large: TextTheme {
      text: TextStyle {
        line_height: 64.,
        font_size: 57.,
        letter_space: 0.,
        font_face: regular_face.clone(),
        overflow: Overflow::Clip,
      },
      decoration: decoration.clone(),
    },
    display_medium: TextTheme {
      text: TextStyle {
        line_height: 52.,
        font_size: 45.,
        letter_space: 0.,
        font_face: regular_face.clone(),
        overflow: Overflow::Clip,
      },
      decoration: decoration.clone(),
    },
    display_small: TextTheme {
      text: TextStyle {
        line_height: 44.,
        font_size: 36.,
        letter_space: 0.0,
        font_face: regular_face.clone(),
        overflow: Overflow::Clip,
      },
      decoration: decoration.clone(),
    },
    headline_large: TextTheme {
      text: TextStyle {
        line_height: 40.,
        font_size: 32.,
        letter_space: 0.0,
        font_face: regular_face.clone(),
        overflow: Overflow::Clip,
      },
      decoration: decoration.clone(),
    },
    headline_medium: TextTheme {
      text: TextStyle {
        line_height: 36.,
        font_size: 28.,
        letter_space: 0.,
        font_face: regular_face.clone(),
        overflow: Overflow::Clip,
      },
      decoration: decoration.clone(),
    },
    headline_small: TextTheme {
      text: TextStyle {
        line_height: 32.,
        font_size: 24.,
        letter_space: 0.0,
        font_face: regular_face.clone(),
        overflow: Overflow::Clip,
      },
      decoration: decoration.clone(),
    },
    title_large: TextTheme {
      text: TextStyle {
        line_height: 28.,
        font_size: 22.,
        letter_space: 0.0,
        font_face: medium_face.clone(),
        overflow: Overflow::Clip,
      },
      decoration: decoration.clone(),
    },
    title_medium: TextTheme {
      text: TextStyle {
        line_height: 24.,
        font_size: 16.,
        letter_space: 0.15,
        font_face: medium_face.clone(),
        overflow: Overflow::Clip,
      },
      decoration: decoration.clone(),
    },
    title_small: TextTheme {
      text: TextStyle {
        line_height: 20.,
        font_size: 14.,
        letter_space: 0.1,
        font_face: medium_face.clone(),
        overflow: Overflow::Clip,
      },
      decoration: decoration.clone(),
    },
    label_large: TextTheme {
      text: TextStyle {
        line_height: 20.0,
        font_size: 14.0,
        letter_space: 0.1,
        font_face: medium_face.clone(),
        overflow: Overflow::Clip,
      },
      decoration: decoration.clone(),
    },
    label_medium: TextTheme {
      text: TextStyle {
        line_height: 16.,
        font_size: 12.,
        letter_space: 0.5,
        font_face: medium_face.clone(),
        overflow: Overflow::Clip,
      },
      decoration: decoration.clone(),
    },
    label_small: TextTheme {
      text: TextStyle {
        line_height: 16.0,
        font_size: 11.,
        letter_space: 0.5,
        font_face: medium_face,
        overflow: Overflow::Clip,
      },
      decoration: decoration.clone(),
    },
    body_large: TextTheme {
      text: TextStyle {
        line_height: 24.0,
        font_size: 16.0,
        letter_space: 0.5,
        font_face: regular_face.clone(),
        overflow: Overflow::Clip,
      },
      decoration: decoration.clone(),
    },
    body_medium: TextTheme {
      text: TextStyle {
        line_height: 20.0,
        font_size: 14.,
        letter_space: 0.25,
        font_face: regular_face.clone(),
        overflow: Overflow::Clip,
      },
      decoration: decoration.clone(),
    },
    body_small: TextTheme {
      text: TextStyle {
        line_height: 16.,
        font_size: 12.,
        letter_space: 0.4,
        font_face: regular_face,
        overflow: Overflow::Clip,
      },
      decoration,
    },
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{reset_test_env, test_helper::*};

  #[test]
  fn themes() {
    reset_test_env!();

    let (watcher, writer) = split_value(vec![]);

    let w = fn_widget! {
      let writer = writer.clone_writer();
      let mut theme = Theme::default();
      theme.palette.brightness = Brightness::Light;
      theme.with_child(fn_widget! {
        $writer.write().push(Palette::of(ctx!()).brightness);
        let writer = writer.clone_writer();
        Palette { brightness: Brightness::Dark, ..Default::default() }
          .with_child(fn_widget!{
            $writer.write().push(Palette::of(ctx!()).brightness);
            let writer = writer.clone_writer();
            Palette { brightness: Brightness::Light, ..Default::default() }
              .with_child(fn_widget!{
                $writer.write().push(Palette::of(ctx!()).brightness);
                Void
            })
        })
      })
    };

    let mut wnd = TestWindow::new(w);
    wnd.draw_frame();

    assert_eq!(*watcher.read(), [Brightness::Light, Brightness::Dark, Brightness::Light]);
  }
}
