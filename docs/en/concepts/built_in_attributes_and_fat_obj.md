---
sidebar_position: 3
---

# Built-in Attributes & FatObj

Ribir provides a powerful built-in attribute system that allows you to add common functionality to any Widget, such as layout control (margin, alignment), visual effects (background, border, opacity, transform), and interaction events (on_tap, on_hover). These features are not implemented individually by each Widget, but are provided through a universal wrapper called `FatObj`.

## The `@` Instantiation Process

When you use the `@` syntax (e.g., `@Text { ... }`) in `fn_widget!`, Ribir performs the following steps to construct the widget:

1.  **Get the Builder**: It calls the `declarer()` method from the `Declare` trait to obtain the builder for the widget.
2.  **Initialize Fields**: For each field specified in the `{ ... }` block, it calls the corresponding `with_xxx()` method on the builder (e.g., `with_text(...)`).
3.  **Finish Construction**: Finally, it calls the builder's `finish()` method (builder implements `ObjDeclarer` trait), to complete the construction and return the declared widget.

### `#[declare]` Options

The `#[declare]` macro supports several options to customize its behavior:

- **Default**: Generates a full builder that returns `FatObj<Stateful<T>>`, enabling all built-in attributes and reactive state.
- **`#[declare(stateless)]`**: Generates a full builder that returns `FatObj<T>`. Supports built-in attributes, but the widget itself is not stateful.
- **`#[declare(simple)]`**: Generates a simplified builder that returns `Stateful<T>` (or `T` if it has no fields). This is suitable for widgets that don't need built-in attributes.
- **`#[declare(simple, stateless)]`**: Similar to `simple`, but always returns the plain object `T`.
- **`#[declare(eager)]`**: Generates a builder that constructs the widget eagerly (immediately upon setting fields). This allows complex fields to be initialized partially (e.g., setting `width` and `height` separately for a `Size` field). It can be combined with `simple` and `stateless`.
- **`#[declare(validate)]`**: Enables the `declare_validate` method for the widget.

> [!NOTE]
> `#[simple_declare]` is now deprecated in favor of `#[declare(simple)]`.

### Eager Mode: Partial Initialization of Complex Fields

The `#[declare(eager)]` mode enables **partial initialization of complex fields**. In lazy mode, the widget is only constructed when `finish()` is called. In eager mode, the widget is created immediately with default values, allowing custom setters to modify parts of complex fields.

**Example: Supporting `width` and `height` for a `Size` field**

```rust no_run
use ribir::prelude::*;

#[derive(Default)]
#[declare(eager)]
struct SizedBox {
    #[declare(default)]
    size: Size,
}

impl Render for SizedBox {
    fn measure(&self, clamp: BoxClamp, _: &mut MeasureCtx) -> Size { clamp.clamp(self.size) }

    fn paint(&self, _: &mut PaintingCtx) {}
}

impl SizedBoxDeclarer {
    /// Sets width, supporting both values and pipes
    fn with_width<K: ?Sized>(&mut self, width: impl RInto<PipeValue<f32>, K>) -> &mut Self {
        let host = self.host().clone_writer();
        let mix = self.mix_builtin_widget();
        mix.init_sub_widget(width, &host, |w: &mut SizedBox, v| w.size.width = v);
        self
    }

    /// Sets height, supporting both values and pipes
    fn with_height<K: ?Sized>(&mut self, height: impl RInto<PipeValue<f32>, K>) -> &mut Self {
        let host = self.host().clone_writer();
        let mix = self.mix_builtin_widget();
        mix.init_sub_widget(height, &host, |w: &mut SizedBox, v| w.size.height = v);
        self
    }
}

// Usage:
fn example() -> Widget<'static> {
    let w = Stateful::new(100.0f32);
    fn_widget! {
        @SizedBox {
            width: pipe!(*$read(w)),  // or just: width: 100.0
            height: 50.0,
        }
    }.into_widget()
}
```

Key points:
- `host()` provides access to the stateful widget during declaration
- `RInto<PipeValue<T>, K>` accepts both plain values and reactive pipes
- `init_sub_widget` handles pipe subscription and cleanup automatically

