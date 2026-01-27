//! # Navigation Rail
//!
//! The `NavigationRail` is an adaptive side-navigation widget designed for
//! medium to large screens. It provides quick access to top-level destinations
//! while adjusting its layout based on the available space and the desired
//! interaction style.
//!
//! ## Core Architecture
//!
//! The rail is organized into three distinct vertical zones:
//! 1. **Header**: Located at the top, typically containing a navigation toggle
//!    (Menu) and a primary action.
//! 2. **Content**: The central area containing navigation destinations
//!    ([RailItem]) and group headers ([RailSection]).
//! 3. **Footer**: Located at the bottom for secondary actions or user profiles.
//!
//! ## Layout Modes
//!
//! The rail operates in two primary modes controlled by the `expanded`
//! property:
//! - **Collapsed (Narrow)**: Optimized for space. Items are usually stacked
//!   vertically, showing icons with optional labels.
//! - **Expanded (Wide)**: Optimized for clarity. Items are laid out
//!   horizontally with icons and labels side-by-side, similar to a navigation
//!   drawer.
//!
//! ## Label Behavior
//!
//! In Collapsed mode, you can control how labels appear using the
//! `label_policy`:
//! - `None`: Only icons are visible.
//! - `Always`: Labels are always visible below the icons.
//! - `OnSelected`: Labels only appear when the item is active, providing a
//!   dynamic "pop-up" effect.
//!
//! In Expanded mode, labels are always visible to provide maximum context.
//!
//! ## RailExpanded Provider
//!
//! `RailExpanded` is expected to be a **writable** state when you want the rail
//! to respond to user interactions (e.g., `RailMenu` toggling). If an external
//! provider supplies only a read-only value or watcher, the expanded state
//! becomes fixed and interactive transitions will not occur. When no
//! `RailExpanded` provider is found, `NavigationRail` will create and provide
//! an internal `Stateful<RailExpanded>` so the rail can still respond to user
//! actions by default.
//!
//! Why a provider and not a property?
//!
//! Making `RailExpanded` a provider (instead of a simple `NavigationRail`
//! field) allows the expanded state to be controlled or observed by widgets
//! that live far away in the widget tree. Often the control that toggles
//! expansion — for example a top app bar menu button — is not a direct child of
//! the rail. By using a provider you can decouple the toggle source from the
//! rail itself, letting multiple parts of the app read or write the same
//! expanded state.
//!
//! Note: if an ancestor provides only a watcher/value (read-only) for
//! `RailExpanded`, `NavigationRail` cannot mutate it and local interactive
//! toggles (like `RailMenu`) will have no effect. To enable toggling from the
//! rail, provide a writable state (e.g., `Provider::writer(Stateful::new(...),
//! None)`) or let the rail create its own internal `Stateful<RailExpanded>` by
//! not providing one at all. The gallery example demonstrates controlling the
//! rail expansion from a remote menu button via a shared provider.
//!
//! ## Semantic Hierarchy
//!
//! - [NavigationRail]: The root container that manages layout modes and
//!   alignment.
//! - [RailHeader]: A structured template providing specific slots for `menu`
//!   and `action`.
//! - [RailItem]: The interactive destination. It uses a template to
//!   automatically match [Icon], [TextValue] (label), and [Badge] from the DSL.
//! - [RailSection]: A semantic boundary for grouping items. It adapts from a
//!   text header in Expanded mode to a visual divider in Collapsed mode.
//!
//! ## Basic Usage
//!
//! ```rust
//! use ribir::prelude::*;
//!
//! let is_expanded = Stateful::new(RailExpanded(false));
//!
//! providers! {
//!   providers: [
//!     Provider::writer(is_expanded.clone_writer(), None),
//!     Provider::new(RailLabelPolicy::OnSelected),
//!   ],
//!   @NavigationRail {
//!     @RailMenu {
//!       @FatObj {
//!         on_tap: move |_| $write(is_expanded).toggle(),
//!         @ { svg_registry::MENU }
//!       }
//!     }
//!
//!     @RailItem {
//!       @ { svg_registry::HOME }
//!       @ { "Home" }
//!     }
//!
//!     @RailSection { @ { "Groups" } }
//!
//!     @RailItem {
//!       @ { svg_registry::FOLDER }
//!       @ { "Projects" }
//!       @Badge { content: "12" }
//!     }
//!
//!     @Footer {
//!       @IconButton { @ { svg_registry::SETTINGS } }
//!     }
//!   }
//! }
//! ```

