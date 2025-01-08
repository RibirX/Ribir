//! Theme use to share visual config or style compose logic. It can be defined
//! to app-wide or particular part of the application.

use std::collections::HashMap;

pub use ribir_algo::{CowArc, Resource};
use smallvec::{SmallVec, smallvec};

use crate::prelude::*;

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

/// The `Theme` widget serves to distribute design settings to its
/// subsequent elements.
///
/// Access it through `Theme::of`, and utilize `Theme::write_of` to obtain
/// a writable reference to the theme for modifications.
///
/// Certain components of the theme that are frequently used, such as `Palette`,
/// `TextStyle`, `IconFont`, `Color`, and `ContainerColor`, are also provide
/// as read-only providers when the `Theme` widget compose. If you want to
/// customize specific aspects of the theme, utilize `Providers` to overwrite
/// elements like `Palette`, `TextStyle`, and more.
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
/// You can use an other theme for a widget.
///
/// ```
/// use ribir::prelude::*;
///
/// // Feel free to use a different theme here.
/// let w = Theme::default().with_child(fn_widget! {
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
  // The theme requires font bytes.
  pub font_bytes: Vec<Vec<u8>>,
  // The theme requires font files.
  pub font_files: Vec<String>,
  /// This font is used for icons to display text as icons through font
  /// ligatures. It is crucial to ensure that this font is included in either
  /// `font_bytes` or `font_files`.
  ///
  /// Theme makers may not know which icons the application will utilize, making
  /// it challenging to provide a default icon font. Additionally, offering a
  /// vast selection of icons in a single font file can result in a large file
  /// size, which is not ideal for web platforms. Therefore, this configuration
  /// allows the application developer to supply the font file. Certainly, the
  /// icon also works with `SVG` and [`named_svgs`](super::named_svgs).
  pub icon_font: IconFont,
}

#[derive(Clone, Debug, Default)]
pub struct IconFont(pub FontFace);

impl Theme {
  pub fn of(ctx: &impl AsRef<ProviderCtx>) -> QueryRef<Self> { Provider::of(ctx).unwrap() }

  pub fn write_of(ctx: &impl AsRef<ProviderCtx>) -> WriteRef<Self> {
    Provider::write_of(ctx).unwrap()
  }

  pub(crate) fn preprocess_before_compose(
    this: impl StateWriter<Value = Self>, child: GenWidget,
  ) -> (SmallVec<[Provider; 1]>, Widget<'static>) {
    fn load_fonts(theme: &impl StateWriter<Value = Theme>) {
      // Loading fonts does not require regenerating the `Theme` subtree, as this
      // method has already been called within a regenerated subtree.
      let mut t = theme.write();
      t.load_fonts();
      t.forget_modifies();
    }

    load_fonts(&this);
    let container_color = this.map_reader(|t| {
      // Safety Note: In this instance, a copied value of the palette is utilized,
      // which is not the correct method of using `PartData`. However, in this case,
      // it is only a read-only value, and once added to the providers, neither the
      // state reader nor its read reference can be accessed by anyone. Therefore, it
      // is considered safe.
      unsafe { PartData::from_ptr(ContainerColor(t.palette.secondary_container())) }
    });

    let providers = smallvec![
      // The theme provider is designated as writable state,
      // while other components of the theme provider are treated as read-only state.
      Provider::value_of_state(this.clone_writer()),
      Provider::value_of_state(part_reader!(&this.palette.primary)),
      Provider::value_of_state(container_color),
      Provider::value_of_state(part_reader!(&this.typography_theme.body_medium.text)),
      Provider::value_of_state(part_reader!(&this.palette)),
      Provider::value_of_state(part_reader!(&this.typography_theme)),
      Provider::value_of_state(part_reader!(&this.icon_theme)),
      Classes::reader_into_provider(part_reader!(&this.classes)),
      Provider::value_of_state(part_reader!(&this.transitions_theme)),
      Provider::value_of_state(part_reader!(&this.compose_decorators)),
      Provider::value_of_state(part_reader!(&this.custom_styles)),
      Provider::value_of_state(part_reader!(&this.icon_font))
    ];
    let child = pipe!($this;)
      .map(move |_| {
        load_fonts(&this);
        child.gen_widget()
      })
      .into_widget();
    (providers, child)
  }

