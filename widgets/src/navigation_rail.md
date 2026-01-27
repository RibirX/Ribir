# NavigationRail Design Specification

`NavigationRail` is a side navigation widget designed for medium and large screens, providing efficient access to top-level navigation and dynamically adjusting layout based on available space. This design strictly follows the **Ribir Interactive Widget Design Standard (v2.4)**.

---

## 1. Core Principles

### 1.1 Composition First

NavigationRail is a **pure selection container**:

- **Widget responsibility**: Manage selection state, provide visual feedback, trigger selection events
- **Application responsibility**: Handle side effects in `on_select` event (route navigation, permission checks, etc.)
- **No built-in routing**: Routing is an application-level concern

### 1.2 Unidirectional Data Flow

1. **State lifting**: Selection state is managed by NavigationRail
2. **Event-driven**: User interactions trigger `RailSelect` events without directly modifying UI
3. **Data-driven**: Application updates data → Pipe emits → UI updates (Path A)

### 1.3 Identifier Strategy

**Key specification**: Each `RailItem` can optionally specify a `key` for stable identification.

- **If `key` is provided**: Used directly for selection matching
- **If `key` is omitted**: NavigationRail automatically uses the item's index (`"0"`, `"1"`, `"2"`, ...) as the key

**Runtime guarantee**: After `ComposeChild`, all RailItems have a valid key (either user-provided or auto-generated index).

**`key` vs `reuse_id`**:

| Attribute | Layer | Purpose |
|-----------|-------|---------|
| `key` | Business | Selection state matching |
| `reuse_id` | Framework | Widget instance reuse |

They are completely independent and can differ.

---

## 2. Interaction Model

### 2.1 Controlled Protocol

| Mode | DSL | Behavior |
|------|-----|----------|
| **Controlled** | `selected: pipe!($model.key)` | UI follows Pipe, requires manual model updates |
| **Two-way binding** | `selected: TwoWay::new(model.key)` | Auto-syncs UI ↔ data |
| **Uncontrolled** | `selected: Some("home")` | Widget manages internally |

**Limitation**: Action Items **cannot use TwoWay** (see Section 4.4).

### 2.2 Event Definition

```rust
/// Navigation item selection event
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RailSelect {
  pub from: Option<String>,  // Previously selected identifier (for animation direction, history)
  pub to: String,            // Newly selected identifier
}

pub type RailSelectEvent = CustomEvent<RailSelect>;
```

---

## 3. Layout and Adaptation

### 3.1 Layout Modes

| Mode | Width | Layout |
|------|-------|--------|
| **Collapsed** | 80dp | Vertical stack, centered icons |
| **Expanded** | 240-320dp | Horizontal layout, icons and labels side by side |

### 3.2 Static and Environment Configuration (via Provider)

**Layout and Label display strategy**:

| Configuration | Provider | Behavior |
|---------------|----------|----------|
| **Expanded state** | `RailExpanded` | `true`: wide mode, `false`: narrow mode (default) |
| **Label policy** | `RailLabelPolicy` | `None` (default), `OnSelected`, `Always` |
| **Content alignment** | `RailContentAlign` | `Align::Center` (default), `Start`, `End` |

**Why use Provider**:
- **Dynamic selection** (`selected`) → Widget property (core business state)
- **Visual context** (`expanded`, `label_policy`, etc.) → Provider (environment configuration)

### 3.3 Selection State Rendering

Pass selection state via **class name**:

```rust
// NavigationRail internal
let class = if is_selected {
  class_names![RAIL_ITEM, RAIL_ITEM_SELECTED]
} else {
  class_names![RAIL_ITEM, RAIL_ITEM_UNSELECTED]
};
```

Theme system defines corresponding styles.

### 3.4 Section Adaptation

`RailSection` automatically switches based on `RailExpanded` state:
- **Expanded**: Text title
- **Collapsed**: `Divider` separator

---

## 4. Usage Examples

### 4.1 Basic Usage