use ribir_core::prelude::*;
use smallvec::smallvec;

use crate::prelude::*;

/// Defines the display strategy for labels in the navigation rail when in
/// collapsed mode.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum RailLabelPolicy {
  /// Labels are never visible. Icons are centered within the item.
  #[default]
  None,
  /// Labels are only visible when the item is selected.
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
  /// Class for the menu button.
  RAIL_MENU,
  /// Class for the primary action button.
  RAIL_ACTION,
  /// Class for the content area (items and sections).
  RAIL_CONTENT,
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

/// A primary action widget wrapper, typically a [FloatingActionButton].
#[derive(Template)]
pub struct RailAction(pub Widget<'static>);

/// A footer widget wrapper for secondary actions or user profiles.
#[derive(Template)]
pub struct RailFooter(pub Widget<'static>);

/// A group header used to categorize items within the [NavigationRail].
/// It provides a semantic boundary and adapts its visual style based on the
/// rail's mode.
#[derive(Template)]
pub struct RailSection(TextValue);

/// The state holder for an individual navigation destination in the rail.
#[declare]
pub struct RailItem {
  /// Business identifier for selection state matching.
  /// - User-provided: used directly
  /// - User-omitted: NavigationRail auto-supplements index string
  /// - Runtime guarantee: non-empty after ComposeChild
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

/// Supported child types for [NavigationRail].
#[derive(Template)]
pub enum RailChild<'c> {
  /// Toggle button for the rail mode.
  Menu(RailMenu),
  /// Primary action button (e.g., FAB).
  Action(RailAction),
  /// Individual navigation destination.
  Item(PairOf<'c, RailItem>),
  /// A section header for grouping items.
  Section(RailSection),
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
  /// This field is bound to `RailSelect.to` (via `event =
  /// RailSelect.to.clone()`) so bubbling `RailSelect` events update
  /// `selected` automatically (supports uncontrolled and TwoWay modes).
  #[declare(default, event = RailSelect.to.clone())]
  pub selected: CowArc<str>,

  /// Internal navigation item key list for query methods.
  #[declare(skip)]
  items: Vec<CowArc<str>>,
}

impl NavigationRail {
  pub fn keys(&self) -> &[CowArc<str>] { &self.items }

  pub fn selected_key(&self) -> Option<&str> {
    let selected = &*self.selected;
    if selected.is_empty() { None } else { Some(selected) }
  }

  pub fn selected_key_owned(&self) -> Option<CowArc<str>> {
    if self.selected.is_empty() { None } else { Some(self.selected.clone()) }
  }

  pub fn next_key(&self) -> Option<&str> {
    let next_idx = self.current_index().map(|i| i + 1).unwrap_or(0);
    self.items.get(next_idx).map(|s| &**s)
  }

  pub fn prev_key(&self) -> Option<&str> {
    match self.current_index() {
      Some(idx) if idx > 0 => Some(&*self.items[idx - 1]),
      None => self.items.last().map(|s| &**s),
      _ => None,
    }
  }

  pub fn next_key_cyclic(&self) -> Option<&str> {
    let idx = self
      .current_index()
      .map(|i| (i + 1) % self.items.len())
      .unwrap_or(0);
    self.items.get(idx).map(|s| &**s)
  }

  pub fn prev_key_cyclic(&self) -> Option<&str> {
    let idx = self
      .current_index()
      .map(|i| (i + self.items.len() - 1) % self.items.len())
      .unwrap_or_else(|| self.items.len().saturating_sub(1));
    self.items.get(idx).map(|s| &**s)
  }

  fn current_index(&self) -> Option<usize> {
    let key = self.selected_key()?;
    self.items.iter().position(|k| &**k == key)
  }
}

impl<'c> ComposeChild<'c> for NavigationRail {
  type Child = Vec<RailChild<'c>>;

  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
    let (menu, action, footer, content_children) = this.silent().partition_children(child);
    let rail_state_cls = NavigationRail::rail_state_class();
    let mut providers = smallvec![Provider::writer(this, None)];
    if Provider::of::<RailExpanded>(BuildCtx::get()).is_none() {
      providers.push(Provider::writer(Stateful::new(RailExpanded::default()), None));
    }
    providers! {
      providers: providers,
      @ClassChain {
        class_chain: [NAVIGATION_RAIL.r_into(), rail_state_cls],
        @Column {
          @ { menu }
          @ { action }
          @Expanded {
            @Column {
              class: RAIL_CONTENT,
              @ { content_children }
            }
          }
          @ { footer }
        }
      }
    }
    .into_widget()
  }
}

impl<'c> ComposeChild<'c> for RailItem {
  type Child = RailItemChildren<'c>;

  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
    let key = this.read().key.clone();
    let RailItemChildren { icon, label, badge } = child;
    let metadata = RailItemMetadata { has_label: label.is_some(), has_badge: badge.is_some() };

    fn_widget! {
      let rail = Provider::state_of::<Stateful<NavigationRail>>(BuildCtx::get())
        .expect("NavigationRail provider must be in scope")
        .clone_writer();

      let expanded = Variant::<RailExpanded>::new_or_default(BuildCtx::get());
      let label_widget = this.read()
        .label_widget(label, expanded.clone(), &rail);

      let sel_item_cls = this.read().selected_cls(&rail);
      @Providers {
        providers: smallvec![Provider::new(metadata)],
        @ClassChain {
          class_chain: [RAIL_ITEM.r_into(), sel_item_cls.r_into()],
          @Flex {
            direction: expanded.clone().map(RailExpanded::direction),
            align_items: Align::Center,
            item_gap: expanded.map(|e| if e.0 { 12. } else { 4. }),
            on_action: move |e| {
              let Some(rail) = Provider::of::<NavigationRail>(e) else { return; };
              let from = rail.selected_key_owned();
              let to = $clone(key);
              if from.as_ref() != Some(&to) {
                e.window()
                  .bubble_custom_event(e.target(), RailSelect { from, to });
              }
            },
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

  fn build_section_widget<'c>(title: TextValue) -> Widget<'c> {
    text! { class: RAIL_SECTION, text: title }.into_widget()
  }

  fn rail_state_class() -> PipeValue<Option<ClassName>> {
    Variant::<RailExpanded>::new_or_default(BuildCtx::get())
      .map(|expanded| if expanded.0 { NAVIGATION_RAIL_EXPANDED } else { NAVIGATION_RAIL_COLLAPSED })
      .r_into()
  }

  fn partition_children<'c>(
    &mut self, children: Vec<RailChild<'c>>,
  ) -> (Option<Widget<'c>>, Option<Widget<'c>>, Option<Widget<'c>>, Vec<Widget<'c>>) {
    let mut menu: Option<Widget<'c>> = None;
    let mut action: Option<Widget<'c>> = None;
    let mut footer: Option<Widget<'c>> = None;
    let mut content_children: Vec<Widget<'c>> = Vec::new();

    let mut index = 0;
    for child in children.into_iter() {
      match child {
        RailChild::Menu(m) => {
          assert!(menu.is_none(), "NavigationRail can only have one RailMenu");
          menu = Some(icon! { class: RAIL_MENU, @ { m.0 } }.into_widget());
        }
        RailChild::Action(a) => {
          assert!(action.is_none(), "NavigationRail can only have one RailAction");
          action = Some(Self::wrap_with_class(RAIL_ACTION, a.0));
        }
        RailChild::Footer(f) => {
          footer = Some(Self::wrap_with_class(RAIL_FOOTER, f.0));
        }
        RailChild::Section(RailSection(title)) => {
          content_children.push(Self::build_section_widget(title))
        }
        RailChild::Item(pair) => {
          index += 1;
          let key = pair.parent().silent().ensure_key(index);
          self.items.push(key.clone());
          content_children.push(pair.into_widget());
        }
      }
    }

    (menu, action, footer, content_children)
  }
}

impl RailExpanded {
  pub fn toggle(&mut self) { self.0 = !self.0; }

  fn direction(&self) -> Direction {
    if self.0 { Direction::Horizontal } else { Direction::Vertical }
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

  fn selected_cls(&self, rail: &Stateful<NavigationRail>) -> Pipe<ClassName> {
    let key = self.key.clone();
    distinct_pipe! {
      let selected = $read(rail).selected.clone();
      if !selected.is_empty() && selected == key {
        RAIL_ITEM_SELECTED
      } else {
        RAIL_ITEM_UNSELECTED
      }
    }
  }

  fn label_widget<'c>(
    &self, label: Option<TextValue>, expanded: Variant<RailExpanded>,
    rail: &Stateful<NavigationRail>,
  ) -> Option<Widget<'c>> {
    let rail_selected = rail.part_watcher(|rail| PartRef::from(&rail.selected));
    let label_visible = self.label_visible_pipe(expanded, rail_selected);
    label.map(|text| {
      text! {
        visible: label_visible,
        class: RAIL_ITEM_LABEL,
        text
      }
      .into_widget()
    })
  }

  fn label_visible_pipe<R>(&self, expanded: Variant<RailExpanded>, selected: R) -> PipeValue<bool>
  where
    R: VariantInput<Value = CowArc<str>>,
    R::Source: VariantSnapshot,
  {
    let label_policy = Variant::<RailLabelPolicy>::new_or_default(BuildCtx::get());
    let key = self.key.clone();
    expanded
      .combine(label_policy)
      .combine_with(
        selected,
        move |((expanded, policy), selected): &((RailExpanded, RailLabelPolicy), CowArc<str>)| {
          RailItem::is_label_visible(*expanded, *policy, selected, &*key)
        },
      )
      .into_pipe_value()
  }

  fn is_label_visible(
    expanded: RailExpanded, policy: RailLabelPolicy, selected: &CowArc<str>, item_key: &str,
  ) -> bool {
    match (expanded.0, policy) {
      (true, _) => true,
      (false, RailLabelPolicy::None) => false,
      (false, RailLabelPolicy::Always) => true,
      (false, RailLabelPolicy::OnSelected) => {
        let selected = &**selected;
        !selected.is_empty() && selected == item_key
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use ribir_core::test_helper::*;

  use super::*;

  #[test]
  fn rail_select_updates_two_way_selected() {
    reset_test_env!();
    let selected = Stateful::new(CowArc::from("todos"));
    let selected_writer = selected.clone_writer();

    let w = fn_widget! {
      let mut rail = @NavigationRail {
        selected: TwoWay::new(selected_writer.clone_writer()),
        @RailItem {
          key: "todos",
          @Icon { @svg_registry::default_svg() }
          @ { "Todos" }
        }
        @RailItem {
          key: "messages",
          @Icon { @svg_registry::default_svg() }
          @ { "Messages" }
        }
      };
      @(rail) {
        on_mounted: move |e| {
          e.window().bubble_custom_event(
            e.current_target(),
            RailSelect { from: Some("todos".into()), to: "messages".into() },
          );
        }
      }
    };

    let wnd = TestWindow::from_widget(w);
    wnd.draw_frame();

    assert_eq!(&**selected.read(), "messages");
  }
}
