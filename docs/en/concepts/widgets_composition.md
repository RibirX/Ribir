---
sidebar_position: 2
---

# Widget System

Ribir's widget system is built on three core traits: `Render`, `Compose`, and `ComposeChild`. Understanding these traits is key to creating custom widgets and understanding how Ribir constructs the UI tree.

## 1. Render vs Compose

### The `Render` Trait

`Render` is the low-level interface for widgets that actually draw something on the screen or manage layout directly. If a widget is a "leaf" node that paints pixels (like `Text` or `Rectangle`) or a container that calculates the positions of its children (like `Row` or `Column`), it implements `Render`.

Key responsibilities of `Render`:
- **Layout**: Calculating its own size and the position of its children (`perform_layout`).
- **Painting**: Drawing content to the canvas (`paint`).
- **Hit Testing**: Determining if a point interacts with the widget (`hit_test`).

```rust ignore
// Simplified concept of a Render widget
impl Render for MyCustomPainter {
    fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
        // Calculate size...
        Size::new(100., 100.)
    }

    fn paint(&self, ctx: &mut PaintingCtx) {
        // Draw something...
        let rect = Rect::from_size(ctx.box_size().unwrap());
        ctx.painter().rect(&rect).fill();
    }
}
```

### The `Compose` Trait

`Compose` is for high-level widgets that are built by combining other widgets. They don't draw anything themselves; they just expand into a tree of other widgets. This is similar to a "Component" in React or Vue.

Most application-level widgets (like a `UserProfile` or `LoginForm`) implement `Compose`.

```rust no_run
use ribir::prelude::*;

#[declare]
pub struct WelcomeCard;

impl Compose for WelcomeCard {
    fn compose(this: impl StateWriter<Value = Self>) -> Widget<'static> {
        fn_widget! {
            @Column {
                @Text { text: "Welcome!" }
                @Button { @{ "Click me" } }
            }
        }.into_widget()
    }
}
```

## 2. ComposeChild & Child Structure

Ribir uses a strictly typed parent-child relationship system. Not all widgets can accept children, and some accept specific types of children.

### The `ComposeChild` Trait

`ComposeChild` is a variation of `Compose` for widgets that wrap or modify a child widget. It defines how the parent and child are combined.

```rust ignore
pub trait ComposeChild<'c>: Sized {
    type Child;
    fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c>;
}
```

### SingleChild vs MultiChild

`SingleChild` and `MultiChild` traits are used to identify the type of widget that accepts the number of children, usually used for layout.

- **SingleChild**: Widgets that accept exactly one child.
  - Example: `SizedBox`, `Padding`, `Container`.
  - In DSL: `@Container { @Text { ... } }`

- **MultiChild**: Widgets that accept a list of children.
  - Example: `Row`, `Column`, `Stack`.
  - In DSL:
    ```rust ignore
    @Column {
        @Text { ... }
        @Text { ... }
    }
    ```

Usually you just need to specify it when defining the type:
```rust ignore
#[derive(SingleChild, Declare)]
pub struct Container;
```

```rust ignore
#[derive(MultiChild, Declare)]
pub struct Row;
```