## What is FatObj?

`FatObj<T>` is a generic struct in Ribir's core library. Its purpose is to temporarily wrap a Widget during the build phase and attach various built-in attributes to it, such as `margin`, `background`, `on_tap`, etc.

### How It Works

1.  **Lazy Initialization**: `FatObj` internally maintains the state of all built-in attributes (like `margin`, `padding`, etc.), but they default to empty. Only when you explicitly use an attribute is the related widget initialized. This ensures that unused features don't bring additional performance overhead.
2.  **Compose**: In the final stage of Widget construction, `FatObj` composes the Widget it wraps with the enabled built-in features (like `Padding`, `Container`, `MixBuiltin`, etc.) into the final Widget tree.

Example: `Margin(MixBuiltin(Text))`



## Common Built-in Attributes

Built-in attributes are mainly divided into two categories: **Properties** and **Events**.

### 1. Properties

These attributes are used to control the appearance and layout of Widgets.

*   **Layout**:
    *   `margin`: Sets outer margin.
    *   `padding`: Sets inner padding.
    *   `x` / `y`: Sets horizontal/vertical positioning using `PosX` and `PosY`.
    *   `clamp`: Forces constraints on the Widget's size range (Layout Constraints).
    *   `box_fit`: Controls how child elements fit into container space (like fill, contain, etc.).
    *   `scrollable`: Controls the Widget's scrolling behavior (X-axis, Y-axis, or both).
    *   `layout_box`: Controls layout box behavior.

*   **Visual**:
    *   `background`: Sets background (color or image).
    *   `foreground`: Sets foreground (usually overlays on top of content).
    *   `border`: Sets border.
    *   `box_shadow`: Sets box shadow (outer shadow effect around the box).
    *   `radius`: Sets border radius.
    *   `backdrop`: Sets backdrop (background effect).
    *   `opacity`: Sets opacity.
    *   `visible`: Controls visibility.
    *   `transform`: Applies graphic transformations (translation, rotation, scaling).
    *   `cursor`: Sets cursor style when hovering.
    *   `backdrop_filter`: Applies background filter effects (like blur).
    *   `filter`: Applies visual filter effects (blur, grayscale, brightness, etc.).
    *   `clip_boundary`: Whether to clip content beyond boundaries.
    *   `painting_style`: Sets painting style (fill or stroke).

*   **Text** (usually inherited by child nodes):
    *   `text_style`: Sets font style.
    *   `text_align`: Sets text alignment.
    *   `text_line_height`: Sets line height.
    *   `font_size`: Sets font size.
    *   `font_face`: Sets font family.

*   **Other**:
    *   `keep_alive`: Keeps Widget state even when removed from view.
    *   `tooltips`: Sets tooltip text.
    *   `disabled`: Disables interaction for Widget and its children.
    *   `providers`: Sets provider context for the widget.
    *   `class`: Applies style classes.
    *   `reuse`: Reuse Widget by setting the reuse attribute. For local reuse, can be used with (LocalWidgets).

### 2. Events

These attributes are used to handle user interactions. All event callbacks receive an event object.

*   **Pointer Events**:
    *   `on_pointer_down`: Triggered when a pointer (mouse button, touch contact, pen) is pressed.
    *   `on_pointer_move`: Triggered when a pointer moves.
    *   `on_pointer_up`: Triggered when a pointer is released.
    *   `on_pointer_cancel`: Triggered when a pointer event is cancelled (e.g., touch interruption).
    *   `on_pointer_enter`: Triggered when a pointer enters the widget's area.
    *   `on_pointer_leave`: Triggered when a pointer leaves the widget's area.
    *   `on_tap`: Triggered on a click or tap (press and release sequence).
    *   `on_tap_capture`: Capture phase version of `on_tap`.
    *   `on_double_tap`: Triggered on a double click/tap.
    *   `on_triple_tap`: Triggered on a triple click/tap.
    *   `on_x_times_tap`: Triggered on a specific number of taps.

