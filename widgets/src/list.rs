use ribir_core::prelude::*;

use crate::prelude::*;

/// A vertical list widget supporting multiple selection modes and item types.
///
/// Key features:
/// - Configurable selection behavior ([`ListSelectMode`])
/// - Mix of standard ([`ListItem`]) and custom ([`ListCustomItem`]) items, as
///   well as dividers between items
/// - Interactive and non-interactive modes
/// - Theming support
///
/// ## Selection Modes
///
/// Control selection behavior with [`ListSelectMode`]:
/// - **`None`**: Display-only with no selection logic
/// - **`Single`**: Exclusive selection (radio button-like)
/// - **`Multi`**: Multiple selection (checkbox-style)
///
/// Note: [`ListItem`] state can be controlled programmatically in any mode.
///
/// ## Standard List Items
///
/// Structured items with consistent sections:
///
/// ```rust
/// use ribir::prelude::*;
///
/// list! {
///   select_mode: ListSelectMode::Single,
///   @ListItem {
///     @Icon { @named_svgs::default() }
///     @ListItemHeadline { @ { "Primary Text" } }
///     @ListItemSupporting { @ { "Secondary Text" } }
///     @ListItemTrailingSupporting { @{ "100+" } }
///   }
///   @ListItem {
///     @Icon { @named_svgs::default() }
///     @ListItemHeadline { @ { "Headline Only" } }
///     @Trailing { @Icon { @named_svgs::default() } }
///   }
/// };
/// ```
///
/// ## Custom List Items
///
/// Embed arbitrary content with [`ListCustomItem`]:
///
/// ```rust
/// use ribir::prelude::*;
///
/// list! {
///   select_mode: ListSelectMode::Multi,
///   @ListCustomItem { @H1 { text: "Custom Header" } }
///   @ListCustomItem { @Text { text: "Custom Content" } }
/// };
/// ```
///
/// ## Mixed Items
///
/// Combine standard and custom items:
///
/// ```rust
/// use ribir::prelude::*;
///
/// list! {
///   select_mode: ListSelectMode::None,
///   @ListCustomItem { @H2 { text: "Section Header" } }
///   @ListItem {
///     @ListItemHeadline { @ { "First Item" } }
///     @Trailing { @Icon { @named_svgs::get_or_default("info") } }
///   }
///   @ListItem {
///     @ListItemHeadline { @ { "Second Item" } }
///     @ListItemSupporting { @ { "With description" } }
///   }
/// };
/// ```
///
/// ## Actionable Items
///
/// Handle interactions directly:
///
/// ```rust
/// use ribir::prelude::*;
///
/// list! {
///   select_mode: ListSelectMode::None,
///   @ListItem {
///     on_tap: move |_| log::info!("Item tapped"),
///     @ListItemHeadline { @ { "Press Me" } }
///   }
///   @ListItem {
///     on_tap: move |_| log::info!("Another tap"),
///     @ListItemHeadline { @ { "Press Me Too" } }
///   }
/// };
/// ```
///
/// ## Non-interactive Mode
///
/// Display static content with disabled interactions:
///
/// ```rust
/// use ribir::prelude::*;
///
/// list! {
///   select_mode: ListSelectMode::None,
///   @ListItem {
///     interactive: false,
///     @ListItemHeadline { @ { "Static Content" } }
///   }
///   @ListItem {
///     interactive: false,
///     @ListItemHeadline { @ { "Checkbox Controlled" } }
///     @Trailing {
///       @TextButton {
///         on_tap: move |_| log::info!("Star clicked"),
///         @Icon { @named_svgs::get_or_default("star") }
///       }
///     }
///   }
/// };
/// ```
///
/// ## Theming
///
/// Customize appearance through these key mechanisms:
///
/// - **Color Providers** control active/selected colors using `Color`x
///   providers
///
/// - **Vertical Alignment** `ListItemAlignItems` provider positions sections
///   (theme implementations override user settings)
///
/// - **Styling Classes** target core classes with cascading specializations:
///   - `LIST`: Root container
///   - `LIST_ITEM`: Individual items
///   - `LIST_ITEM_CONTENT`: Main content area
///   - *(Plus additional classes for sub-widgets, states, and variants)*
///
/// - **Structural Metadata**   `ListItemStructInfo` exposes item structure for
///   theme class implementations

