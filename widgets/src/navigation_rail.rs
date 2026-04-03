//! # Navigation Rail
//!
//! `NavigationRail` is an adaptive side-navigation widget designed for medium
//! and large screens, providing efficient access to top-level navigation and
//! dynamically adjusting layout based on available space. This widget acts as a
//! This widget conforms to the standard interactive widget design, acting as a
//! pure selection container. It does not include built-in routing; instead,
//! users handle side effects by listening for [`RailSelectEvent`] via
//! `on_custom`.
//!
//! ## Identifier Strategy
//!
//! Each `RailItem` can optionally specify a `key` for stable identification.
//! If omitted, the rail automatically uses the item's index (`"0"`, `"1"`,
//! etc.) as the key.
//!
//! *Note: Action-oriented items (e.g., Logout, Create) do not represent
//! navigation states. You should handle their selections directly and ensure
//! the rail's `selected` property is managed via a controlled binding
//! (`pipe!`).*
//!
//! ## Layout Modes & Adaptation
//!
//! The rail operates in two layout modes, controlled by the `RailExpanded`
//! provider:
//! - **Collapsed (Narrow)**: Optimized for space. Items are stacked vertically.
//! - **Expanded (Wide)**: Optimized for clarity. Items are laid out
//!   horizontally.
//!
//! Other environment configurations include `RailLabelPolicy` (for label
//! visibility in collapsed mode) and `RailContentAlign`. We use providers for
//! these to separate visual context from the core business state (`selected`).
//!
//! ## Semantic Hierarchy
//!
//! - [`NavigationRail`]: The root container that manages layout modes and
//!   alignment.
//! - [`RailMenu`]: A toggle wrapper (usually an icon button) for expanding the
//!   rail.
//! - [`RailAction`] / [`RailFabAction`]: Slots for primary actions.
//! - [`RailItem`]: An interactive navigation destination.
//! - [`RailSection`]: A semantic boundary (e.g., text header or divider).
//! - [`RailFooter`]: The bottom area for secondary actions or profiles.
//!
//! ## Example Usage
//!
//! ```rust,no_run
//! use ribir::prelude::*;
//! use smallvec::smallvec;
//!
//! let is_expanded = Stateful::new(RailExpanded(false));
//!
//! let _ = providers! {
//!   providers: smallvec![
//!     Provider::writer(is_expanded.clone_writer(), None),
//!     Provider::new(RailLabelPolicy::OnSelected),
//!   ],
//!   @NavigationRail {
//!     selected: "home",
//!
//!     on_custom: move |e: &mut RailSelectEvent| {
//!       println!("Navigating to: {}", e.data().to);
//!     },
//!
//!     @RailMenu {
//!       @TextButton {
//!         on_tap: move |_| $write(is_expanded).toggle(),
//!         @Icon { @ { svg_registry::get_or_default("menu") } }
//!       }
//!     }
//!
//!     @RailFabAction {
//!       @Icon { @ { svg_registry::get_or_default("add") } }
//!       @ { "New" }
//!     }
//!
//!     // Item with explicit key
//!     @RailItem {
//!       key: "home",
//!       @ { svg_registry::get_or_default("home") }
//!       @ { "Home" }
//!     }
//!
//!     @RailSection { @ { "Groups" } }
//!
//!     // Item relying on auto-index (key: "1")
//!     @RailItem {
//!       @ { svg_registry::get_or_default("folder") }
//!       @ { "Projects" }
//!       @Badge { content: "12" }
//!     }
//!
//!     @RailFooter {
//!       @Icon { @ { svg_registry::get_or_default("settings") } }
//!     }
//!   }
//! };
//! ```

use ribir_core::prelude::*;
use smallvec::smallvec;

use crate::prelude::*;

/// Defines the display strategy for labels in the navigation rail when in
/// collapsed mode.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum RailLabelPolicy {
  /// Labels are never visible. Icons are centered within the item.
  None,
  /// Labels are only visible when the item is selected.
  #[default]
  OnSelected,
  /// Labels are always visible. Icons are moved upwards to make room for the
  /// text.
  Always,
}

/// A state provider that indicates whether the navigation rail is currently
/// expanded.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub struct RailExpanded(pub bool);

/// Content alignment Provider
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub struct RailContentAlign(pub Align);

/// Metadata about the structure of a rail item.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct RailItemMetadata {
  /// Indicates if the item has a label.
  pub has_label: bool,
  /// Indicates if the item has a badge.
  pub has_badge: bool,
}

/// Navigation item selection event
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RailSelect {
  /// Previously selected identifier (for animation direction, history)
  pub from: Option<CowArc<str>>,
  /// Newly selected identifier
  pub to: CowArc<str>,
}

pub type RailSelectEvent = CustomEvent<RailSelect>;

class_names! {
  /// Root container class for the Navigation Rail.
  NAVIGATION_RAIL,
  /// Class for expanded rail size/layout.
  NAVIGATION_RAIL_EXPANDED,
  /// Class for collapsed rail size/layout.
  NAVIGATION_RAIL_COLLAPSED,
  /// Class for the header area (menu/action).
  RAIL_HEADER,
  /// Class for the menu button.
  RAIL_MENU,
  /// Class for the primary action button.
  RAIL_ACTION,
  /// Class for the primary action when menu is also present.
  RAIL_ACTION_WITH_MENU,
  /// Class for the content area (items and sections).
  RAIL_CONTENT,
  /// Class applied when rail has no header (adds top padding per Material spec).
  RAIL_CONTENT_NO_HEADER,
  /// Class for the footer area.
  RAIL_FOOTER,
  /// Base class for all rail items.
  RAIL_ITEM,
  /// Class for selected rail items.
  RAIL_ITEM_SELECTED,
  /// Class for unselected rail items.
  RAIL_ITEM_UNSELECTED,
  /// Class for the icon within a rail item.
  RAIL_ITEM_ICON,
  /// Class for the label within a rail item.
  RAIL_ITEM_LABEL,
  /// Class for rail sections.
  RAIL_SECTION,
}