*   **Wheel Events**:
    *   `on_wheel`: Triggered when the mouse wheel is scrolled.
    *   `on_wheel_capture`: Capture phase version of `on_wheel`.
    *   `on_wheel_changed`: Triggered when the wheel delta changes.

*   **Keyboard Events**:
    *   `on_key_down`: Triggered when a key is pressed.
    *   `on_key_down_capture`: Capture phase version of `on_key_down`.
    *   `on_key_up`: Triggered when a key is released.
    *   `on_key_up_capture`: Capture phase version of `on_key_up`.

*   **Focus Events**:
    *   `on_focus`: Triggered when the widget gains focus.
    *   `on_blur`: Triggered when the widget loses focus.
    *   `on_focus_in`: Triggered when the widget or one of its descendants gains focus (bubbles).
    *   `on_focus_out`: Triggered when the widget or one of its descendants loses focus (bubbles).

*   **Lifecycle Events**:
    *   `on_mounted`: Triggered when the widget is mounted to the widget tree.
    *   `on_performed_layout`: Triggered after the widget has been laid out.
    *   `on_disposed`: Triggered when the widget is removed from the widget tree.

*   **IME Events**:
    *   `on_ime_pre_edit`: Triggered during IME pre-edit (e.g., composing text).
    *   `on_chars`: Triggered when text characters are received.

## Usage Scenarios

### Scenario 1: Declaring a New Widget

In most cases, widgets are defined with the `#[declare]` macro. This means you can directly use built-in attributes when declaring a widget using the `@` syntax.

For example, the `Text` widget itself does not contain `margin` or `background` fields, but through the `#[declare]` and `FatObj` mechanism, you can use them directly during declaration:

```rust no_run
use ribir::prelude::*;

fn simple_card_traditional() -> Widget<'static> {
    fn_widget! {
        @Text {
            text: "Hello, Ribir!",
            // Built-in attributes: Layout
            margin: EdgeInsets::all(10.),
            padding: EdgeInsets::symmetrical(10., 5.),
            x: AnchorX::center(),

            // Built-in attributes: Visual
            background: Color::from_u32(0xFFEEAA00),
            border: Border::all(BorderSide::new(2., Color::BLACK.into())),
            radius: Radius::all(4.),

            // Built-in attributes: Interaction
            on_tap: |_: &mut PointerEvent| println!("Card Tapped!"),
            cursor: CursorIcon::Pointer,
        }
    }.into_widget()
}
```

### Scenario 2: Wrapping an Existing Widget

When you need to add built-in attributes to an already constructed Widget instance (e.g., a widget passed as a function argument, or a widget in a variable), you can use the `@FatObj { ... }` syntax.

```rust no_run
use ribir::prelude::*;

fn simple_card(w: Widget<'static>) -> Widget<'static> {
    fn_widget! {
        // Wrap the widget with FatObj to add built-in attributes
        @FatObj {
            margin: EdgeInsets::all(10.),
            padding: EdgeInsets::symmetrical(10., 5.),
            x: AnchorX::center(),
            background: Color::from_u32(0xFFEEAA00),
            border: Border::all(BorderSide::new(2., Color::BLACK.into())),
            radius: Radius::all(4.),
            on_tap: |_: &mut PointerEvent| println!("Card Tapped!"),
            cursor: CursorIcon::Pointer,
            // Embed the child widget
            @ { w }
        }
    }.into_widget()
}
```

This approach is very clear and idiomatic. It is recommended to use `@FatObj { ... }` instead of manually creating `FatObj::new(w)`.

## FatObj Core Mechanics

### Inner Wrapping Order of Built-in Attributes

`FatObj` wraps built-in attributes in a fixed order. This order determines the structure of the final widget tree and how attributes interact with each other.

The wrapping order from **inner to outer** is as follows (simplified for common attributes):

