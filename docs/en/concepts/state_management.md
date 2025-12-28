---
sidebar_position: 4
---

# State Management

Ribir uses a data-driven approach to state management. Instead of manually updating widgets, you modify the data (state), and Ribir automatically updates the parts of the UI that depend on that data.

## Stateful Objects

The core primitive for state in Ribir is `Stateful<T>`. It wraps a piece of data `T` and makes it observable. When the data inside a `Stateful` object is modified, Ribir notifies all observers (widgets, pipes, watchers) that the state has changed.

To create a stateful object, use `Stateful::new(value)`:

```rust ignore
use ribir::prelude::*;

fn main() {
    let count = Stateful::new(0);
}
```

`Stateful<T>` actually implements `StateReader<T>`, `StateWatcher<T>`, and `StateWriter<T>` traits, which provide access to the state.

### StateReader<T>

The `StateReader<T>` trait provides read-only access to the state. Through the implementation of `StateReader<T>`, you can obtain a read reference to the state.

### StateWatcher<T>

The `StateWatcher<T>` trait provides read-only access to the state. But unlike `StateReader<T>`, through the implementation of `StateWatcher<T>`, you get a subscription to the state changes of the host `T` (i.e., when the state of the host `T` changes, you will be notified).

### StateWriter<T>

The `StateWriter<T>` trait provides write access to the state. Through the implementation of `StateWriter<T>`, you can obtain a write reference to the state of the host `T`. When the modification to the mut ref is completed, Ribir will automatically notify all UI parts dependent on that data.

## Reading and Writing State

In the `fn_widget!` DSL, you can use specific syntax helpers to access state:

- **`$read(state)`**: Obtains a read reference to the state via `StateReader<T>`.
- **`$reader(impl StateReader<T>)`**: Obtains a clone of `StateReader<T>`, typically used to hold read permission in a closure.
- **`$watcher(impl StateWatcher<T>)`**: Obtains a clone of `StateWatcher<T>`, typically used to hold subscription permission in a closure.
- **`$writer(impl StateWriter<T>)`**: Obtains a clone of `StateWriter<T>`, typically used to hold write permission in a closure.

**Important**: The `$read`, `$write`, `pipe!`, `watch!` operators are **DSL-specific** and only work within macros that support the Ribir DSL syntax, such as `fn_widget!` and `rdl!`. These operators are not valid Rust syntax outside of these macros and will cause compilation errors if used in regular Rust code or nested within third-party macros.

**Note**: Outside of DSL macros, you can use `.read()` and `.write()` methods on the `Stateful` object, but these do not establish reactive dependencies automatically.

## DSL Operators in Third-Party Macros

The DSL operators (`@`, `$read`, `$write`, etc.) are **not valid** when nested inside third-party macros. This is because we cannot anticipate the processing logic of third-party macros. For example:

**❌ Invalid usage:**
```rust ignore
fn_widget! {
    ...
    // This will NOT work - $read is processed by println! which doesn't understand DSL syntax
    println!("{}", $read(some_state));
    ...
}
```

**✅ Valid usage:**
```rust ignore
fn_widget! {
    ...
    let val = $read(some_state);
    println!("{}", val);
    ...
}
```

## Simplified State Access in `fn_widget!` Closures

When using event handlers (like `on_tap`) inside `fn_widget!`, you often need to modify state. Ribir's helper macros (`$write`, `$read`, `$writer`, `$reader`, `$watcher`) are designed to work seamlessly with `move` closures.

They automatically detect when they are used inside a closure and handle the necessary cloning of the underlying state writer/reader. This means you rarely need to manually call `.clone_writer()` before the closure.

**Verbose (Old Way):**
```rust ignore
let writer = state.clone_writer();
@Button {
    on_tap: move |_| {
        *writer.write() += 1; // Manually cloned writer used here
    }
}
```

**Simplified (Recommended):**
```rust ignore
@Button {
    on_tap: move |_| {
        *$write(state) += 1; // Automatic cloning handled by $write
    }
}
```

## Reactive Binding with `pipe!`

The `pipe!` macro is the primary way to bind state to widget properties. It evaluates an expression and re-evaluates it whenever any state marked with `$read` or `$write` inside the expression changes.

`pipe!` creates a one-way data flow: from State to View.