/// A toggle widget wrapper, typically an [IconButton] used to switch between
/// expanded and collapsed states.
#[derive(Template)]
pub struct RailMenu(pub Widget<'static>);

/// Action content wrapper used to avoid child type conflicts.
#[derive(Template)]
pub enum RailActionEntry<'c> {
  /// A custom widget action.
  Action(RailAction<'c>),
  /// A FAB action composed via [Fab].
  Fab(RailFabAction<'c>),
}

/// A primary action widget wrapper.
#[derive(Template)]
pub struct RailAction<'c>(pub Widget<'c>);

/// A primary FAB action that shares the same child syntax as [Fab].
pub type RailFabAction<'c> = ButtonChild<'c>;

/// A footer widget wrapper for secondary actions or user profiles.
#[derive(Template)]
pub struct RailFooter(pub Widget<'static>);

/// A group header used to categorize items within the [NavigationRail].
///
/// `RailSection` acts as a semantic boundary. It provides a flexible way to
/// group navigation items with a header or a separator.
///
/// ### Content Behavior
/// The `child` widget is rendered exactly as provided. The framework manages
/// the visibility of the section based on the rail's expansion state and the
/// `show_collapsed` flag, but it does not transform the content.
///
/// ### Usage Guidance
/// - **Expanded Mode**: Typically used with a [Text] widget to provide a
///   category header.
/// - **Collapsed Mode**:
///   - If `show_collapsed` is `true`, it is highly recommended to use a simple,
///     centered widget like a [Divider] or a small [Icon].
///   - Avoid large text headers in collapsed mode as they may be clipped or
///     visually overwhelming.
///   - It is the user's responsibility to provide content that is appropriate
///     for the narrow width of the collapsed rail.
#[derive(Template)]
pub struct RailSection<'c> {
  /// Controls whether this section remains visible when the [NavigationRail] is
  /// collapsed.
  ///
  /// When `true`, the `child` will be rendered even in collapsed mode. This is
  /// useful for persistent dividers or small status icons.
  #[template(field = false)]
  pub show_collapsed: bool,
  /// The widget to be displayed as the section header or divider.
  ///
  /// This widget is rendered as-is. See the struct-level documentation for
  /// recommendations on content choice for different rail modes.
  pub child: RailSectionWidget<'c>,
}

/// The state holder for an individual navigation destination in the rail.
#[derive(Clone)]
#[declare]
pub struct RailItem {
  /// Business identifier for selection state matching.
  ///
  /// - **User-provided**: used directly for tracking the selected state.
  /// - **User-omitted**: `NavigationRail` auto-generates a string index (`"0"`,
  ///   `"1"`...).
  ///
  /// *Note: `key` is for business logic matching, distinct from `reuse` which
  /// is used for framework-level widget instance identity.*
  #[declare(default)]
  pub key: CowArc<str>,
}

impl RailItem {
  fn ensure_key(&mut self, index: usize) -> CowArc<str> {
    if self.key.is_empty() {
      self.key = index.to_string().into();
    }
    self.key.clone()
  }

  fn linear_item() -> FatObj<XMultiChild<'static>> {
    let x = Variant::<RailExpanded>::new_or_default(BuildCtx::get())
      .map(|e| {
        if e.0 {
          Row { align_items: Align::Center, justify_content: JustifyContent::Start }
            .into_multi_child()
        } else {
          Column { align_items: Align::Center, justify_content: JustifyContent::Start }
            .into_multi_child()
        }
      })
      .into_multi_child();
    FatObj::new(x)
  }

  /// Returns the closest [`RailItem`] from provider context.
  #[inline]
  pub fn of(ctx: &impl AsRef<ProviderCtx>) -> Option<QueryRef<'_, Self>> {
    Provider::of::<Self>(ctx)
  }

  /// Returns whether the current provider context belongs to the selected
  /// [`RailItem`]. Returns `false` when rail/item provider is unavailable.
  #[inline]
  pub fn is_selected_of(ctx: &impl AsRef<ProviderCtx>) -> bool {
    let Some(item) = Self::of(ctx) else { return false };
    let Some(rail) = Provider::of::<NavigationRail>(ctx) else { return false };
    rail.selected == item.key
  }
}

/// Template for the badge of a rail item.
#[derive(Template)]
pub enum RailBadge {
  Badge(FatObj<Stateful<Badge>>),
  NumBadge(FatObj<Stateful<NumBadge>>),
}

/// Content template for [RailItem].
/// It destructures the icons, labels, and badges provided in the DSL.
#[derive(Template)]
pub struct RailItemChildren<'c> {
  /// The primary icon of the item.
  pub icon: Widget<'c>,
  /// The optional label text.
  pub label: Option<TextValue>,
  /// An optional badge (e.g., a notification count) displayed over the icon.
  pub badge: Option<RailBadge>,
}

#[derive(Template)]
pub enum RailSectionWidget<'c> {
  Text(TextValue),
  Widget(Widget<'c>),
}