#[derive(Declare)]
pub struct List {
  /// The selection mode for the list items.
  /// - `None`: No selection
  /// - `Single`: Only one item can be selected at a time
  /// - `Multi`: Multiple items can be selected
  ///
  /// Default: [`ListSelectMode::None`]
  #[declare(default)]
  select_mode: ListSelectMode,
  /// Tracks keyboard navigation focus
  #[declare(skip)]
  active_item: Option<usize>,
  /// The items in the list.
  #[declare(skip)]
  items: Vec<Stateful<ListItem>>,
  /// Handles single-select subscription cleanup
  #[declare(skip)]
  subscriptions: Vec<BoxSubscription<'static>>,
}

// Defines class names used for styling list widgets
class_names! {
  /// Root container class for the List widget
  LIST,
  /// Class for selected list item containers
  LIST_ITEM_SELECTED,
  /// Class for unselected list item containers
  LIST_ITEM_UNSELECTED,
  /// Class for interactive list items
  LIST_ITEM_INTERACTIVE,
  /// Base class for all list items
  LIST_ITEM,
  /// Content section of a list item
  LIST_ITEM_CONTENT,
  /// Primary headline text in a list item
  LIST_ITEM_HEADLINE,
  /// Supporting text section in a list item
  LIST_ITEM_SUPPORTING,
  /// Trailing supporting text section
  LIST_ITEM_TRAILING_SUPPORTING,
  /// Image container within a list item
  LIST_ITEM_IMG,
  /// Thumbnail image container
  LIST_ITEM_THUMBNAIL,
  /// Leading widget container (left side)
  LIST_ITEM_LEADING,
  /// Trailing widget container (right side)
  LIST_ITEM_TRAILING
}

/// A single widget within a [`List`] that can be selected and/or interacted
/// with.
///
/// This widget accepts a template as its child that predefined the children
/// structure. The template contains four optional sections:
///
///
/// ```text
/// +------------------ +--------------------+---------------------+------------------+
/// │ Leading           │ Content            │ Trailing Supporting │  Trailing        │
/// │------------------ +--------------------+---------------------+------------------+
/// │ Icon              │ ListItemHeadline   │                     │                  │
/// │ Avatar            │ (Required)         │  ListItem-          │ Trailing(Widget) │
/// │ ListItemImage     +--------------------+  TrailingSupporting │ (Optional)       │
/// │ ListItemThumbNail │ ListItemSupporting │  (Optional)         │                  │
/// │ Any widget        │ (Optional)         │                     │                  │
/// +------------------ +--------------------+---------------------+------------------+
/// ```

#[derive(Declare)]
pub struct ListItem {
  /// Controls visual feedback for user interactions without affecting selection
  /// logic. The visual effect is determined by the user's theme settings.
  /// Feedback can be enabled or disabled per item, independent of the
  /// select_mode mode configuration in the parent [`List`] widget.
  #[declare(default = true)]
  interactive: bool,
  /// Indicates if this item is currently chosen/activated
  #[declare(default)]
  selected: bool,
  /// Internal identifier for keyboard navigation tracking
  #[declare(skip)]
  wid: TrackId,
}

/// A reusable list item widget supporting custom widget content.
///
/// This is a transparent wrapper around [`ListItem`] that provides:
/// - All the same properties and usage patterns as [`ListItem`]
/// - The ability to use any widget as a child instead of a predefined structure
#[repr(transparent)]
pub struct ListCustomItem(Stateful<ListItem>);