```rust
navigation_rail! {
  selected: TwoWay::new(app.current_section),
  
  // Without explicit key: auto-generated index is used
  @RailItem { @{ svg_registry::HOME }, @{ "Home" } }      // key = "0"
  
  // With explicit key: stable identification
  @RailItem { key: "profile", @{ svg_registry::PROFILE }, @{ "Profile" } }  // key = "profile"
}

@match $app.current_section.as_deref() {
  Some("0") => @HomePage,
  Some("profile") => @ProfilePage,
  _ => @Void,
}
```

### 4.2 Router Integration

```rust
navigation_rail! {
  selected: pipe!({
    match Location::of(ctx).path() {
      "/" => Some("home".to_string()),
      "/profile" => Some("profile".to_string()),
      _ => None,
    }
  }),
  
  on_select: move |e| {
    let route = match e.value().to.as_str() {
      "home" => "/",
      "profile" => "/profile",
      _ => return,
    };
    Location::of(ctx).write().navigate(route);
  },
  
  @RailItem { key: "home", @{ svg_registry::HOME }, @{ "Home" } }
  @RailItem { key: "profile", @{ svg_registry::PROFILE }, @{ "Profile" } }
}
```

### 4.3 Action Items (Non-navigation)

**⚠️ Important**: Action Items **cannot use TwoWay**, must use controlled mode.

```rust
navigation_rail! {
  selected: pipe!($app.current_view),  // ✅ Controlled mode
  
  on_select: move |e| {
    match e.value().to.as_str() {
      "logout" => app.logout(),  // Action: don't update selected
      "create" => show_create_dialog(),  // Action: don't update selected
      _ => {
        $write(app).current_view = Some(e.value().to.clone());  // Navigation: manually update
      }
    }
  },
  
  @RailItem { key: "home", @{ svg_registry::HOME }, @{ "Home" } }
  @RailSection { @{ "Actions" } }
  @RailItem { key: "logout", @{ svg_registry::LOGOUT }, @{ "Logout" } }
}
```

---

## 5. Type Definitions

### 5.1 Configuration and State

```rust
/// Label display strategy in collapsed mode
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum RailLabelPolicy {
  #[default]
  None,        // Icon only
  OnSelected,  // Label only on selected item
  Always,      // Label on all items
}

/// Global expanded state Provider
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub struct RailExpanded(pub bool);

/// Label display strategy Provider
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub struct RailLabelPolicy(pub RailLabelPolicy);

/// Content alignment Provider
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub struct RailContentAlign(pub Align);

/// RailItem structure metadata Provider
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct RailItemMetadata {
  pub has_label: bool,
  pub has_badge: bool,
}
```

### 5.2 Event Types

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RailSelect {
  pub from: Option<String>,
  pub to: String,
}

pub type RailSelectEvent = CustomEvent<RailSelect>;
```

### 5.3 Style Classes

```rust
class_names! {
  NAVIGATION_RAIL,
  RAIL_MENU,
  RAIL_ACTION,
  RAIL_CONTENT,
  RAIL_FOOTER,
  RAIL_ITEM,
  RAIL_ITEM_SELECTED,
  RAIL_ITEM_UNSELECTED,
  RAIL_ITEM_ICON,
  RAIL_ITEM_LABEL,
  RAIL_ITEM_INDICATOR,
  RAIL_ITEM_BADGE,
  RAIL_SECTION,
  RAIL_SECTION_TITLE,
}
```

### 5.4 Widget Definitions

```rust
/// Navigation item
#[declare]
pub struct RailItem {
  /// Business identifier
  /// - User-provided: used directly
  /// - User-omitted: NavigationRail auto-supplements index string
  /// - Runtime guarantee: always `Some` after ComposeChild
  #[declare(default)]
  pub key: Option<String>,
}

/// NavigationRail main widget
#[declare]
pub struct NavigationRail {
  /// Currently selected item identifier
  #[declare(default, event = RailSelectEvent)]
  pub selected: Option<String>,
  