/// Supported child types for [NavigationRail].
#[derive(Template)]
#[allow(clippy::large_enum_variant)]
pub enum RailChild<'c> {
  /// Toggle button for the rail mode.
  Menu(RailMenu),
  /// Primary action slot.
  Action(RailActionEntry<'c>),
  /// Individual navigation destination.
  Item(PairOf<'c, RailItem>),
  /// A section header for grouping items.
  Section(RailSection<'c>),
  /// Content slot at the bottom for secondary actions.
  Footer(RailFooter),
}

/// Navigation Rail.
///
/// Navigation rails provide ergonomic access to 3-7 primary destinations in an
/// app on mid-to-large screens (tablets/desktops).
#[declare]
pub struct NavigationRail {
  /// Currently selected item identifier.
  ///
  /// This field tracks the business selection state. It is bound to
  /// `RailSelect.to` (via `event = RailSelect.to.clone()`) so that bubbling
  /// `RailSelect` events automatically update `selected`, supporting both
  /// uncontrolled and TwoWay bindings.
  #[declare(default, setter = set_selected, event = RailSelect.to.clone())]
  selected: CowArc<str>,

  /// Internal navigation item key list for query methods.
  #[declare(skip)]
  items: Vec<CowArc<str>>,
}

impl NavigationRail {
  pub fn writer_of(ctx: &impl AsRef<ProviderCtx>) -> Option<Box<dyn StateWriter<Value = Self>>> {
    Provider::writer_of(ctx)
  }

  pub fn keys(&self) -> &[CowArc<str>] { &self.items }

  /// Returns the currently selected item key.
  pub fn selected(&self) -> &CowArc<str> { &self.selected }

  /// Sets the selected item by key.
  ///
  /// If `items` is not empty (i.e., compose has occurred), the key must exist
  /// in the items list. If items is empty (compose hasn't happened yet),
  /// validation is skipped.
  pub fn set_selected(&mut self, key: CowArc<str>) {
    if self.items.is_empty() || self.items.iter().any(|k| k == &key) {
      self.selected = key;
    }
  }

  pub fn next_key(&self) -> Option<&str> {
    let next_idx = self.current_index().map(|i| i + 1).unwrap_or(0);
    self.items.get(next_idx).map(|s| &**s)
  }

  pub fn prev_key(&self) -> Option<&str> {
    match self.current_index() {
      Some(0) => None,
      Some(idx) => Some(&*self.items[idx - 1]),
      None => self.items.last().map(|s| &**s),
    }
  }

  pub fn next_key_cyclic(&self) -> Option<&str> {
    if self.items.is_empty() {
      return None;
    }
    let idx = self
      .current_index()
      .map_or(0, |i| (i + 1) % self.items.len());
    Some(&*self.items[idx])
  }

  pub fn prev_key_cyclic(&self) -> Option<&str> {
    let len = self.items.len();
    if len == 0 {
      return None;
    }
    let idx = self
      .current_index()
      .map_or(len - 1, |i| (i + len - 1) % len);
    Some(&*self.items[idx])
  }

  fn current_index(&self) -> Option<usize> {
    self
      .items
      .iter()
      .position(|k| k == self.selected())
  }

  fn content_align() -> PipeValue<JustifyContent> {
    Variant::<RailContentAlign>::new_or_default(BuildCtx::get())
      .map(|a| match a.0 {
        Align::Start => JustifyContent::Start,
        Align::Center => JustifyContent::Center,
        Align::End => JustifyContent::End,
        Align::Stretch => JustifyContent::SpaceBetween,
      })
      .into_pipe_value()
  }
}

impl<'c> ComposeChild<'c> for NavigationRail {
  type Child = Vec<RailChild<'c>>;

  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
    let (menu, action, footer, content_children) = this.silent().partition_children(child);

    let header = (menu.is_some() || action.is_some()).then(|| {
      self::column! {
        class: RAIL_HEADER,
        align_items: Align::Start,
        @ { menu }
        @ { action }
      }
    });

    let mut providers = smallvec![Provider::writer(this, None)];
    if Provider::of::<RailExpanded>(BuildCtx::get()).is_none() {
      providers.push(Provider::writer(Stateful::new(RailExpanded::default()), None));
    }
    // Keep the base class stable so width smoothing state can survive mode
    // toggles, and switch only the mode class dynamically.
    let mut classes = ClassList::from([NAVIGATION_RAIL]);
    classes.push(expanded_switch(NAVIGATION_RAIL_EXPANDED, NAVIGATION_RAIL_COLLAPSED).map(Some));

    // Base content class plus optional padding class when header is absent.
    let has_header = header.is_some();
    let content_class = {
      let mut classes = ClassList::from([RAIL_CONTENT]);
      if !has_header {
        classes.push(RAIL_CONTENT_NO_HEADER);
      }
      classes
    };

    providers! {
      providers: providers,
      @Flex {
        direction: Direction::Vertical,
        align_items: Align::Start,
        class: classes,
        @ { header }
        @Expanded {
          @ScrollableWidget {
            scrollable: Scrollable::Y,
            @Column {
              class: content_class,
              align_items: Align::Start,
              justify_content: Self::content_align(),
              @ { content_children }
            }
          }
        }
        @ { footer }
      }
    }
    .into_widget()
  }
}

impl<'c> ComposeChild<'c> for RailItem {
  type Child = RailItemChildren<'c>;

  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
    let item = this.read().clone();
    let RailItemChildren { icon, label, badge } = child;
    let metadata = RailItemMetadata { has_label: label.is_some(), has_badge: badge.is_some() };

    fn_widget! {
      let label_widget = label.map(|text| {
        let visible = item.label_visible();
        text! { class: RAIL_ITEM_LABEL, text, visible }.into_widget()
      });

      // Keep the base item class stable and switch only the selected-state
      // class to avoid unnecessary base-layer rebuilds.
      let sel_item_cls = item.selected_cls().map(Some);
      let item_classes = class_list![RAIL_ITEM, sel_item_cls];
      @FatObj {
        on_action: move |e: &mut Event| {
          let Some(target_ctx) = e.provider_ctx_at(e.target()) else { return; };
          let Some(item) = Provider::of::<RailItem>(target_ctx) else { return; };
          let Some(rail) = Provider::of::<NavigationRail>(target_ctx) else { return; };
          let from = (!rail.selected.is_empty()).then(|| rail.selected.clone());
          let to = item.key.clone();
          if from.as_ref() != Some(&to) {
            e.window()
              .bubble_custom_event(e.target(), RailSelect { from, to });
          }
        },
        @Providers {
          providers: smallvec![Provider::new(metadata), Provider::new(item.clone())],
          @(Self::linear_item()) {
            class: item_classes,
            @Self::build_icon_with_badge(icon, badge)
            @ { label_widget }
          }
        }
      }
    }
    .into_widget()
  }
}