/// Enum representing possible children of a List widget
#[derive(Template)]
pub enum ListChild<'c> {
  /// Standard list item with predefined structure
  StandardItem(PairOf<'c, ListItem>),
  /// Custom-designed list item
  CustomItem(PairOf<'c, ListCustomItem>),
  /// Visual divider between list items
  Divider(FatObj<State<Divider>>),
}

/// Defines the selection behavior for the List widget
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum ListSelectMode {
  /// No items can be selected (default)
  #[default]
  None,
  /// Only one item can be selected at a time
  Single,
  /// Multiple items can be selected
  Multi,
}

/// Theme provider for vertical alignment of list item widgets
#[derive(Default, Clone)]
pub struct ListItemAlignItems(pub Align);

// The following templates define structural widgets of list items:

/// Template for the primary headline text of a list item
#[derive(Template)]
pub struct ListItemHeadline(TextValue);

/// Template for supporting text with line clamping
#[derive(Template)]
pub struct ListItemSupporting {
  /// The number of visible text lines
  #[template(field = 1usize)]
  lines: PipeValue<usize>,
  text: TextValue,
}

/// Template for trailing supporting text
#[derive(Template)]
pub struct ListItemTrailingSupporting(TextValue);

/// Widget for images in list items
#[simple_declare]
pub struct ListItemImg;

/// Widget for thumbnail images in list items
#[simple_declare]
pub struct ListItemThumbnail;

/// The template for the [`ListItem`] widget, which includes the leading,
/// content, and trailing sections
#[derive(Template)]
pub struct ListItemChildren<'w> {
  leading: Option<Widget<'w>>,
  headline: ListItemHeadline,
  supporting: Option<ListItemSupporting>,
  trailing_supporting: Option<ListItemTrailingSupporting>,
  trailing: Option<Trailing<Widget<'w>>>,
}

/// Metadata about the structure of a list item, it provides for the children of
/// the `ListItem`.
pub struct ListItemStructInfo {
  pub supporting: bool,
  pub trailing_supporting: bool,
  pub leading: bool,
  pub trailing: bool,
}

impl ListItem {
  /// Returns true if the item is currently selected
  pub fn is_selected(&self) -> bool { self.selected }

  /// Marks the item as selected
  pub fn select(&mut self) { self.selected = true; }

  /// Marks the item as unselected
  pub fn deselect(&mut self) { self.selected = false; }

  /// Toggles the selection state
  pub fn toggle(&mut self) { self.selected = !self.selected; }

  /// Returns true if the item is interactive
  pub fn is_interactive(&self) -> bool { self.interactive }

  /// Sets the interactive state of the item
  pub fn set_interactive(&mut self, interactive: bool) {
    if self.interactive != interactive {
      self.interactive = interactive;
    }
  }

  /// Internal method to handle selection with toggle support
  fn select_action(mut this: WriteRef<Self>, mode: ListSelectMode) {
    if this.interactive {
      match mode {
        ListSelectMode::None => {}
        ListSelectMode::Single => this.toggle(),
        ListSelectMode::Multi => this.select(),
      }
    }
  }

  /// Generates classes based on the item's state
  fn item_classes(item: &impl StateWatcher<Value = Self>) -> [PipeValue<Option<ClassName>>; 3] {
    class_array![
      distinct_pipe! {
        if $item.is_selected() { LIST_ITEM_SELECTED } else { LIST_ITEM_UNSELECTED }
      },
      distinct_pipe! { $item.is_interactive().then_some(LIST_ITEM_INTERACTIVE) },
      LIST_ITEM
    ]
  }
}

pub struct ListCustomItemDeclarer(ListItemDeclarer);

impl<'c> ComposeChild<'c> for List {
  type Child = Vec<ListChild<'c>>;

  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
    List::collect_items(&this, &child);
    let select_mode = this.read().select_mode;

    self::column! {
      class: LIST,
      align_items: Align::Stretch,
      on_disposed: move |_| $this.write().clear(),
      on_key_down: move |e| {
        if select_mode != ListSelectMode::None {
          match e.key() {
            VirtualKey::Named(NamedKey::ArrowUp)  => $this.write().focus_prev_item(&e.window()),
            VirtualKey::Named(NamedKey::ArrowDown) => $this.write().focus_next_item(&e.window()),
            _ => {}
          }
        }
      },
      @ {
        child.into_iter().map(move |item| match item {
          ListChild::StandardItem(pair) => {
            let item = pair.parent().as_stateful().clone_writer();
            $this.item_select_actions(item, pair.into_fat_widget())
          },
          ListChild::CustomItem(pair) => {
            let item = pair.parent().read().0.clone_writer();
            $this.item_select_actions(item, pair.into_fat_widget())
          },
          ListChild::Divider(divider) => divider.into_widget(),
        })
      }
    }
    .into_widget()
  }
}

