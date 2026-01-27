use ribir_core::{class_names, prelude::*};

use crate::prelude::*;

/// The `BadgeColor` is used to specify the color of the badge.
#[derive(Clone)]
pub struct BadgeColor(pub Color);

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
///   content: Some(CowArc::from("")),
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
  /// - `Some("text")`: Display the text.
  /// - `Some("")`: Display a small dot.
  /// - `None`: Hide the badge.
  #[declare(default)]
  pub content: Option<CowArc<str>>,
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
///   count: Some(5),
///   @Text { text: "Child widget" }
/// };
///
/// let _overflow_badge = num_badge! {
///   count: Some(100),
///   max_count: 99u32,
///   @Text { text: "Child widget" }
/// };
///
/// let _color_badge = num_badge! {
///   providers: [Provider::new(BadgeColor(Color::GREEN))],
///   count: Some(5),
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
          visible: pipe!($read(this).content.is_some()),
          x: pipe!($read(this).offset.x.clone().unwrap_or_default()),
          y: pipe!($read(this).offset.y.clone().unwrap_or_default()),
          text: pipe!($read(this).content.clone().unwrap_or_default()),
          class: pipe! {
            if $read(this).content.as_ref().map_or(true, |s| s.is_empty()) {
              BADGE_SMALL
            } else {
              BADGE_LARGE
            }
          }
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
        v.map(|count| {
            let max = $read(this).max_count;
            if count > max {
            format!("{}+", max).into()
          } else {
            count.to_string().into()
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

  widget_image_tests!(
    badge,
    WidgetTester::new(self::column! {
      @Badge {
        content: Some("".into()),
        @Container { size: Size::new(40., 40.), background: Color::GRAY }
      }
      @Badge {
        content: Some("error!".into()),
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