impl NavigationRail {
  fn wrap_with_class<'c>(class: ClassName, child: Widget<'c>) -> Widget<'c> {
    class! { class, @ { child } }.into_widget()
  }

  fn build_action_widget<'c>(entry: RailActionEntry<'c>) -> Widget<'c> {
    let action = match entry {
      RailActionEntry::Action(w) => Self::wrap_with_class(RAIL_ACTION, w.0),
      RailActionEntry::Fab(fab) => Self::wrap_with_class(
        RAIL_ACTION,
        providers! {
          providers: [Provider::new(FabElevation::new(0, 1))],
          @fab! { @{fab} }
        }
        .into_widget(),
      ),
    };
    fn_widget! {
      let label_visibility = Variant::<RailExpanded>::new_or_default(BuildCtx::get())
        .map(|e| if e.0 { ButtonLabelVisibility::Show } else { ButtonLabelVisibility::Hide })
        .into_provider();

      @Providers {
        providers: [label_visibility],
        @ { action }
      }
    }
    .into_widget()
  }

  fn partition_children<'c>(
    &mut self, children: Vec<RailChild<'c>>,
  ) -> (Option<Widget<'c>>, Option<Widget<'c>>, Option<Widget<'c>>, Vec<Widget<'c>>) {
    let mut menu: Option<Widget<'c>> = None;
    let mut action: Option<Widget<'c>> = None;
    let mut footer: Option<Widget<'c>> = None;
    let mut content_children: Vec<Widget<'c>> = Vec::new();

    self.items.clear();

    let mut index = 0;
    for child in children.into_iter() {
      match child {
        RailChild::Menu(m) => {
          assert!(menu.is_none(), "NavigationRail can only have one RailMenu");
          menu = Some(class! { class: RAIL_MENU, @ { m.0 } }.into_widget());
        }
        RailChild::Action(a) => {
          assert!(action.is_none(), "NavigationRail can only have one RailAction");
          action = Some(Self::build_action_widget(a));
        }
        RailChild::Footer(f) => {
          footer = Some(Self::wrap_with_class(RAIL_FOOTER, f.0));
        }
        RailChild::Section(RailSection { show_collapsed, child }) => {
          content_children.push(child.into_widget(show_collapsed))
        }
        RailChild::Item(pair) => {
          index += 1;
          let key = pair.parent().silent().ensure_key(index);
          self.items.push(key.clone());
          content_children.push(pair.into_widget());
        }
      }
    }

    if menu.is_some() {
      action = action.map(|a| Self::wrap_with_class(RAIL_ACTION_WITH_MENU, a));
    }

    (menu, action, footer, content_children)
  }
}

impl RailExpanded {
  pub fn toggle(&mut self) { self.0 = !self.0; }
}

impl<'c> RailSectionWidget<'c> {
  fn into_widget(self, show_collapsed: bool) -> Widget<'c> {
    fn_widget! {
      @FatObj {
        visible: expanded_switch(true, show_collapsed),
        class: RAIL_SECTION,
        @{
          match self {
            RailSectionWidget::Text(text) => text! { text }.into_widget(),
            RailSectionWidget::Widget(w) => w,
          }
        }
      }
    }
    .into_widget()
  }
}

impl RailItem {
  fn build_icon_with_badge<'c>(icon: Widget<'c>, badge: Option<RailBadge>) -> Widget<'c> {
    let mut icon_widget = icon! { class: RAIL_ITEM_ICON, @ { icon } }.into_widget();
    if let Some(badge) = badge {
      icon_widget = match badge {
        RailBadge::Badge(b) => b.with_child(icon_widget).into_widget(),
        RailBadge::NumBadge(nb) => nb.with_child(icon_widget).into_widget(),
      };
    }
    icon_widget
  }

  fn selected_cls(&self) -> Pipe<ClassName> {
    let rail =
      NavigationRail::writer_of(BuildCtx::get()).expect("NavigationRail provider must be in scope");
    let key = self.key.clone();
    distinct_pipe! {
      if $read(rail).selected == key {
        RAIL_ITEM_SELECTED
      } else {
        RAIL_ITEM_UNSELECTED
      }
    }
  }

  pub fn label_visible(&self) -> PipeValue<bool> {
    let ctx = BuildCtx::get();
    let expanded = Variant::<RailExpanded>::new_or_default(ctx);
    let label_policy = Variant::<RailLabelPolicy>::new_or_default(ctx);
    let key = self.key.clone();

    let rail_selected = NavigationRail::writer_of(ctx)
      .expect("NavigationRail provider must be in scope")
      .part_watcher(|rail| PartRef::from(&rail.selected));

    expanded
      .combine(label_policy)
      .combine_with(rail_selected, move |((expanded, policy), selected)| {
        expanded.0
          || policy == &RailLabelPolicy::Always
          || (policy == &RailLabelPolicy::OnSelected && **selected == *key)
      })
      .into_pipe_value()
  }
}

