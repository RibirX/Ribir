---
sidebar_position: 1
---

# Widget Components

Widgets are the building blocks of Ribir applications. In Ribir, everything is a Widget, from the most basic text nodes to complex layout containers, to complete applications - all are built by composing different Widgets.

This document introduces the core and common Widgets provided by Ribir to help you quickly build user interfaces. Before reading this document, it is recommended to first understand the concepts of [Declarative UI](../core_concepts/declarative_ui.md) and [Built-in Attributes](../core_concepts/built_in_attributes_and_fat_obj.md).

## Widget Categories

In Ribir, Widgets are mainly divided into two categories (for details, see [Custom Widgets](./custom_widgets.md)):

1.  **Compose Widgets**: Build UI by composing other Widgets. This is the most common Widget type, such as `Button`, `List`, etc.
2.  **Render Widgets**: Responsible for specific layout and drawing logic. For example, `Text`, `Container`, etc.

Regardless of the type, you can use the unified `fn_widget!` macro and `@` syntax to declare and use them.

## Common Capabilities (FatObj)

All Widgets gain a set of common built-in attributes through the `FatObj` mechanism. This means you can directly use layout (like `margin`), visual (like `background`, `border`), and interaction (like `on_tap`) attributes on any Widget, without the Widget itself explicitly implementing these features.

For a detailed list, see [Built-in Attributes](../core_concepts/built_in_attributes_and_fat_obj.md).

```rust no_run
use ribir::prelude::*;

fn common_props_example() -> Widget<'static> {
    fn_widget! {
        @Text {
            text: "I am a text with background and margin",
            // Built-in attributes used directly
            margin: EdgeInsets::all(10.),
            background: Color::YELLOW,
            padding: EdgeInsets::all(5.),
            border: Border::all(BorderSide::new(1., Color::BLACK.into())),
        }
    }.into_widget()
}
```

## Core Widgets

These widgets are provided by `ribir_core` and form the basic skeleton of the UI.

### Basic Display

*   **`Text`**: Used to display text. Supports rich text styles, alignment, etc.
    *   Properties: `text` (content), `text_style` (style), `text_align` (alignment).
    *   **Typography Widgets**: `H1`, `H2`, `H3`, `H4`, `H5`, `H6` are `Text` variants with preset theme styles.

### Layout Containers

*   **`Container`**: The most commonly used box model container. Can set size, background, border, radius, etc. Although these can also be set through built-in attributes, explicitly using `Container` is clearer when combining them (such as needing both specific size and background).
*   **`SizedBox`**: A box that enforces a fixed size. Usually used for placeholders or forcing child element sizes.
*   **`ConstrainedBox`**: Applies additional layout constraints to child elements (such as max/min width/height).
*   **`UnconstrainedBox`**: Removes certain constraints from the parent on child elements, allowing children to draw at their own size.

### Linear Layout

*   **`Row`**: Arranges children horizontally.
*   **`Column`**: Arranges children vertically.
*   **`Flex`**: A more general flexible layout container, both `Row` and `Column` are wrappers based on it.
    *   Key properties: `align_items` (cross-axis alignment), `justify_content` (main-axis alignment), `wrap` (whether to wrap).

### Flex Control

*   **`Expanded`**: Used in `Row`, `Column`, or `Flex` to force child elements to fill remaining space.

### Stack Layout

*   **`Stack`**: Allows child elements to be placed overlapping. Usually used with `Positioned` (implemented through built-in attributes `anchor` / `global_anchor`).

### Transforms & Effects

*   **`TransformWidget`**: Applies matrix transformations (translation, rotation, scaling) to child elements. Usually used through the built-in `transform` attribute.
*   **`Opacity`**: Sets the opacity of child elements. Usually used through the built-in `opacity` attribute.
*   **`Clip`**: Clips the content of child elements. Has variants like `ClipRect`, `ClipRRect`, `ClipPath`.

## Common Widgets

These widgets are located in the `ribir_widgets` library and provide rich high-level UI controls.

### Buttons

Ribir provides a series of buttons that conform to common design specifications:

*   **`Button`** (or `OutlinedButton`): Button with border, used for secondary actions.
*   **`FilledButton`**: Button with filled background color, used for primary actions.
*   **`TextButton`**: Plain text button, used for low-emphasis actions.
*   **`Fab`**: Floating Action Button.

All buttons support flexible content composition and can contain only text, only icons, or both.

```rust no_run
use ribir::prelude::*;

fn buttons_example() -> Widget<'static> {
    fn_widget! {
        @Row {
            @FilledButton { @{ "Primary Action" } }
            @Button { @{ "Secondary Action" } }
            @TextButton { @{ "Cancel" } }
            // Button with icon
            @FilledButton {
                @Icon { @ { svg_registry::get_or_default("add") } }
                @ { "New" }
            }
        }
    }.into_widget()
}
```

### Input

*   **`Input`**: Basic text input box.
*   **`Checkbox`**: Checkbox.
*   **`Switch`**: Switch control.
*   **`Radio`**: Radio button.
*   **`Slider`**: Slider for selecting numeric ranges.

### Lists

*   **`List`**: Vertical list container, supports single-select and multi-select modes.
*   **`ListItem`**: Standard list item, includes leading icon (Leading), main title (Headline), subtitle (Supporting), and trailing control (Trailing).
*   **`Divider`**: Divider line.

```rust no_run
use ribir::prelude::*;

fn list_example() -> Widget<'static> {
    fn_widget! {
        @List {
            @ListItem {
                @ListItemHeadline { @{ "List Item 1" } }
                @ListItemSupporting { @{ "This is description information" } }
            }
            @Divider {}
            @ListItem {
                @ListItemHeadline { @{ "List Item 2" } }
                @Trailing { @Switch { checked: true } }
            }
        }
    }.into_widget()
}
```

### Navigation & Menus

*   **`Tabs`**: Tab switching component.
*   **`Menu`**: Popup menu.

### Display

*   **`Icon`**: Icon component, usually used with SVG.
*   **`Avatar`**: Avatar component, supports images or characters.
*   **`Badge`**: Badge, usually attached to the corner of other Widgets to display notification counts.
*   **`Progress`**: Progress bar (linear or circular).

### Scroll

*   **`Scrollable`**: Provides scrolling capability for child content. Usually you can directly use the built-in attribute `scrollable: Scrollable::X` or `Scrollable::Y` on any Widget to enable it.
*   **`Scrollbar`**: Explicit scrollbar component.

## Summary

Ribir provides a rich component library. Combined with powerful composition capabilities (`Compose`) and a universal attribute system (`FatObj`), you can efficiently build complex and beautiful user interfaces.

*   For basic layout and drawing, mainly use Widgets from `ribir_core`.
*   For common interactive controls, first check if there is an implementation in `ribir_widgets`.
*   If existing components cannot meet your needs, you can easily compose existing components through `fn_widget!`, or implement the `Render` trait to create completely new custom components.