```rust no_run
use ribir::prelude::*;

fn counter_example() -> Widget<'static> {
    fn_widget! {
        let count = Stateful::new(0);
        
        @Column {
            // Bind the text property to the count state
            @Text { 
                text: pipe!($read(count).to_string()) 
            }
            @Button {
                // Increment count on tap
                on_tap: move |_| *$write(count) += 1,
                @Text { text: "Increment" }
            }
        }
    }.into_widget()
}
```

In this example:
1. `pipe!($read(count).to_string())` creates a dynamic value.
2. Initially, it reads `count` (0) and returns "0".
3. When `on_tap` executes `*$write(count) += 1`, `count` changes.
4. The `pipe!` detects the change, re-runs `.to_string()`, and updates the `Text` widget.

### Important: Avoid Using `BuildCtx` Inside `pipe!` Expressions

`pipe!` expressions are re-evaluated whenever their dependent state changes. However, `BuildCtx` (build context) is only valid during the widget's build phase. Using `BuildCtx::get()` inside a `pipe!` expression will cause a runtime error when the expression is re-evaluated, as it attempts to access an invalid context.
 
 > [!WARNING]
 > **Runtime Error Risk**: Never use `BuildCtx::get()` directly inside a `pipe!` expression. It will panic when the pipe updates. See [Troubleshooting](../getting_started/troubleshooting.md#buildctxget-inside-pipe) for more details.

**Incorrect Example:**

```rust no_run
/// This is an incorrect example that will cause a runtime error!
use ribir::prelude::*;

fn bad_example() -> Widget<'static> {
    fn_widget! {
        let count = Stateful::new(0);
        @Text {
            // Error: BuildCtx::get() may be invalid when pipe! re-evaluates
            text: pipe!(*$read(count)).map(move |c| format!("tap {} on windows {:?}", c, BuildCtx::get().window().id())),
            on_tap: move |_| *$write(count) += 1,
        }
    }.into_widget()
}
```

**Correct Approach:**

If you need to access information from `BuildCtx`, capture it at the top level of `fn_widget!` and use it as a dependency or constant in the `pipe!` expression.

```rust no_run
use ribir::prelude::*;

fn good_example() -> Widget<'static> {
    fn_widget! {
        let count = Stateful::new(0);
        // Capture window ID during build phase and use it as a constant in pipe!
        let window_id = BuildCtx::get().window().id();
        @Text {
            text: pipe!(*$read(count)).map(move |c| format!("tap {} on windows {:?}", c, window_id)),
            on_tap: move |_| *$write(count) += 1,
        }
    }.into_widget()
}
```

## Reacting to Changes with `watch!`

While `pipe!` is for binding values to properties, `watch!` is for performing side effects (like logging, network requests, or complex logic) when state changes.

`watch!` creates an observable stream (rxRust stream). You must `.subscribe()` to it to execute code.

```rust no_run
use ribir::prelude::*;

fn watch_example() {
    let count = Stateful::new(0);

    // Watch for changes and print them
    let _subscription = watch!(*$read(count))
        .subscribe(|val| println!("Count changed to: {}", val));

    *count.write() = 1; // Prints: Count changed to: 1
    *count.write() = 2; // Prints: Count changed to: 2
}
```

### `pipe!` vs `watch!`

- **`pipe!(expr)`**: Returns a value (or a stream of values) intended for **initializing and updating widget properties**. It always has an initial value.
- **`watch!(expr)`**: Returns a stream. It is used for **side effects**. It does not emit an initial value, and you must explicitly subscribe to it.

## Advanced: Mapped & Distinct Pipes

`pipe!` can be combined with rxRust operators for more control. Since `Pipe` wraps the underlying stream, you can use `.transform()` to access the full power of rxRust operators.

```rust no_run
use ribir::prelude::*;

fn advanced_pipe() -> Widget<'static> {
    fn_widget! {
        let count = Stateful::new(0);
        @Row {
            @Text {
                // Only update the text if the value is even
                text: pipe!(*$read(count))
                    .transform(|s| s.filter(|v| v % 2 == 0).box_it())
                    .map(|v| format!("Even number: {}", v))
            }
            @Button {
                on_tap: move |_| *$write(count) += 1,
                @{"Increment" }
            }
        }
    }.into_widget()
}
```

Common operators include `.map()`, `.filter()`, `.distinct_until_changed()`, etc. Use `.transform()` when you need operators that change the stream structure or logic beyond simple mapping.