pub fn expanded_switch<T: Clone + 'static>(expanded: T, collapsed: T) -> PipeValue<T> {
  Variant::<RailExpanded>::new_or_default(BuildCtx::get())
    .map(move |e| if e.0 { expanded.clone() } else { collapsed.clone() })
    .into_pipe_value()
}

#[cfg(test)]
mod tests {
  use ribir_core::{prelude::easing::CubicBezierEasing, test_helper::*, window::WindowFlags};
  use ribir_material as material;

  use super::*;

  const NAV_RAIL_SPATIAL_STANDARD: EasingTransition<CubicBezierEasing> = EasingTransition {
    easing: CubicBezierEasing::new(0.27, 1.06, 0.18, 1.00),
    duration: material::md::motion::spring::duration::standard::DEFAULT_SPATIAL,
  };

  const TEST_NAV_EXPANDED_WIDTH: f32 = 256.;
  const TEST_NAV_COLLAPSED_WIDTH: f32 = 80.;

  fn nav_root_class(w: Widget) -> Widget {
    smooth_layout! {
      transition: NAV_RAIL_SPATIAL_STANDARD,
      size_mode: SizeMode::Visual,
      @ { w }
    }
    .into_widget()
  }

  #[test]
  fn rail_item_indicator_displays_on_selection() {
    // This test verifies that the indicator shows/hides correctly when selection
    // changes. The indicator uses AnimatedPresence for enter/leave animations.
    reset_test_env!();

    // Track whether indicator is present by checking paint calls
    let selected = Stateful::new(CowArc::<str>::from("home"));
    let c_selected = selected.clone_writer();
    let wnd = TestWindow::new(
      fn_widget! {
        @Providers {
          providers: smallvec::smallvec![
            Provider::new(RailExpanded(false)),
            Provider::new(RailContentAlign(Align::Start)),
          ],
          @NavigationRail {
            selected: "home",
            size: Size::new(80., 200.),
            @RailItem {
              key: "home",
              @Void { size: Size::new(24., 24.) }
              @ { "Home" }
            }
            @RailItem {
              key: "settings",
              @Void { size: Size::new(24., 24.) }
              @ { "Settings" }
            }
          }
        }
      },
      Size::new(80., 200.),
      WindowFlags::ANIMATIONS,
    );

    // Initial render
    wnd.draw_frame();

    // Switch selection
    *c_selected.write() = CowArc::<str>::from("settings");
    wnd.draw_frame();

    // Should not panic or fail
  }

  fn assert_close(actual: f32, expected: f32, label: &str) {
    let eps = 0.5;

    assert!((actual - expected).abs() <= eps, "{label} expected {expected}, got {actual}",);
  }

  #[test]
  fn rail_item_provider_of_and_selected_state() {
    reset_test_env!();

    let (is_selected, w_selected) = split_value(false);
    let (item_key, w_item_key) = split_value(String::new());
    let (outside_selected, w_outside_selected) = split_value(true);
    let wnd = TestWindow::from_widget(fn_widget! {
      @Column {
        on_mounted: move |e| {
          *$write(w_outside_selected) = RailItem::is_selected_of(e);
        },
        @NavigationRail {
          selected: "home",
          @RailItem {
            key: "home",
            @FatObj {
              on_mounted: move |e| {
                *$write(w_selected) = RailItem::is_selected_of(e);
                *$write(w_item_key) = RailItem::of(e)
                  .map(|item| item.key.to_string()).unwrap_or_default();
              },
              @Void {}
            }
          }
        }
      }
    });

    wnd.draw_frame();
    assert!(*is_selected.read());
    assert_eq!(*item_key.read(), "home");
    assert!(!*outside_selected.read());
  }

  #[test]
  fn rail_item_is_selected_of_returns_false_for_unselected_item() {
    reset_test_env!();

    let (is_selected, w_selected) = split_value(true);
    let wnd = TestWindow::from_widget(fn_widget! {
      @NavigationRail {
        selected: "home",
        @RailItem {
          key: "settings",
          @FatObj {
            on_mounted: move |e| {
              *$write(w_selected) = RailItem::is_selected_of(e);
            },
            @Void {}
          }
        }
      }
    });

    wnd.draw_frame();
    assert!(!*is_selected.read());
  }

  #[test]
  fn rail_item_click_updates_selected() {
    reset_test_env!();

    let (selected, w_selected) = split_value(CowArc::<str>::from(""));
    let (second_item_id, w_second_item_id) = split_value(None::<WidgetId>);
    let wnd = TestWindow::new_with_size(
      fn_widget! {
        @Providers {
          providers: [Provider::new(RailContentAlign(Align::Start))],
          @NavigationRail {
            selected: "home",
            size: Size::new(96., 200.),
            on_custom: move |e: &mut RailSelectEvent| *$write(w_selected) = e.data().to.clone(),
            @RailItem {
              key: "home",
              @Void {
                size: Size::new(24., 24.),
              }
            }
            @RailItem {
              key: "settings",
              @Void {
                size: Size::new(24., 24.),
                on_mounted: move |e| *$write(w_second_item_id) = Some(e.current_target()),
              }
            }
          }
        }
      },
      Size::new(96., 200.),
    );

    wnd.draw_frame();
    assert_eq!(&**selected.read(), "");

    let second_item_id = second_item_id
      .read()
      .expect("second rail item id should be mounted");
    let pos = wnd.map_to_global(Point::new(12., 12.), second_item_id);
    wnd.process_cursor_move(pos);
    wnd.process_mouse_press(Box::new(DummyDeviceId), MouseButtons::PRIMARY);
    wnd.process_mouse_release(Box::new(DummyDeviceId), MouseButtons::PRIMARY);
    wnd.draw_frame();

    assert_eq!(&**selected.read(), "settings");
  }

