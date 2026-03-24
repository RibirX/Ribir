use ribir_core::{
  class_names,
  prelude::{anchor::Anchor, *},
};

use crate::prelude::*;

/// The `BadgeColor` is used to specify the color of the badge.
#[derive(Clone)]
pub struct BadgeColor(pub Color);

/// Semantic content model for a badge.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum BadgeContent {
  /// Hide the badge entirely.
  #[default]
  Hidden,
  /// Render a small dot badge.
  Dot,
  /// Render text inside the badge.
  Text(CowArc<str>),
}

impl BadgeContent {
  fn visible(&self) -> bool { !matches!(self, Self::Hidden) }

  fn text(&self) -> CowArc<str> {
    match self {
      Self::Text(text) => text.clone(),
      Self::Hidden | Self::Dot => CowArc::default(),
    }
  }

  fn class(&self) -> ClassName {
    match self {
      Self::Text(_) => BADGE_LARGE,
      Self::Hidden | Self::Dot => BADGE_SMALL,
    }
  }
}

impl<T> From<T> for BadgeContent
where
  CowArc<str>: From<T>,
{
  fn from(value: T) -> Self { Self::Text(CowArc::from(value)) }
}

/// The `Badge` widget is used to show notifications, counts, or status
/// information on top of another widget.
///
/// # Example
///
/// ```
/// # use ribir_core::prelude::*;
/// # use ribir_widgets::prelude::*;
///
/// let _badge = badge! {
///   content: "New",
///   @Text { text: "Child widget" }
/// };
///
/// let _dot_badge = badge! {
///   content: BadgeContent::Dot,
///   @Text { text: "Child widget" }
/// };
///
/// let _color_badge = badge! {
///   providers: [Provider::new(BadgeColor(Color::GREEN))],
///   content: "New",
///   @Text { text: "Child widget" }
/// };
/// ```
#[derive(Clone, Declare, PartialEq)]
pub struct Badge {
  /// The content to display inside the badge.
  /// Values accepted by `CowArc<str>` become `BadgeContent::Text(...)`.
  /// - `"text"` / `BadgeContent::Text("text".into())`: Display the text.
  /// - `BadgeContent::Dot`: Display a small dot.
  /// - `BadgeContent::Hidden`: Hide the badge.
  #[declare(default)]
  pub content: BadgeContent,
  /// The offset to adjust the badge's position relative to the bounding
  /// rectangle of the child widget.
  #[declare(default = Anchor::right_top(0., 0.))]
  pub offset: Anchor,
}

/// The `NumBadge` widget is a specialized badge for displaying numeric counts.
///
/// # Example
///
/// ```
/// # use ribir_core::prelude::*;
/// # use ribir_widgets::prelude::*;
///
/// let _num_badge = num_badge! {
///   count: 5,
///   @Text { text: "Child widget" }
/// };
///
/// let _overflow_badge = num_badge! {
///   count: 100,
///   max_count: 99u32,
///   @Text { text: "Child widget" }
/// };
///
/// let _color_badge = num_badge! {
///   providers: [Provider::new(BadgeColor(Color::GREEN))],
///   count: 5,
///   @Text { text: "Child widget" }
/// };
/// ```
#[derive(Clone, Declare, PartialEq)]
pub struct NumBadge {
  /// The number to display.
  /// - `Some(n)`: Display the number `n` (or `max_count+` if `n > max_count`).
  /// - `None`: Hide the badge.
  #[declare(default)]
  pub count: Option<u32>,
  /// The maximum number to display before truncating with a "+".
  /// Defaults to 999.
  #[declare(default = 999u32)]
  pub max_count: u32,
  /// The offset to adjust the badge's position relative to the bounding
  /// rectangle of the child widget.
  #[declare(default = Anchor::right_top(0., 0.))]
  pub offset: Anchor,
}

class_names! {
  /// The class name for the small badge (dot).
  BADGE_SMALL,
  /// The class name for the large badge (with content).
  BADGE_LARGE,
}

impl<'a> ComposeChild<'a> for Badge {
  type Child = Widget<'a>;

  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'a> {
    stack! {
      @ { child }
      @ InParentLayout {
        @Text {
          visible: pipe!($read(this).content.visible()),
          x: pipe!($read(this).offset.x.clone().unwrap_or_default()),
          y: pipe!($read(this).offset.y.clone().unwrap_or_default()),
          text: pipe!($read(this).content.text()),
          class: pipe!($read(this).content.class()),
        }
      }
    }
    .into_widget()
  }
}

impl<'a> ComposeChild<'a> for NumBadge {
  type Child = Widget<'a>;

  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'a> {
    badge! {
      content: pipe!($read(this).count).map(move |v| {
        v.map_or(BadgeContent::Hidden, |count| {
          let max = $read(this).max_count;
          if count > max {
            BadgeContent::Text(format!("{}+", max).into())
          } else {
            BadgeContent::Text(count.to_string().into())
          }
        })
      }),
      offset: pipe!($read(this).offset.clone()),
      @ { child }
    }
    .into_widget()
  }
}

#[cfg(test)]
mod tests {
  use ribir_core::test_helper::*;
  use ribir_dev_helper::*;

  use super::*;

  #[test]
  fn badge_content_shorthand_wraps_some() {
    reset_test_env!();
    let mut badge = Badge::declarer();
    badge.with_content("error!");
    let badge = badge.finish();
    assert_eq!(badge.read().content, BadgeContent::Text("error!".into()));
  }

  #[test]
  fn badge_content_hidden_keeps_hidden() {
    reset_test_env!();
    let mut badge = Badge::declarer();
    badge.with_content(BadgeContent::Hidden);
    let badge = badge.finish();
    assert_eq!(badge.read().content, BadgeContent::Hidden);
  }

  widget_image_tests!(
    badge,
    WidgetTester::new(self::column! {
      @Badge {
        content: BadgeContent::Dot,
        @Container { size: Size::new(40., 40.), background: Color::GRAY }
      }
      @Badge {
        content: "error!",
        offset: Anchor::right(-14.),
        @Container { size: Size::new(40., 40.)}
      }
      @NumBadge {
        count: 1000,
        max_count: 99_u32,
        providers: [Provider::new(BadgeColor(Color::GREEN))],
        @Container { size: Size::new(40., 40.), background: Color::GRAY }
      }
    })
    .with_wnd_size(Size::new(200., 200.))
    .with_comparison(0.0001),
  );
}