  /// Internal navigation item list
  #[declare(skip)]
  items: Vec<String>,
}

impl NavigationRail {
  /// Get all navigation item keys
  pub fn keys(&self) -> &[String];
  
  /// Calculate next key (non-cyclic)
  /// 
  /// **Behavior**:
  /// - Valid selection and not last: return next
  /// - No selection or invalid: return first
  /// - Last item: return None
  /// - Empty list: return None
  pub fn next_key(&self) -> Option<&str>;
  
  /// Calculate previous key (non-cyclic)
  /// 
  /// **Behavior**:
  /// - Valid selection and not first: return previous
  /// - No selection or invalid: return last
  /// - First item: return None
  /// - Empty list: return None
  pub fn prev_key(&self) -> Option<&str>;
  
  /// Calculate next key (cyclic)
  pub fn next_key_cyclic(&self) -> Option<&str>;
  
  /// Calculate previous key (cyclic)
  pub fn prev_key_cyclic(&self) -> Option<&str>;
}
```

### 5.5 Templates and Auxiliary Types

```rust
#[derive(Template)]
pub struct RailMenu(pub Widget<'static>);

#[derive(Template)]
pub struct RailAction(pub Widget<'static>);

#[derive(Template)]
pub struct RailFooter(pub Widget<'static>);

#[derive(Template)]
pub struct RailSection(TextValue);

#[derive(Template)]
pub enum RailBadge {
  Badge(FatObj<Stateful<Badge>>),
  NumBadge(FatObj<Stateful<NumBadge>>),
}

#[derive(Template)]
pub struct RailItemChildren<'c> {
  pub icon: Widget<'c>,
  pub label: Option<TextValue>,
  pub badge: Option<RailBadge>,
}


#[derive(Template)]
pub struct RailItemChildren {
  pub icon: Widget<'static>,
  pub label: Option<TextValue>,
  pub badge: Option<RailBadge<'static>>,
}
```


---

## 6. Design Decisions

### 6.1 Why no built-in routing?

Following the **composition first** principle: the widget focuses on selection mechanism, routing is an application-level concern. Users can implement arbitrary logic in `on_select` (permissions, confirmations, analytics, etc.) and compose with any routing solution.

### 6.2 Why does the event include `from` and `to`?

- Animation direction: `from → to` determines upward/downward slide
- History tracking: know where the user came from
- Event self-containment: no need to access external state

### 6.3 Why `Option<String>` instead of `i32` index?

- Stability: `key` is unaffected by list order changes
- Semantic: `selected: Some("settings")` is more intuitive than `selected: 2`
- Serialization-friendly: strings can be directly serialized
- Backward compatible: indices can be converted to strings (`"0"`, `"1"`)

### 6.4 Why provide read-only query methods instead of direct mutation methods?

Directly providing `select_next()` would cause data-UI separation (violating Path A). Read-only query methods let the application layer update the data model, and Pipe automatically syncs the UI, conforming to Ribir's data flow specification.

### 6.5 Why use Provider for configuration?

- **Core state** (changes frequently with user business interaction) → Widget properties (`selected`)
- **Environment configuration** (rarely changes or controlled by external layout) → Provider (`expanded`, `label_policy`)

Benefits:
1. **Clean API**: Reduces the number of properties on the widget itself.
2. **Global Control**: Multiple rails can share the same expanded state if needed.
3. **Responsive ease**: External layout managers can inject the `RailExpanded` state without needing direct access to the widget instance.

---

## 7. Theme and Styling

### 7.1 Class-Based Styling

Defines comprehensive `class_names!` ensuring all visual details (spacing, colors, animations) can be overridden by the theme layer.

### 7.2 Structural Metadata

`RailItem` exposes internal structure information (whether it has Label, whether it has Badge) via `RailItemMetadata` Provider, allowing the theme system to implement pixel-perfect alignment and conditional styling.

---

**Status**: ✅ Finalized