impl<'c> ComposeChild<'c> for ListItem {
  type Child = ListItemChildren<'c>;

  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
    let item_struct_info = child.struct_info();
    let item_classes = ListItem::item_classes(&this);

    providers! {
      providers: [Provider::new(item_struct_info)],
      @ $item_classes { @ { child.compose_sections() } }
    }
    .into_widget()
  }
}

impl<'c> ComposeChild<'c> for ListCustomItem {
  type Child = Widget<'c>;

  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
    let mut child = FatObj::new(child);
    let item_classes = ListItem::item_classes(&this.read().0);
    item_classes
      .with_child(stack! {
        fit: StackFit::Passthrough,
        @ $child {
          v_align: ListItemAlignItems::get_align(BuildCtx::get()).map(|v| v.into()),
        }
      })
      .into_widget()
  }
}

impl ListItemSupporting {
  fn into_widget(self) -> Widget<'static> {
    let Self { lines, text } = self;
    text_clamp! {
      class: LIST_ITEM_SUPPORTING,
      rows: lines.map(|v| { Some(v as f32) }),
      @Text { text }
    }
    .into_widget()
  }
}

impl<'c> ComposeChild<'c> for ListItemImg {
  type Child = Widget<'c>;

  fn compose_child(_: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
    class! { class: LIST_ITEM_IMG, @ { child } }.into_widget()
  }
}

impl<'c> ComposeChild<'c> for ListItemThumbnail {
  type Child = Widget<'c>;

  fn compose_child(_: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
    class! { class: LIST_ITEM_THUMBNAIL, @ { child } }.into_widget()
  }
}

impl ListItemAlignItems {
  pub fn get_align(ctx: &BuildCtx) -> PipeValue<Align> {
    Variant::<Self>::new_or_default(ctx)
      .map(|v| v.0)
      .r_into()
  }
}

impl<'w> ListItemChildren<'w> {
  pub fn compose_sections(self) -> Widget<'w> {
    let ListItemChildren { headline, supporting, trailing_supporting, leading, trailing } = self;
    let content = content_section(headline, supporting.map(ListItemSupporting::into_widget));

    let trailing_supporting = trailing_supporting.map(|s| {
      text! { class: LIST_ITEM_TRAILING_SUPPORTING, text: s.0 }
    });
    let leading_widget = leading.map(|l| {
      class! { class: LIST_ITEM_LEADING, @ { l } }
    });
    let trailing_widget = trailing.map(|t| {
      class! { class: LIST_ITEM_TRAILING, @ { t.unwrap() } }
    });

    row! {
      align_items: ListItemAlignItems::get_align(BuildCtx::get()),
      @ { leading_widget }
      @Expanded { defer_alloc: true, @ { content } }
      @ { trailing_supporting }
      @ { trailing_widget }
    }
    .into_widget()
  }

  fn struct_info(&self) -> ListItemStructInfo {
    ListItemStructInfo {
      supporting: self.supporting.is_some(),
      trailing_supporting: self.trailing_supporting.is_some(),
      leading: self.leading.is_some(),
      trailing: self.trailing.is_some(),
    }
  }
}

impl List {
  /// Returns an iterator over all selected items in the list
  ///
  /// # Returns
  ///
  /// An iterator yielding pairs of index and reference to the selected
  /// `ListItem`.
  pub fn selected_items(&self) -> impl DoubleEndedIterator<Item = (usize, &Stateful<ListItem>)> {
    self
      .items
      .iter()
      .enumerate()
      .filter(|(_, item)| item.read().is_selected())
  }

