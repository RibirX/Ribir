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
  fn into_widget(self) -> Widget<'static> {
    match self {
      Self::Hidden => void! {}.into_widget(),
      Self::Dot => text! { text: "", class: BADGE_SMALL }.into_widget(),
      Self::Text(text) => text! { text, class: BADGE_LARGE }.into_widget(),
    }
  }
}

impl<T> From<T> for BadgeContent
where
  CowArc<str>: From<T>,
{
  fn from(value: T) -> Self { Self::Text(CowArc::from(value)) }
}

/// The `Badge` widget is used to show compact notifications, counts, or status
/// information on top of another widget.
///
/// A `Badge` is an overlay-first component:
///
/// - it attaches to a host-relative corner point instead of occupying normal
///   layout space
/// - it may visually overflow the host in a controlled way
/// - it is intended for compact content only: dot indicators, numeric counts,
///   or very short text
///
/// If the content grows into a longer label, chip, or tag, it should usually
/// be modeled as a different component instead of an overlaid badge.
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
  /// The host-relative attachment point for the badge's visual center.
  ///
  /// - If not set explicitly, the badge attaches to the host's top-right
  ///   corner.
  /// - If only one axis is provided, the other axis still follows the default
  ///   top-right behavior.
  /// - The attachment point is then shifted outward by half of the badge's
  ///   resolved size so the badge straddles that point.
  ///
  /// This keeps the corner attachment visually stable as badge content grows,
  /// instead of pinning the badge box strictly inside the host bounds.
  #[declare(default = Anchor::right_top(0., 0.))]
  pub offset: Anchor,
}

/// The `NumBadge` widget is a specialized badge for displaying numeric counts.
///
/// `NumBadge` is optimized for compact overlay counts. Values larger than
/// `max_count` are formatted as `{max_count}+` so the badge remains short and
/// legible on small hosts.
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
  /// The maximum number to display before truncating with a `+`.
  /// Defaults to 999.
  #[declare(default = 999u32)]
  pub max_count: u32,
  /// The host-relative attachment point for the badge.
  ///
  /// See [`Badge::offset`] for the exact semantics.
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
      @InParentLayout {
        @CustomAnchor {
          data: pipe!($read(this).offset.clone()),
          anchor: badge_layout_anchor,
          @ { pipe!($read(this).content.clone()).map(BadgeContent::into_widget) }
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

/// Resolves badge placement from a host-relative attachment point.
///
/// The incoming `offset` describes where the badge should attach on the host;
/// this helper resolves the final badge box origin directly from the host
/// clamp, then shifts by half of the resolved badge size so the badge center
/// straddles that point. Missing axes keep the default top-right attachment
/// behavior.
///
/// We compute an explicit left/top point here because `CustomAnchor` sits
/// under an `InParentLayout` wrapper whose own box collapses to the badge's
/// size, so relying on end-aligned parent-relative anchors would resolve
/// against the wrapper instead of the host.
fn badge_layout_anchor(
  offset: &Anchor, badge_size: Size, clamp: BoxClamp, _ctx: &mut PlaceCtx,
) -> Anchor {
  let x = offset.x.clone().unwrap_or_else(AnchorX::right);
  let y = offset.y.clone().unwrap_or_else(AnchorY::top);
  let host_size = clamp.max;
  let attachment = Point::new(x.calculate(host_size.width, 0.), y.calculate(host_size.height, 0.));

  Anchor::from_point(Point::new(
    attachment.x - badge_size.width / 2.,
    attachment.y - badge_size.height / 2.,
  ))
}

#[cfg(test)]
mod tests {
  use ribir_core::test_helper::*;
  use ribir_dev_helper::*;
  use ribir_material as material;

  use super::*;
  #[allow(unused_imports)]
  use crate::prelude::*;

  #[test]
  fn badge_partial_offset_keeps_default_top_right_axes() {
    reset_test_env!();

    let (top_badge_id, w_top_badge_id) = split_value(None::<WidgetId>);
    let (right_badge_id, w_right_badge_id) = split_value(None::<WidgetId>);
    let wnd = TestWindow::from_widget(fn_widget! {
      @Column {
        @Stack {
          @MockBox { size: Size::new(24., 24.) }
          @InParentLayout {
            @CustomAnchor {
              data: Anchor::top(4.),
              anchor: badge_layout_anchor,
              @MockBox {
                size: Size::new(24., 16.),
                on_mounted: move |e| *$write(w_top_badge_id) = Some(e.current_target()),
              }
            }
          }
        }
        @Stack {
          @MockBox { size: Size::new(24., 24.) }
          @InParentLayout {
            @CustomAnchor {
              data: Anchor::right(-4.),
              anchor: badge_layout_anchor,
              @MockBox {
                size: Size::new(24., 16.),
                on_mounted: move |e| *$write(w_right_badge_id) = Some(e.current_target()),
              }
            }
          }
        }
      }
    });

    wnd.draw_frame();

    let top_badge_id = top_badge_id
      .read()
      .expect("top-only badge should mount for layout assertions");
    let right_badge_id = right_badge_id
      .read()
      .expect("right-only badge should mount for layout assertions");

    assert_eq!(wnd.widget_pos(top_badge_id), Some(Point::new(12., -4.)));
    assert_eq!(wnd.widget_pos(right_badge_id), Some(Point::new(16., -8.)));
  }

  #[test]
  fn badge_layout_can_place_badge_outside_host() {
    reset_test_env!();

    let (badge_id, w_badge_id) = split_value(None::<WidgetId>);
    let wnd = TestWindow::from_widget(fn_widget! {
      @Stack {
        @MockBox { size: Size::new(24., 24.) }
        @InParentLayout {
          @CustomAnchor {
            data: Anchor::right_top(0., 0.),
            anchor: badge_layout_anchor,
            @MockBox {
              size: Size::new(24., 16.),
              on_mounted: move |e| *$write(w_badge_id) = Some(e.current_target()),
            }
          }
        }
      }
    });

    wnd.draw_frame();

    let id = badge_id
      .read()
      .expect("badge widget should mount for layout assertions");
    assert_eq!(wnd.widget_pos(id), Some(Point::new(12., -8.)));
    assert_eq!(
      wnd
        .layout_info_by_path(&[0])
        .unwrap()
        .size
        .unwrap(),
      Size::new(24., 24.)
    );
  }

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

  #[test]
  fn badge_dot_attaches_to_host_top_right() {
    reset_test_env!();
    AppCtx::set_app_theme(material::purple::light());

    let wnd = TestWindow::new_with_size(
      badge! {
        content: BadgeContent::Dot,
        @Container { size: Size::new(40., 40.), background: Color::GRAY }
      },
      Size::new(200., 200.),
    );

    wnd.draw_frame();

    assert_eq!(wnd.layout_info_by_path(&[0, 1, 0]).unwrap().pos, Point::new(37., -3.));
  }

  widget_image_tests!(
    badge,
    WidgetTester::new(self::column! {
      x: AnchorX::center(),
      y: AnchorY::center(),
      align_items: Align::Center,
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
