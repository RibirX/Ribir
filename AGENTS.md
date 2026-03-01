# 🤖 Ribir AI Agent Development Guide

Welcome, AI Agent. To contribute effectively to Ribir, you must understand our unique non-intrusive reactive architecture and DSL. This guide defines the "Ribir Way" of coding.

## 1. The Ribir Philosophy
- **Data-Centric**: UI is a projection of data. Focus on designing data structures first.
- **Non-Intrusive**: Data doesn't need to know about UI. We wrap data in `Stateful<T>` to make it reactive.
- **Precise Updates**: Only the specific part of the UI that depends on a changed field re-renders.

## 2. Core DSL & Macros (The "$ & @" Syntax)
Ribir uses custom macros that **only work inside `fn_widget!`, `rdl!`, or `widget!`**.

| Operator | Usage | Purpose |
| :--- | :--- | :--- |
| `@` | `@Row { ... }` | Declare a widget (shorthand for `rdl!`). |
| `$read(s)` | `$read(my_state).name` | Tracks dependency. Re-builds UI when `name` changes. |
| `$writer(s)` | `$writer(my_state).part_writer(...)` | Creates a partial writer for state slicing. |
| `$write(s)` | `$write(my_state).name = ...` | Modifies data and triggers UI update on drop. |
| `pipe!(...)` | `pipe!($read(s).val.to_string())` | Creates a reactive stream of values. |
| `distinct_pipe!` | `distinct_pipe!($read(s).val)` | Like `pipe!`, but only emits when value changes. |
| `watch!(...)` | `watch!($read(s).field)` | Observes changes and returns an Observable stream. |
| `fn_widget!` | `fn_widget! { ... }` | The standard way to define a UI block. |

### ⚠️ Critical Rules for AI:
1. **Scope Limit**: Never use `$read`, `$write`, or `pipe!` outside of a Ribir macro scope.
2. **Move Capture**: Always use `move` in closures (e.g., `on_tap: move |_| ...`) because Ribir widgets often require `'static` lifetime.
3. **Avoid Manual Refresh**: Never try to manually trigger a UI refresh. Use `$write` and let the framework handle it.

## 3. State Management Patterns

### 3.1 Creating State
```rust
let count = Stateful::new(0); // Simple value
let todo_list = Stateful::new(TodoList::default()); // Complex struct
```

### 3.2 Slicing State (Partial State)
For performance in lists, don't pass the whole list to sub-widgets. Use `part_writer`:
```rust
// Only the sub-widget depends on this specific item
let task = $writer(this).part_writer(
  format!("task {id:?}").into(),  // PartialId for debugging
  move |todos| PartMut::new(todos.get_task_mut(id).unwrap()),
);
```

### 3.3 Silent Updates
Use `.silent()` if you need to update data without triggering UI (e.g., internal cache):
```rust
state.silent().internal_flag = true;
```

## 4. Standard Component Pattern
Follow this structure when creating new UI components:

```rust
use ribir::prelude::*;

#[derive(Declare)]
pub struct MyComponent { ... }

impl Compose for MyComponent {
  fn compose(this: impl StateWriter<Value = Self>) -> Widget<'static> {
    row! {
      @Text { text: pipe!($read(this).title.clone()) }
      @Button {
        on_tap: move |_| *$write(this).title = "Clicked!".to_string(),
        @ { Label::new("Click Me") }
      }
    }.into_widget()
  }
}
```

## 5. Development Workflow
1. **Format**: `cargo +nightly ci fmt`
2. **Check**: `cargo +nightly ci check`
3. **Lint**: `cargo +nightly ci lint`
4. **Test**: `cargo +nightly ci test`
5. **Visual Tests**: If you change rendering, check `test_cases/`. Update expected images with `RIBIR_IMG_TEST=overwrite cargo +nightly ci test`.

## 6. Debug & MCP

Use the **Ribir Debug Server** MCP to inspect and debug running applications.

### Starting Applications

Use the `start_app` tool to launch or attach to a Ribir debug session. 
If the app is already running manually, you can use `attach_app` with the `RIBIR_DEBUG_URL`.

For specific parameter details and exact behavior, please refer directly to the MCP tool descriptions as they are the most accurate source.

### Key Tools

Commonly used MCP tools:
- `start_app` / `attach_app` / `stop_app` - Application lifecycle
- `capture_screenshot` - Visual state inspection
- `inspect_tree` / `inspect_widget` - Widget tree inspection
- `add_overlay` / `remove_overlay` - Visual debugging
- `inject_events` - Simulate user interactions
- `set_log_filter` - Dynamic log level adjustment

### Resources

- `ribir://logs` - Application logs (NDJSON, ~60s history, 50k entry cap)
- `ribir://windows` - Active windows
- `ribir://status` - Server status

See `dev-docs/debug-features.md` for detailed API documentation.

### Custom Debug Names (`debug_name`)

**IMPORTANT FOR AI AGENTS**: When debugging or interacting with a Ribir application, **proactively** assign stable names to target widgets using `debug_name: "some_name"` in the widget declaration. This allows you to easily find and interact with them via MCP tools (`inspect_widget`, `inject_events`, `add_overlay`, etc.) using the `name:some_name` format without needing to traverse the tree for a numeric `index1` ID.

```rust
button! {
  debug_name: "counter_button", // Add this to target it via MCP
  @{ "+1" }
}
```

Then, you can directly use it in MCP tools:
- `inspect_widget` with `{"id": "name:counter_button"}`
- `inject_events` with `{"events": [{"type": "click", "id": "name:counter_button"}]}`
- `add_overlay` with `{"id": "name:counter_button"}`

Rules for `debug_name`:
- Works via builtin `with_debug_name` on `FatObj`.
- Only active in debug builds.
- Falls back to type-based names if not set, but explicit names are highly recommended for robust MCP tool interactions.

## 7. Interaction & Data Flow
For interactive widgets, follow the **Single Source of Truth** rule: UI is a projection of data.

1. **Path A (Standard)**: Data changes -> `pipe!` emits -> UI updates.
2. **Path B (User Intent)**: Interaction -> `on_change` -> Update Data -> (Go to Path A).
3. **Path C (Optimistic)**: Direct `$write(widget)` -> Update Data (Async). *Use sparingly for latency compensation.*

> 📘 **Deep Dive**: For complex interactive widgets, strictly follow `dev-docs/interactive-widget-design.md`.

## 8. Rust Code Organization

Use Rust 2018+ module style: `foo.rs` + `foo/` directory (not `foo/mod.rs`).

```
# ✅ Preferred
src/
├── mcp.rs           # Module entry
└── mcp/
    ├── serve.rs
    └── schema.rs

# ❌ Avoid
src/
└── mcp/
    ├── mod.rs
    └── ...
```

## 9. Common Pitfalls
- **Deadlocks/Panics**: Calling `$write(s)` while a `$read(s)` is active in the same scope (RefCell borrow conflict).
- **Infinite Loops**: A `pipe!` that modifies the same state it reads from.
- **Orphan Widgets**: Forgetting to call `.into_widget()` when returning a widget from a function that isn't `fn_widget!`.
