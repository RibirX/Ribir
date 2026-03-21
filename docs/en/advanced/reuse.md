---
sidebar_position: 2
---

# Dynamic Generation and Reuse

When building dynamic UIs in Ribir, you frequently need either a fresh widget instance or a way to keep reusing an existing one. Ribir provides two primary mechanisms for this: **GenWidget** and **Widget Reuse**.

They solve opposite problems:
- **`GenWidget`**: Generates a **brand new** widget instance every time it is evaluated.
- **Widget Reuse (`ReuseKey`, `ReuseScope`)**: **Recycles and shares** existing widget instances to prevent unnecessary recreation.

---

## GenWidget

`GenWidget` is a special object that represents a deferred widget builder. When you use a `GenWidget` in your UI tree, Ribir executes its closure to create a fresh widget at that exact moment.

### When to use `GenWidget`
- When you want to defer heavy widget construction until it is actually needed in the layout.
- When you need to instantiate multiple **independent** copies of a widget from the same template.

```rust
use ribir::prelude::*;

fn generate_new_item() -> Widget<'static> {
    text! { text: "I am a new instance!" }.into_widget()
}

fn app() -> Widget<'static> {
    fn_widget! {
        @Column {
            @ { generate_new_item }
            @ { generate_new_item } // Creates a completely separate second instance
        }
    }
    .into_widget()
}
```

---

## Widget Reuse System

Unlike `GenWidget`, the reuse system allows you to recycle, cache, and share widget structures efficiently. This helps optimize performance for dynamic structures like lists, and enables seamless state persistence for shared UI elements like global toolbars.

The reuse system centers around three primary concepts: `ReuseScope`, `ReuseKey`, and `@Reuse` (or the `reuse:` property). `ReuseKey::resolve()` is only a convenience for creating a resolve-only `Reuse` expression; it is not another widget generator.

### 1. `ReuseScope`
A `ReuseScope` acts as the boundary and lifecycle manager for reusable bindings. 
- The framework implicitly provides a default root `ReuseScope` for your application.
- You can explicitly declare `@ReuseScope { ... }` to establish new local boundaries.
- The scope holds cached widgets inside it and efficiently returns them when requested again by their `ReuseKey`.

### 2. `ReuseKey`
A `ReuseKey` represents the unique identity and the **lookup policy** for a widget. Keys are declared with an explicit search scope:
- `ReuseKey::local(...)`: Searches only in the nearest visible `ReuseScope`. If a match is not found during `resolve_or_build`, it registers in this nearest scope. Ideal for recycling dynamic children (like list items) or local named elements.
- `ReuseKey::global(...)`: Searches outward through all `ReuseScope`s up to the root. If a match is found somewhere up the tree, it is reused. If the lookup fails across all scopes, it registers the new instance in the root scope. Ideal for global singletons (like an audio player floating widget).

### 3. Usage Patterns

#### Dynamic Children using Local Keys

The most standard use case for widget reuse is iterating over dynamic data and using `ReuseKey::local(...)`. Ribir provides `reuse:` as a built-in property on `FatObj`, which can be attached to any standard widget.

```rust
use ribir::prelude::*;

fn dynamic_list() -> Widget<'static> {
    let items = Stateful::new(vec!["Apple", "Banana", "Cherry"]);
    
    fn_widget! {
        // Establishes a boundary for the list items
        @ReuseScope {
            @Column {
                @ {
                    pipe!($read(items).clone()).map(move |list| {
                        list.into_iter().enumerate().map(|(i, title)| {
                            // Assigning a local ReuseKey allows Ribir to 
                            // recycle the @Text widget when data changes
                            @Text {
                                reuse: ReuseKey::local(i),
                                text: title
                            }
                        })
                    })
                }
            }
        }
    }
    .into_widget()
}
```

#### Global Shared Reusable Widgets

A `ReuseKey::global(...)` executes an explicit outward lookup. It's an excellent way to share a single UI element across totally different parts of your application architecture.

```rust
use ribir::prelude::*;

fn global_shared_example() -> Widget<'static> {
    let audio_player = ReuseKey::global("audio_player");

    fn_widget! {
        @Column {
            // At the first location: if missed, the framework will build and cache it at the root scope.
            @Reuse {
                reuse: audio_player.clone(),
                @Void {}
            }

            // In another view or deeply nested part of the UI, simply resolve it.
            // It will look upwards, hit the global/root scope, and reuse the same widget instance!
            @ { audio_player.resolve() }
        }
    }
    .into_widget()
}
```
`audio_player.resolve()` above is just shorthand for `@Reuse { reuse: audio_player }`. It does not create a `GenWidget`, and it does not imply creating a new instance.

When a global key is matched, the fallback child block (`@Void {}` above) will simply be ignored and will not participate in building, preventing unnecessary performance overhead.

#### Pre-registering Definitions (`defs`)

If you want to prepare reusable definitions in a scope without building them immediately, you can register a factory using the `defs` behavior of `ReuseScope`.

```rust
use ribir::prelude::*;

fn defs_example() -> Widget<'static> {
    let header = ReuseKey::local("header");

    fn_widget! {
        @ReuseScope {
            defs: [
                // Register early
                reuse_def(header.clone(), || fn_widget! { @Text { text: "Persistent Header" } }.into_widget())
            ],
            @Column {
                // Resolve exactly when needed
                @ { header.resolve() }
            }
        }
    }
    .into_widget()
}
```

#### Eviction and `leave()`

When a widget is no longer needed in the current context but you are within dynamic transitions, you can instruct it to explicitly leave.

```rust
use ribir::prelude::*;

fn leave_example() -> Widget<'static> {
    let key = ReuseKey::local("nav_bar");

    fn_widget! {
        @Void {
            on_tap: move |e| {
                key.leave(e);
            }
        }
    }
    .into_widget()
}
```

Using `.leave(ctx)` interacts with the lookup scopes:
- **Live Widgets**: If the widget is successfully resolved and currently visible, it will stay alive until it naturally disposes, but subsequent `.resolve()` queries against the same key in this scope will fail (or re-build).
- **Cached Widgets**: If the widget has detached and went into the standard cached registry, it is evicted immediately.
