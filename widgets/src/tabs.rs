use ribir_core::prelude::*;

use crate::prelude::*;

/// Hierarchical content organization widget for navigation and view
/// switching
///
/// The Tabs widget manages grouped content at equivalent hierarchy levels
/// through tabbed navigation.
///
/// Each tab consists of:
/// - Optional header widgets (icon/label)
/// - Lazy-loaded content pane (`GenWidget`)
///
/// ## Basic Usage
///
/// While tabs support flexible content configurations, maintain visual
/// consistency across headers for better UX:
///
/// ```rust
/// use ribir::prelude::*;
///
/// tabs! {
///   @Tab {
///     @ { "News" }
///     @text! { text: "Breaking news content..." }
///   }
///   @Tab {
///     @ { "Sports" }
///     @Icon { @named_svgs::get_or_default("sports") }
///     @text! { text: "Live sports updates..." }
///   }
///   @Tab {
///     @ { "Settings" }
///     @Icon { @named_svgs::get_or_default("settings") }
///     @text! { text: "System configuration..." }
///   }
/// };
/// ```
///
/// ## Configuration Architecture
///
/// Style parameters use provider-based configuration to:
///
/// 1. Enable theme customization and override capabilities
/// 2. Maintain lean API surface for core component
///
/// Key configuration providers:
///
/// | Provider           | Purpose                          | Default     |
/// |--------------------|----------------------------------|-------------|
/// | `TabPos`           | Tab header position              | `Top`       |
/// | `TabType`          | Visual hierarchy level           | `Primary`   |
/// | `TabsInlineIcon`   | Icon/label layout mode           | `true`      |
/// | `Color`            | Color active header              |  theme.primary    |
///
/// ```rust
/// use ribir::prelude::*;
/// use smallvec::smallvec;
///
/// tabs! {
///   providers: smallvec![
///     Provider::new(TabPos::Left),
///     Provider::new(TabType::Secondary),
///     Provider::new(TabsInlineIcon(true)),
///     Provider::new(Palette::of(BuildCtx::get()).secondary()),
///   ],
///   @Tab {
///     @ { "Mail" }
///     @Icon { @named_svgs::get_or_default("mail") }
///     @text!{ text: "Mail widget here" }
///   }
///   @Tab {
///     @ { "Calendar" }
///     @Icon { @named_svgs::get_or_default("calendar") }
///     @text!{ text: "Calendar widget here" }
///   }
///   @Tab {
///     @ { "Files" }
///     @Icon { @named_svgs::get_or_default("files") }
///     @text!{ text: "Files widget here" }
///   }
/// };
/// ```
///
/// ## Implementation Notes
///
/// 1. Content panes initialize lazily through `GenWidget`
/// 2. Header consistency checks recommend using `TabInfo` metadata
/// 3. Theme overrides may ignore certain configurations
#[derive(Declare, Clone)]
pub struct Tabs {
  /// The index of the currently active tab.
  #[declare(default)]
  active: usize,
  /// The number of tabs.
  #[declare(skip)]
  tabs_cnt: usize,
}

class_names! {
  /// Class name for the icon of the tab header
  TAB_ICON,
  /// Class name for the label of the tab header
  TAB_LABEL,
  /// Class name for the tab header no matter is active or not
  TAB_HEADER,
  /// Class name for the scrollable view of the tab headers.
  TAB_HEADERS_VIEW,
  /// Class name for the tab headers container
  TAB_HEADERS_CONTAINER,
  /// Class name for the tab pane
  TAB_PANE,
  /// Class name for the whole tabs
  TABS
}

/// The `Tab` is utilized to define a tab within a set of tabs. Each tab
/// consists of a label and an icon as properties, with a pane as its child
/// widget.
#[derive(Template)]
pub struct Tab<'t> {
  icon: Option<IconTml<'t>>,
  label: Option<TextInit>,
  pane: Option<GenWidget>,
}

/// A provider let the user specify the position of the tabs. The default value
/// is top.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum TabPos {
  #[default]
  Top,
  Bottom,
  Left,
  Right,
}

/// The provider allows users to select a tab style. The actual appearance is
/// determined by the theme and a theme may only support a subset of the
/// options.
///
/// The `Tabs` widget does not require the theme maker to support dynamic
/// changes of this provider. Therefore, the user should not provide a writer
/// state of the `TabType`, as this may not work as expected.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum TabType {
  #[default]
  Primary,
  Secondary,
  Tertiary,
  Quaternary,
}

/// The provider controls inline display of tab labels and icons in header. This
/// is not a forceful requirement for the theme, so the themes may override this
/// configuration if their design system prohibits.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TabsInlineIcon(pub bool);

/// Represents metadata about a tab, including its index and whether it contains
/// an icon or a label.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TabInfo {
  pub idx: usize,
  pub has_icon: bool,
  pub has_label: bool,
}

impl TabInfo {
  /// Checks if the tab displays only an icon (no label).
  pub fn is_icon_only(&self) -> bool { self.has_icon && !self.has_label }

  /// Checks if the tab displays only a label (no icon).
  pub fn is_label_only(&self) -> bool { self.has_label && !self.has_icon }

  /// Checks if the tab displays both an icon and a label.
  pub fn has_icon_and_label(&self) -> bool { self.has_icon && self.has_label }
}

impl Tabs {
  pub fn active_idx(&self) -> usize { self.active }

  pub fn set_active(&mut self, idx: usize) {
    if idx < self.tabs_cnt {
      self.active = idx;
    }
  }
}