  /// Returns the active item's index and reference if available
  ///
  /// The active item is either:
  /// - Last focused item
  /// - Last selected item
  pub fn active_item(&self) -> Option<(usize, &Stateful<ListItem>)> {
    self.active_item.and_then(|active_idx| {
      self
        .items
        .get(active_idx)
        .map(|item| (active_idx, item))
    })
  }

  /// Deselects all items in the list.
  ///
  /// This method iterates over all items in the list and marks each one as
  /// unselected.
  pub fn deselect_all(&mut self) {
    self
      .items
      .iter()
      .for_each(|item| item.write().deselect());
    self.active_item = None;
  }

  /// Selects items according to the current [`ListSelectMode`]:
  /// - **`Single`**: Selects the first item
  /// - **`Multi`**: Selects all items
  /// - **`None`**: No action
  ///
  /// # Returns
  ///
  /// The number of items selected:
  /// - `0` in `None` mode
  /// - `1` in `Single` mode (if items exist)
  /// - Total item count in `Multi` mode
  pub fn select_all(&mut self) -> usize {
    let take_count = match self.select_mode {
      ListSelectMode::Single => 1,
      ListSelectMode::Multi => self.items.len(),
      ListSelectMode::None => return 0,
    };

    self
      .items
      .iter()
      .take(take_count)
      .for_each(|item| item.write().select());

    self.active_item = Some(0);

    take_count
  }

  fn focus_next_item(&mut self, wnd: &Window) {
    let len = self.items.len();
    let start = self.active_item.map_or(0, |idx| idx + 1);
    for i in 0..len {
      let idx = (start + i) % len;
      let Some(id) = self.items[idx].read().wid.get() else { break };
      if wnd
        .request_focus(id, FocusReason::Keyboard)
        .is_some()
      {
        self.active_item = Some(idx);
        break;
      }
    }
  }

  fn focus_prev_item(&mut self, wnd: &Window) {
    let len = self.items.len();
    let start = self.active_item.map_or(0, |idx| idx + len - 1);
    for i in 0..len {
      let idx = (start + len - i) % len;
      let Some(id) = self.items[idx].read().wid.get() else { break };
      if wnd
        .request_focus(id, FocusReason::Keyboard)
        .is_some()
      {
        self.active_item = Some(idx);
        break;
      }
    }
  }

  fn clear(&mut self) {
    self
      .items
      .iter()
      .for_each(|item| item.write().deselect());
    self.active_item = None;
    self
      .subscriptions
      .drain(..)
      .for_each(|u| u.unsubscribe());
  }

  fn collect_items<'c>(this: &impl StateWriter<Value = Self>, children: &Vec<ListChild<'c>>) {
    let mut list = this.write();
    let List { items, subscriptions, .. } = &mut *list;
    children.iter().for_each(|child| match child {
      ListChild::StandardItem(pair) => {
        let item = pair.parent().as_stateful().clone_writer();
        items.push(item.clone_writer());
      }
      ListChild::CustomItem(pair) => {
        let item = pair.parent().read().0.clone_writer();
        items.push(item.clone_writer());
      }
      ListChild::Divider(_) => {}
    });

    items.iter().enumerate().for_each(|(idx, item)| {
      let item = item.clone_writer();
      let this = this.clone_writer();
      let u = watch!($item.is_selected())
        .distinct_until_changed()
        .filter(|selected| *selected)
        .subscribe(move |_| this.write().on_item_select(idx));
      subscriptions.push(BoxSubscription::new(u));
    });
  }

  fn on_item_select(&mut self, idx: usize) {
    self.active_item = Some(idx);
    if self.select_mode == ListSelectMode::Single {
      for (i, item) in self.items.iter().enumerate() {
        if i != idx && item.read().is_selected() {
          item.write().deselect();
        }
      }
    }
  }

  fn item_select_actions<'c>(
    &self, item: Stateful<ListItem>, mut list_item: FatObj<Widget<'c>>,
  ) -> Widget<'c> {
    item.silent().wid = list_item.get_track_id_widget().read().track_id();

    let mode = self.select_mode;
    if mode == ListSelectMode::None {
      list_item.into_widget()
    } else {
      rdl! {
        @ $list_item {
          on_tap: move |_| ListItem::select_action($item.write(), mode),
          on_key_down: move |e| {
            if matches!(e.key(), VirtualKey::Named(NamedKey::Enter)
              | VirtualKey::Named(NamedKey::Space)) {
              ListItem::select_action($item.write(), mode)
            }
          }
        }.into_widget()
      }
    }
  }
}