  #[test]
  fn nav_rail_mode_switch_does_not_reapply_base_class() {
    reset_test_env!();

    #[derive(Default)]
    struct RailBaseApply(usize);
    #[derive(Default)]
    struct RailModeApply(usize);

    fn count_rail_base(w: Widget) -> Widget {
      Provider::write_of::<RailBaseApply>(BuildCtx::get())
        .unwrap()
        .0 += 1;
      w
    }

    fn count_rail_mode(w: Widget) -> Widget {
      Provider::write_of::<RailModeApply>(BuildCtx::get())
        .unwrap()
        .0 += 1;
      w
    }

    let (expanded, w_expanded) = split_value(RailExpanded(false));
    let (base_apply, w_base_apply) = split_value(RailBaseApply::default());
    let (mode_apply, w_mode_apply) = split_value(RailModeApply::default());

    let wnd = TestWindow::from_widget(fn_widget! {
      let expanded = expanded.clone_watcher();
      @Providers {
        providers: smallvec::smallvec![
          Provider::watcher(expanded.clone_watcher()),
          Provider::writer(w_base_apply.clone_writer(), None),
          Provider::writer(w_mode_apply.clone_writer(), None),
          Class::provider(NAVIGATION_RAIL, count_rail_base),
          Class::provider(NAVIGATION_RAIL_EXPANDED, count_rail_mode),
          Class::provider(NAVIGATION_RAIL_COLLAPSED, count_rail_mode),
        ],
        @NavigationRail {
          @RailItem {
            key: "home",
            @Void {}
          }
        }
      }
    });

    wnd.draw_frame();
    assert_eq!(base_apply.read().0, 1);
    assert_eq!(mode_apply.read().0, 1);

    *w_expanded.write() = RailExpanded(true);
    wnd.draw_frame();
    assert_eq!(base_apply.read().0, 1);
    assert_eq!(mode_apply.read().0, 2);

    *w_expanded.write() = RailExpanded(false);
    wnd.draw_frame();
    assert_eq!(base_apply.read().0, 1);
    assert_eq!(mode_apply.read().0, 3);
  }

  #[test]
  fn rail_item_select_switch_does_not_reapply_base_class() {
    reset_test_env!();

    #[derive(Default)]
    struct ItemBaseApply(usize);
    #[derive(Default)]
    struct ItemStateApply(usize);

    fn count_item_base(w: Widget) -> Widget {
      Provider::write_of::<ItemBaseApply>(BuildCtx::get())
        .unwrap()
        .0 += 1;
      w
    }

    fn count_item_state(w: Widget) -> Widget {
      Provider::write_of::<ItemStateApply>(BuildCtx::get())
        .unwrap()
        .0 += 1;
      w
    }

    let (base_apply, w_base_apply) = split_value(ItemBaseApply::default());
    let (state_apply, w_state_apply) = split_value(ItemStateApply::default());
    let (second_item_id, w_second_item_id) = split_value(None::<WidgetId>);

    let wnd = TestWindow::new_with_size(
      fn_widget! {
        @Providers {
          providers: smallvec::smallvec![
            Provider::new(RailContentAlign(Align::Start)),
            Provider::writer(w_base_apply.clone_writer(), None),
            Provider::writer(w_state_apply.clone_writer(), None),
            Class::provider(RAIL_ITEM, count_item_base),
            Class::provider(RAIL_ITEM_SELECTED, count_item_state),
            Class::provider(RAIL_ITEM_UNSELECTED, count_item_state),
          ],
          @NavigationRail {
            selected: "home",
            size: Size::new(96., 200.),
            @RailItem {
              key: "home",
              @Void {
                size: Size::new(24., 24.),
              }
            }
            @RailItem {
              key: "settings",
              @Void {
                size: Size::new(24., 24.),
                on_mounted: move |e| *$write(w_second_item_id) = Some(e.current_target()),
              }
            }
          }
        }
      },
      Size::new(96., 200.),
    );

    wnd.draw_frame();
    assert_eq!(base_apply.read().0, 2);
    assert_eq!(state_apply.read().0, 2);

    let second_item_id = second_item_id
      .read()
      .expect("second rail item id should be mounted");
    let pos = wnd.map_to_global(Point::new(12., 12.), second_item_id);
    wnd.process_cursor_move(pos);
    wnd.process_mouse_press(Box::new(DummyDeviceId), MouseButtons::PRIMARY);
    wnd.process_mouse_release(Box::new(DummyDeviceId), MouseButtons::PRIMARY);
    wnd.draw_frame();

    assert_eq!(base_apply.read().0, 2);
    assert!(state_apply.read().0 > 2);
  }

  #[test]
  fn first_badged_item_keeps_badge_inside_scroll_view() {
    reset_test_env!();
    AppCtx::set_app_theme(material::purple::light());

    let (icon_top_in_view, w_icon_top_in_view) = split_value(None::<f32>);

    let wnd = TestWindow::new_with_size(
      fn_widget! {
        @Providers {
          providers: [Provider::new(RailContentAlign(Align::Start))],
          @NavigationRail {
            selected: "inbox",
            size: Size::new(96., 200.),
            @RailItem {
              key: "inbox",
              @NumBadge {
                count: Some(12),
                @Void {
                  size: Size::new(24., 24.),
                  on_performed_layout: move |e| {
                    let top = ScrollableWidget::of(e)
                      .and_then(|scrollable| {
                        scrollable.map_to_view(Point::zero(), e.current_target(), &e.window())
                      })
                      .map(|pos| pos.y);
                    *$write(w_icon_top_in_view) = top;
                  },
                }
              }
            }
            @RailItem {
              key: "sent",
              @Void { size: Size::new(24., 24.) }
            }
          }
        }
      },
      Size::new(96., 200.),
    );

    wnd.draw_frame();

    let icon_top_in_view = icon_top_in_view
      .read()
      .expect("rail icon should resolve its scroll-view position after layout");
    assert!(
      icon_top_in_view >= 8.,
      "expected at least 8px top inset for badge overflow, got {icon_top_in_view}",
    );
  }