impl<'c> ComposeChild<'c> for Tabs {
  type Child = Vec<Tab<'c>>;

  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
    fn_widget! {
      this.silent().tabs_cnt = child.len();
      let position = Variant::<TabPos>::new_or_default(BuildCtx::get());

      let (headers, panes): (Vec<_>, Vec<_>) = child
        .into_iter()
        .enumerate()
        .map(|(idx, tab)| tab.into_header_and_pane(idx))
        .unzip();

      @Flex {
        providers: [Provider::value_of_writer(this.clone_writer(), None)],
        class: TABS,
        direction: position.clone().map(TabPos::main_dir),
        reverse: position.clone().map(TabPos::main_reverse),
        align_items: Align::Stretch,
        @ScrollableWidget {
          class: TAB_HEADERS_VIEW,
          scrollable: position.clone().map(TabPos::headers_scroll_dir),
          @Flex {
            align_items: Align::Stretch,
            direction: position.map(TabPos::headers_dir),
            class: TAB_HEADERS_CONTAINER,
            @ { headers }
          }
        }
        @Expanded {
          defer_alloc: true,
          @ { pipe!($this.active).map(move |idx| panes[idx].gen_widget()) }
        }
      }
    }
    .into_widget()
  }
}

impl<'w> Tab<'w> {
  fn into_header_and_pane(mut self, idx: usize) -> (Widget<'w>, GenWidget) {
    let pane = self.take_pane();
    let header = self.tab_header(idx);
    (header, pane)
  }

  fn tab_header(self, idx: usize) -> Widget<'w> {
    let tab_info = self.info(idx);
    fn_widget! {
      let ctx = BuildCtx::get();
      let inline = Variant::<TabsInlineIcon>::new_or_default(ctx);
      let line = match inline {
        Variant::Value(inline) => inline.into_line_widget(),
        Variant::Watcher(w) => Box::new(pipe!($w.into_line_widget())),
      };

      let header = @Class {
        class: TAB_HEADER,
        on_tap: move |e| {
          let prev = Provider::of::<Tabs>(e).unwrap().active;
          if prev != idx {
            Provider::write_of::<Tabs>(e).unwrap().set_active(idx);
          }
        },
        @ $line {
          @ { self.icon.map(|icon| icon! { class: TAB_ICON, @{ icon } }) }
          @ { self.label.map(|label| text! { text: label, class: TAB_LABEL }) }
        }
      };

      @Expanded {
        @Providers {
          providers: [Provider::new(tab_info)],
          @ { header }
        }
      }
    }
    .into_widget()
  }

  pub fn take_pane(&mut self) -> GenWidget {
    let pane = self
      .pane
      .take()
      .unwrap_or_else(|| void! {}.into());

    fat_obj! {
      class: TAB_PANE,
      @pane.gen_widget()
    }
    .into()
  }

  pub fn info(&self, idx: usize) -> TabInfo {
    TabInfo { has_icon: self.icon.is_some(), has_label: self.label.is_some(), idx }
  }
}

impl TabPos {
  pub fn is_horizontal(self) -> bool { matches!(self, TabPos::Top | TabPos::Bottom) }

  fn main_dir(self) -> Direction {
    match self {
      TabPos::Top | TabPos::Bottom => Direction::Vertical,
      TabPos::Left | TabPos::Right => Direction::Horizontal,
    }
  }

  fn main_reverse(self) -> bool {
    match self {
      TabPos::Top | TabPos::Left => false,
      TabPos::Bottom | TabPos::Right => true,
    }
  }

  fn headers_dir(self) -> Direction {
    match self {
      TabPos::Top | TabPos::Bottom => Direction::Horizontal,
      TabPos::Left | TabPos::Right => Direction::Vertical,
    }
  }

  fn headers_scroll_dir(self) -> Scrollable {
    match self {
      TabPos::Top | TabPos::Bottom => Scrollable::X,
      TabPos::Left | TabPos::Right => Scrollable::Y,
    }
  }
}

impl TabsInlineIcon {
  fn into_line_widget(self) -> Box<dyn MultiChild> {
    if self.0 { Box::new(HorizontalLine) } else { Box::new(VerticalLine) }
  }
}

impl Default for TabsInlineIcon {
  fn default() -> Self { TabsInlineIcon(true) }
}

#[cfg(test)]
mod tests {
  use ribir_core::test_helper::*;
  use ribir_dev_helper::*;
  use smallvec::smallvec;

  use super::*;

  fn tabs_tester(tab_type: TabType, pos: TabPos) -> WidgetTester {
    WidgetTester::new(tabs! {
      providers: smallvec![Provider::new(tab_type), Provider::new(pos)],
      h_align: HAlign::Stretch,
      v_align: VAlign::Stretch,
      // Tab only label
      @Tab {
        @{ "Tab 1" }
        @text! { text: "Only label" }
      }
      // Tab only icon
      @Tab {
         @Icon { @named_svgs::default() }
      }
      // Tab with label and icon
      @Tab {
        @ { "Tab 3" }
        @Icon { @named_svgs::default() }
        @text! { text: "Label and icon" }
      }
    })
    .with_wnd_size(Size::new(256., 128.))
  }

  widget_image_tests!(primary_left, tabs_tester(TabType::Primary, TabPos::Left),);

  widget_image_tests!(primary_right, tabs_tester(TabType::Primary, TabPos::Right),);

  widget_image_tests!(primary_top, tabs_tester(TabType::Primary, TabPos::Top),);

  widget_image_tests!(primary_bottom, tabs_tester(TabType::Primary, TabPos::Bottom),);

  widget_image_tests!(secondary_left, tabs_tester(TabType::Secondary, TabPos::Left),);

  widget_image_tests!(secondary_right, tabs_tester(TabType::Secondary, TabPos::Right),);

  widget_image_tests!(secondary_top, tabs_tester(TabType::Secondary, TabPos::Top),);

  widget_image_tests!(secondary_bottom, tabs_tester(TabType::Secondary, TabPos::Bottom),);
}