  /// Loads the fonts specified in the theme configuration.
  fn load_fonts(&mut self) {
    let mut font_db = AppCtx::font_db().borrow_mut();
    let Theme { font_bytes, font_files, .. } = self;

    font_bytes
      .drain(..)
      .for_each(|data| font_db.load_from_bytes(data.clone()));

    font_files.drain(..).for_each(|path| {
      let _ = font_db.load_font_file(path);
    });
  }
}

impl ComposeChild<'static> for Theme {
  /// The child should be a `GenWidget` so that when the theme is modified, we
  /// can regenerate its sub-tree.
  type Child = GenWidget;
  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'static> {
    use crate::prelude::*;

    let (providers, child) = Theme::preprocess_before_compose(this, child);
    Providers::new(providers).with_child(child)
  }
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

    let icon_theme = IconTheme::new(icon_size);

    Theme {
      palette: Palette::default(),
      typography_theme: typography_theme(),
      icon_theme,
      classes: <_>::default(),
      transitions_theme: Default::default(),
      compose_decorators: Default::default(),
      custom_styles: Default::default(),
      font_bytes: vec![],
      font_files: vec![],
      icon_font: Default::default(),
    }
  }
}

fn typography_theme() -> TypographyTheme {
  fn text_theme(line_height: f32, font_size: f32, letter_space: f32) -> TextTheme {
    let font_face = FontFace {
      families: Box::new([FontFamily::Name(std::borrow::Cow::Borrowed("Lato")), FontFamily::Serif]),
      weight: FontWeight::NORMAL,
      ..<_>::default()
    };
    let overflow = TextOverflow::Clip;
    TextTheme {
      text: TextStyle { line_height, font_size, letter_space, font_face, overflow },
      decoration: TextDecorationStyle {
        decoration: TextDecoration::NONE,
        decoration_color: Color::BLACK.with_alpha(0.87).into(),
      },
    }
  }

  TypographyTheme {
    display_large: text_theme(64., 57., 0.),
    display_medium: text_theme(52., 45., 0.),
    display_small: text_theme(44., 36., 0.),
    headline_large: text_theme(40., 32., 0.),
    headline_medium: text_theme(36., 28., 0.),
    headline_small: text_theme(32., 24., 0.),
    title_large: text_theme(28., 22., 0.),
    title_medium: text_theme(24., 16., 0.15),
    title_small: text_theme(20., 14., 0.1),
    label_large: text_theme(20., 14., 0.1),
    label_medium: text_theme(16., 12., 0.5),
    label_small: text_theme(16., 11., 0.5),
    body_large: text_theme(24., 16., 0.5),
    body_medium: text_theme(20., 14., 0.25),
    body_small: text_theme(16., 12., 0.4),
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
        $writer.write().push(Palette::of(BuildCtx::get()).brightness);
        let writer = writer.clone_writer();
        let palette = Palette { brightness: Brightness::Dark, ..Default::default() };
        @Providers {
          providers: [Provider::new(palette)],
          @  {
            $writer.write().push(Palette::of(BuildCtx::get()).brightness);
            let writer = writer.clone_writer();
            let palette = Palette { brightness: Brightness::Light, ..Default::default() };
            @Providers {
              providers: [Provider::new(palette)],
              @ {
                $writer.write().push(Palette::of(BuildCtx::get()).brightness);
                Void
              }
            }
          }
        }
      })
    };

    let mut wnd = TestWindow::new(w);
    wnd.draw_frame();

    assert_eq!(*watcher.read(), [Brightness::Light, Brightness::Dark, Brightness::Light]);
  }
}