1.  **Content** (The widget being wrapped)
2.  `padding`
3.  `foreground`
4.  `border`
5.  `background`
6.  `backdrop`
7.  `filter`
8.  `clip_boundary`
9.  `box_shadow`
10. `radius`
11. `scrollable`
12. `layout_box`
13. `providers`
14. `class`
15. `clamp` (constrained_box)
16. `tooltips`
17. `margin`
18. `cursor`
19. **Events** (`mix_builtin`: `on_tap`, `on_pointer_move`, etc.)
20. `transform`
21. `opacity`
22. `visibility`
23. `disabled`
24. `x` / `y` (position)
25. `keep_alive`
26. `reuse`

#### Key Takeaways

Because wrapping has a fixed order, attributes wrapped in outer layers will affect the scope of attributes in inner layers. If you set multiple built-in attributes and find that the effect does not meet expectations, you can try adjusting the order of the attributes.

*   **Events include Margin**: Since **Events** wrap **Margin**, the interactive area of a widget includes its margin by default.
*   **Transform affects everything**: `transform` wraps most visual and layout attributes, so rotating a widget rotates its margin, background, and border as well.
*   **Visibility hides everything**: `visibility` is near the outermost layer, so setting it to hidden hides the entire widget including its margin.

### How to Override the Order?

Sometimes the default wrapping order doesn't match your requirements. For example, you might want the click area (`on_tap`) to **exclude** the margin.

Since `FatObj` applies attributes in a fixed order, you can achieve this by manually nesting `FatObj`. You can apply the inner attributes first, and then wrap it with another `FatObj` for the outer attributes.

**Example: Click area excluding margin**

If you simply write:
```rust ignore
@FatObj {
    margin: EdgeInsets::all(20.),
    on_tap: |_| println!("Clicked!"),
    @ { w }
}
```
The structure is `MixBuiltin(Margin(w))`, so clicking the margin triggers the event.

To exclude the margin from the click area, you want the structure `Margin(MixBuiltin(w))`. You can do this by:

```rust ignore
fn_widget! {
    // Outer FatObj handles margin
    @FatObj {
        margin: EdgeInsets::all(20.),
        // Inner FatObj handles the click event
        @FatObj {
            on_tap: |_| println!("Clicked inside content (excluding margin)!"),
            @ { w }
        }
    }
}
```

By nesting `FatObj`, you have full control over the composition order of attributes.

## Advanced: Dynamic Access & Modification

Built-in attributes (like `opacity`, `background`, `margin`) are properties of the `FatObj` wrapper. In a declarative UI, you typically bind these properties to state during creation. However, **if you need to modify them dynamically from code (e.g., inside an event handler) or pipe! the field, you have to access the field's Writer.**

### Simple Follower Example (one component follows another)

```rust no_run
use ribir::prelude::*;

/// A tiny example where `follower`'s `background` follows `leader`'s
/// `background`. Clicking the `leader` toggles its background color; the
/// `follower` updates automatically because it binds to `leader`'s writer
/// via `pipe!($read(...))`.
fn follower_example() -> Widget<'static> {
   fn_widget! {
        // Create stateful handles so we can access built-in attributes.
        let mut leader = @Text { text: "Leader (click me)" };

        // Follower: bind background to leader's background so it follows.
        let follower = @Text { 
          text: "Follower (follows leader)",
          background: pipe!($read(leader.background()).clone()),
        };

        let seed = Instant::now();
        @Column {
          // Leader: clicking toggles its background color
          @(leader)  {
            cursor: CursorIcon::Pointer,
            on_tap: move |_| {
              *$write(leader.background()) = Color::from_u32(seed.elapsed().as_millis() as u32).into();
            },
          }
          @ { follower }
        }
    }.into_widget()
}

```
Notes:

- The `leader` is a stateful handle created with `@`. Its `on_tap` event toggles its `background` using `$write` to update it.
- The `follower` binds its `background` to `leader`'s background using `pipe!($read(...)).map(...)`, so it updates automatically whenever the leader's background changes.
- This pattern is useful when multiple widgets should visually reflect a single source of truth (theme, selection, focus, etc.) without manually propagating events.

## Summary

`FatObj` is the key to Ribir's flexibility. It allows any Widget to have rich common capabilities while keeping the core Widget definition concise. Through built-in attributes, you can quickly build beautiful and interactive UIs without repeatedly implementing these basic features for each Widget.