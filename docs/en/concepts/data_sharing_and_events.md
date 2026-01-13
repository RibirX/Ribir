---
sidebar_position: 5
---

# Data Sharing & Events

Ribir provides mechanisms to share data across the widget tree and handle communication between widgets efficiently.

- **Provider**: Pass data down the tree (Parent -> Child / Descendants).
- **Custom Event**: Pass messages up the tree (Child -> Parent / Ancestors).

## Provider (Data Down)

The `Provider` system allows you to share data with all descendant widgets without passing props through every level of the hierarchy.

### Providing Data

You can provide data using the `Providers` widget or the `providers!` macro (or `providers` property).

#### Using `providers` Field (Recommended)

Most widgets in Ribir (via `FatObj`) support a `providers` field directly (see [Built-in Attributes & FatObj](./built_in_attributes_and_fat_obj.md)). This is often cleaner than wrapping your widget in a `Providers` widget.

```rust ignore
use ribir::prelude::*;

struct Theme {
    primary: Color,
}

fn app() -> Widget<'static> {
    fn_widget! {
        let theme = Stateful::new(Theme { primary: Color::RED });

        @Column {
            // Provide data directly on the widget
            providers: [Provider::new(theme)],
            @Text { text: "Hello" }
        }
    }.into_widget()
}
```

#### Using `Providers` Widget

Alternatively, you can use the `Providers` widget to wrap a subtree.

```rust ignore
use ribir::prelude::*;

struct UserInfo {
    name: String,
}

fn app() -> Widget<'static> {
    fn_widget! {
        let user = Stateful::new(UserInfo { name: "Alice".to_string() });

        @Providers {
            providers: [
                // Share a state writer, allowing descendants to read and modify 'user'
                // The second argument `None` means we manage dirty notifications manually or don't need layout updates
                Provider::writer(user, None)
            ],
            @Column {
                @ChildWidget {}
            }
        }
    }.into_widget()
}
```

### Consuming Data

Descendant widgets can access the provided data using `Provider::of` (read) or `Provider::write_of` (write).

```rust ignore
fn child_widget() -> Widget<'static> {
    fn_widget! {
        @Text {
            // Read from the nearest UserInfo provider
            text: pipe! {
                let user = Provider::of::<UserInfo>(BuildCtx::get())?;
                format!("Hello, {}", user.name)
            }
        }
    }.into_widget()
}
```

### Types of Providers

| Method | Description | Access |
|---|---|---|
| `Provider::new(value)` | Share an immutable value. | `Provider::of` |
| `Provider::reader(state)` | Share a read-only state. | `Provider::of`, `Provider::state_of` (reader) |
| `Provider::writer(state, phase)` | Share a mutable state. `phase` controls layout dirty marking. | `Provider::of`, `Provider::write_of`, `Provider::state_of` (writer) |
| `Provider::watcher(state)` | Share a watcher (observable). | `Provider::of`, `Provider::state_of` (watcher) |

## Variant (Flexible Data Consumption)

`Variant` is a convenient wrapper around the `Provider` system that automatically handles both static values and dynamic states (watchers). It simplifies consuming provider data by unifying the API for both cases.

### Difference from Direct Provider Access

When using `Provider` directly, you need to know whether the data is:
- A static value (`Provider::of`)
- A state reader/writer (`Provider::state_of`)
- A watcher (`Provider::state_of` with watcher type)

`Variant` automatically detects the provider type and handles reactivity for you:
- First checks for a **watcher provider** (reactive)
- Falls back to a **value provider** (static)

### Advantages of Using Variant
1. **Unified API**: One method (`Variant::new`) works for both static and dynamic providers
2. **Automatic Reactivity**: If the provider is a watcher, your widget automatically updates when the value changes
3. **Easy Mapping**: Transform values while preserving reactivity using `map()`
4. **Built-in Fallbacks**: Use `new_or()`, `new_or_default()`, or `new_or_else()` to provide default values

### Basic Usage

```rust no_run
use ribir::prelude::*;

fn themed_box() -> Widget<'static> {
    fn_widget! {
        // Automatically gets Color from provider (static or reactive)
        let color = Variant::<Color>::new(BuildCtx::get()).unwrap();
        
        @Container {
            size: Size::new(100., 100.),
            // If an ancestor provides a writer of Color, this will react to changes
            background: color,
        }
    }.into_widget()
}
```

### Using Fallbacks

```rust ignore
fn themed_text() -> Widget<'static> {
    fn_widget! {
        // Use theme color if available, otherwise use red
        let color = Variant::<Color>::new_or(
            BuildCtx::get(), 
            Color::from_rgb(255, 0, 0)
        );
        
        @Text {
            text: "Hello",
            foreground: color,
        }
    }.into_widget()
}
```

### Mapping Values

```rust ignore
fn subtle_background() -> Widget<'static> {
    fn_widget! {
        let color = Variant::<Color>::new_or_default(BuildCtx::get());
        
        // Transform the color while preserving reactivity
        let subtle_color = color.map(|c| c.with_alpha(0.1));
        
        @Container {
            background: subtle_color,
            // ... other properties
        }
    }.into_widget()
}
```

### Special Color Helpers

For `Variant<Color>`, there are built-in methods for visiting different tones based on the theme's lightness group:

```rust ignore
let primary = Variant::<Color>::new(BuildCtx::get()).unwrap();

// Get different tones based on the theme's lightness group
let base = primary.into_base_color(BuildCtx::get());
let container = primary.into_container_color(BuildCtx::get());
let on_color = primary.on_this_color(BuildCtx::get());
let on_container = primary.on_this_container_color(BuildCtx::get());
```

These methods automatically adjust the lightness while maintaining reactivity to the source `Variant`, making it easy to create consistent, theme-aware UIs.

## Custom Events (Messages Up)

Custom events allow a widget to dispatch an event that bubbles up the widget tree. Ancestors can listen to these events without explicitly passing callbacks down.

### Defining an Event

Define a struct to hold your event data.

```rust ignore
struct MyCustomEvent {
    message: String,
}
```

### Dispatching an Event

Use `ctx.window().bubble_custom_event(...)` to dispatch the event from a widget.

```rust ignore
fn child_emitter() -> Widget<'static> {
    fn_widget! {
        @Button {
            on_tap: move |e| {
                let event = MyCustomEvent { message: "Clicked!".to_string() };
                // Bubble the event from this widget (e.id)
                e.window().bubble_custom_event(e.id, event);
            },
            @{ "Emit Event" }
        }
    }.into_widget()
}
```

### Listening to Events

Ancestors can listen to specific custom events using `on_custom`.

```rust ignore
fn parent_listener() -> Widget<'static> {
    fn_widget! {
        @Column {
            // Listen for MyCustomEvent bubbling up from children
            on_custom: |e: &mut CustomEvent<MyCustomEvent>| {
                println!("Received: {}", e.message);
                
                // Stop the event from bubbling further up
                e.stop_propagation();
            },
            @child_emitter {}
        }
    }.into_widget()
}
```

### Listening to Any Custom Event

You can also listen to all bubbling custom events using `on_raw_custom`, but you will receive a `RawCustomEvent` that you need to downcast manually if needed.

```rust ignore
@Container {
    on_raw_custom: |e: &mut RawCustomEvent| {
        println!("Something happened!");
    }
}
```