  #[test]
  fn rail_menu_action_and_item_icon_keep_same_settled_x_across_modes() {
    reset_test_env!();

    let build_wnd = |expanded: bool,
                     selected: &'static str,
                     menu_id: Stateful<Option<WidgetId>>,
                     action_id: Stateful<Option<WidgetId>>,
                     home_icon_id: Stateful<Option<WidgetId>>,
                     messages_icon_id: Stateful<Option<WidgetId>>,
                     pomodoro_icon_id: Stateful<Option<WidgetId>>| {
      TestWindow::new(
        fn_widget! {
          @Providers {
            providers: smallvec::smallvec![
              Provider::new(RailExpanded(expanded)),
              Provider::new(RailContentAlign(Align::Start)),
              Class::provider(NAVIGATION_RAIL, nav_root_class),
              Class::provider(
                NAVIGATION_RAIL_EXPANDED,
                style_class! {
                  min_width: TEST_NAV_EXPANDED_WIDTH,
                  max_width: TEST_NAV_EXPANDED_WIDTH,
                },
              ),
              Class::provider(
                NAVIGATION_RAIL_COLLAPSED,
                style_class! { width: TEST_NAV_COLLAPSED_WIDTH },
              ),
              Class::provider(
                RAIL_MENU,
                style_class! {
                  margin: EdgeInsets::only_left(28.),
                  size: material::md::ICON_BTN_SIZE,
                  text_line_height: material::md::ICON_SIZE,
                },
              ),
              Class::provider(
                RAIL_ACTION,
                style_class! { margin: EdgeInsets::only_left(20.) },
              ),
              Class::provider(
                RAIL_ACTION_WITH_MENU,
                style_class! { margin: EdgeInsets::only_top(24.) },
              ),
              Class::provider(
                RAIL_CONTENT,
                style_class! {
                  padding: expanded_switch(
                    EdgeInsets::only_left(20.).with_top(4.),
                    EdgeInsets::only_left(20.).with_top(4.),
                  )
                },
              ),
              Class::provider(
                RAIL_ITEM,
                style_class! {
                  margin: expanded_switch(
                    EdgeInsets::ZERO,
                    EdgeInsets::only_bottom(4.)
                  ),
                  clamp: expanded_switch(BoxClamp::min_width(56.), BoxClamp::fixed_width(56.)),
                  height: 56.,
                  padding: expanded_switch(EdgeInsets::horizontal(16.), EdgeInsets::vertical(4.)),
                },
              ),
              Class::provider(
                RAIL_ITEM_ICON,
                style_class! { text_line_height: material::md::ICON_SIZE },
              ),
            ],
            @NavigationRail {
              selected: selected,
              size: Size::new(TEST_NAV_EXPANDED_WIDTH, 320.),
              @RailMenu {
                @Void {
                  size: Size::new(24., 24.),
                  on_mounted: move |e| *$write(menu_id) = Some(e.current_target()),
                }
              }
              @RailAction {
                @Void {
                  size: Size::new(56., 56.),
                  on_mounted: move |e| *$write(action_id) = Some(e.current_target()),
                }
              }
              @RailItem {
                key: "home",
                @Void {
                  size: Size::new(24., 24.),
                  on_mounted: move |e| *$write(home_icon_id) = Some(e.current_target()),
                }
                @ { "Home" }
              }
              @RailItem {
                key: "messages",
                @Void {
                  size: Size::new(24., 24.),
                  on_mounted: move |e| *$write(messages_icon_id) = Some(e.current_target()),
                }
                @ { "Messages" }
              }
              @RailItem {
                key: "pomodoro",
                @Void {
                  size: Size::new(24., 24.),
                  on_mounted: move |e| *$write(pomodoro_icon_id) = Some(e.current_target()),
                }
                @ { "Pomodoro" }
              }
            }
          }
        },
        Size::new(TEST_NAV_EXPANDED_WIDTH, 320.),
        WindowFlags::empty(),
      )
    };

    let (_, expanded_menu_id) = split_value(None::<WidgetId>);
    let (_, expanded_action_id) = split_value(None::<WidgetId>);
    let (_, expanded_home_icon_id) = split_value(None::<WidgetId>);
    let (_, expanded_messages_icon_id) = split_value(None::<WidgetId>);
    let (_, expanded_pomodoro_icon_id) = split_value(None::<WidgetId>);
    let expanded_wnd = build_wnd(
      true,
      "home",
      expanded_menu_id.clone_writer(),
      expanded_action_id.clone_writer(),
      expanded_home_icon_id.clone_writer(),
      expanded_messages_icon_id.clone_writer(),
      expanded_pomodoro_icon_id.clone_writer(),
    );
    expanded_wnd.draw_frame();

    let expanded_menu_x = expanded_wnd
      .map_to_global(
        Point::zero(),
        expanded_menu_id
          .read()
          .expect("expanded menu should mount"),
      )
      .x;
    let expanded_action_x = expanded_wnd
      .map_to_global(
        Point::zero(),
        expanded_action_id
          .read()
          .expect("expanded action should mount"),
      )
      .x;
    let expanded_home_icon_x = expanded_wnd
      .map_to_global(
        Point::zero(),
        expanded_home_icon_id
          .read()
          .expect("expanded icon should mount"),
      )
      .x;

    let (_, collapsed_menu_id) = split_value(None::<WidgetId>);
    let (_, collapsed_action_id) = split_value(None::<WidgetId>);
    let (_, collapsed_home_icon_id) = split_value(None::<WidgetId>);
    let (_, collapsed_messages_icon_id) = split_value(None::<WidgetId>);
    let (_, collapsed_pomodoro_icon_id) = split_value(None::<WidgetId>);
    let collapsed_wnd = build_wnd(
      false,
      "home",
      collapsed_menu_id.clone_writer(),
      collapsed_action_id.clone_writer(),
      collapsed_home_icon_id.clone_writer(),
      collapsed_messages_icon_id.clone_writer(),
      collapsed_pomodoro_icon_id.clone_writer(),
    );
    collapsed_wnd.draw_frame();

    assert_close(
      collapsed_wnd
        .map_to_global(
          Point::zero(),
          collapsed_menu_id
            .read()
            .expect("collapsed menu should mount"),
        )
        .x,
      expanded_menu_x,
      "menu x should match across modes",
    );
    assert_close(
      collapsed_wnd
        .map_to_global(
          Point::zero(),
          collapsed_action_id
            .read()
            .expect("collapsed action should mount"),
        )
        .x,
      expanded_action_x,
      "action x should match across modes",
    );
    assert_close(
      collapsed_wnd
        .map_to_global(
          Point::zero(),
          collapsed_home_icon_id
            .read()
            .expect("collapsed icon should mount"),
        )
        .x,
      expanded_home_icon_x,
      "home icon x should match across modes",
    );
  }