fn content_section(headline: ListItemHeadline, supporting: Option<Widget>) -> Widget {
  let headline = text! { class: LIST_ITEM_HEADLINE, text: headline.0 };
  if let Some(supporting) = supporting {
    self::column! {
      class: LIST_ITEM_CONTENT,
      align_items: Align::Stretch,
      @ { headline }
      @ { supporting }
    }
    .into_widget()
  } else {
    class! {
      class: LIST_ITEM_CONTENT,
      @ { headline }
    }
    .into_widget()
  }
}

impl Declare for ListCustomItem {
  type Builder = ListCustomItemDeclarer;

  #[inline]
  fn declarer() -> Self::Builder { ListCustomItemDeclarer(ListItem::declarer()) }
}

impl ObjDeclarer for ListCustomItemDeclarer {
  type Target = FatObj<ListCustomItem>;

  fn finish(self) -> Self::Target {
    let item = self.0.finish();
    item.map(|item| ListCustomItem(item.as_stateful().clone_writer()))
  }
}

impl std::ops::Deref for ListCustomItemDeclarer {
  type Target = ListItemDeclarer;

  fn deref(&self) -> &Self::Target { &self.0 }
}

impl std::ops::DerefMut for ListCustomItemDeclarer {
  fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}

#[cfg(test)]
mod tests {
  use ribir_core::{prelude::*, test_helper::*};
  use ribir_dev_helper::widget_image_tests;

  use super::*;

  widget_image_tests! {
   list,
   WidgetTester::new(list!{
     @ListItem {
       @Icon { @named_svgs::default() }
       @ListItemHeadline { @ { "Icon"} }
       @ListItemSupporting { @ { "description"} }
       @ListItemTrailingSupporting { @ { "100+"} }
     }
     @ListItem {
       disabled: true,
       @Icon { @named_svgs::default() }
       @ListItemHeadline { @ { "Only Headline"} }
       @Trailing { @Icon { @named_svgs::default() } }
     }
     @ListCustomItem { @Text { text: "Custom Item" } }
     @ListItem {
       @Icon { @named_svgs::default() }
       @ListItemHeadline { @ { "Only Headline"} }
       @ListItemTrailingSupporting { @ { "100+"} }
       @Trailing { @Icon { @named_svgs::default() } }
     }
     @ListItem {
       @Avatar { @ { "A" } }
       @ListItemHeadline { @ { "Avatar"} }
       @ListItemSupporting { @ { "description"} }
       @Trailing { @Icon { @named_svgs::default() } }
     }
     @ListItem {
       @ListItemImg {
         @Container { size: Size::new(100., 100.), background: Color::PINK }
       }
       @ListItemHeadline { @ { "Image Item"} }
       @ListItemSupporting { @ { "description"} }
     }
     @ListItem {
       @ListItemThumbnail {
         @Container { size: Size::new(160., 90.), background: Color::GREEN }
       }
       @ListItemHeadline { @ { "Counter"} }
       @ListItemSupporting {
         lines: 2usize,
         @ { "there is supporting lines, many lines, wrap to multiple lines, xxhadkasda"}
       }
       @ListItemTrailingSupporting { @ { "100+" } }
       @Trailing { @Icon { @named_svgs::default() } }
     }

     @ListItem {
      @ListItemHeadline { @ { "Counter"} }
      @Trailing {
        @ListItemThumbnail {
          @Container { size: Size::new(160., 90.), background: Color::GREEN }
        }
      }
    }
   }).with_wnd_size(Size::new(320., 640.))
   .with_comparison(0.00005)
  }
}
