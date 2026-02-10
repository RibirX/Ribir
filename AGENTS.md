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

## 6. Debug Logs & MCP (Important)
- **Start app with debug enabled**:
```bash
cargo run --features debug
```
- **Startup URLs**: On launch it prints `RIBIR_DEBUG_URL=...` and `RIBIR_DEBUG_UI=...`. Use these for manual access and include them in bug reports.
- **Logs**: In-memory ring buffer only, capped to 50,000 entries and ~60s of history. `/logs` returns NDJSON and includes `x-ribir-log-dropped` for any dropped count.
- **Captures**: Saved to `captures/<capture_id>/logs.jsonl` by default. Override with `RIBIR_CAPTURE_DIR=/path`.
- **MCP usage (agent-facing)**: From an MCP client, read resources and run tools below.
```text
Resources: ribir://status, ribir://windows, ribir://logs
Tools: capture_screenshot, inspect_tree, inspect_widget, get_overlays, set_log_filter,
       add_overlay, remove_overlay, clear_overlays, start_recording, stop_recording,
       capture_one_shot, start_app(project_path), attach_app(url), stop_app
```
- **Authoritative schema**: `core/src/debug_tool/mcp_schema.json`. Setup details in `dev-docs/debug-features.md`.

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