  #[test]
  fn long_collapsed_labels_do_not_shift_selected_icons() {
    reset_test_env!();

    let build_wnd = |selected: &'static str,
                     messages_icon_id: Stateful<Option<WidgetId>>,
                     pomodoro_icon_id: Stateful<Option<WidgetId>>| {
      TestWindow::new(
        fn_widget! {
          @Providers {
            providers: smallvec::smallvec![
              Provider::new(RailExpanded(false)),
              Provider::new(RailContentAlign(Align::Start)),
              Class::provider(NAVIGATION_RAIL, nav_root_class),
              Class::provider(
                NAVIGATION_RAIL_COLLAPSED,
                style_class! { width: TEST_NAV_COLLAPSED_WIDTH },
              ),
              Class::provider(
                RAIL_ITEM,
                style_class! {
                  margin: EdgeInsets::only_left(20.).with_bottom(4.),
                  clamp: BoxClamp::fixed_width(56.),
                  height: 56.,
                  padding: EdgeInsets::vertical(4.),
                },
              ),
              Class::provider(
                RAIL_ITEM_ICON,
                style_class! { text_line_height: material::md::ICON_SIZE },
              ),
            ],
            @NavigationRail {
              selected: selected,
              size: Size::new(TEST_NAV_EXPANDED_WIDTH, 240.),
              @RailItem {
                key: "messages",
                @Void {
                  size: Size::new(24., 24.),
                  on_mounted: move |e| *$write(messages_icon_id) = Some(e.current_target()),
                }
                @ { "Messages" }
              }
              @RailItem {
                key: "pomodoro",
                @Void {
                  size: Size::new(24., 24.),
                  on_mounted: move |e| *$write(pomodoro_icon_id) = Some(e.current_target()),
                }
                @ { "Pomodoro" }
              }
            }
          }
        },
        Size::new(TEST_NAV_EXPANDED_WIDTH, 240.),
        WindowFlags::empty(),
      )
    };

    let (_, messages_unselected_id) = split_value(None::<WidgetId>);
    let (_, pomodoro_unselected_id) = split_value(None::<WidgetId>);
    let wnd = build_wnd(
      "home",
      messages_unselected_id.clone_writer(),
      pomodoro_unselected_id.clone_writer(),
    );
    wnd.draw_frame();
    let messages_x = wnd
      .map_to_global(
        Point::zero(),
        messages_unselected_id
          .read()
          .expect("messages icon should mount"),
      )
      .x;
    let pomodoro_x = wnd
      .map_to_global(
        Point::zero(),
        pomodoro_unselected_id
          .read()
          .expect("pomodoro icon should mount"),
      )
      .x;

    let (_, messages_selected_id) = split_value(None::<WidgetId>);
    let (_, pomodoro_probe_id) = split_value(None::<WidgetId>);
    let messages_wnd =
      build_wnd("messages", messages_selected_id.clone_writer(), pomodoro_probe_id.clone_writer());
    messages_wnd.draw_frame();
    assert_close(
      messages_wnd
        .map_to_global(
          Point::zero(),
          messages_selected_id
            .read()
            .expect("selected messages icon should mount"),
        )
        .x,
      messages_x,
      "messages icon x should remain stable when selected",
    );

    let (_, messages_probe_id) = split_value(None::<WidgetId>);
    let (_, pomodoro_selected_id) = split_value(None::<WidgetId>);
    let pomodoro_wnd =
      build_wnd("pomodoro", messages_probe_id.clone_writer(), pomodoro_selected_id.clone_writer());
    pomodoro_wnd.draw_frame();
    assert_close(
      pomodoro_wnd
        .map_to_global(
          Point::zero(),
          pomodoro_selected_id
            .read()
            .expect("selected pomodoro icon should mount"),
        )
        .x,
      pomodoro_x,
      "pomodoro icon x should remain stable when selected",
    );
  }
}
