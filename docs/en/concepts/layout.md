---
sidebar_position: 6
---

# Layout System

Ribir's layout system uses a "Constraints Down, Size Up" single-pass model. This is very similar to Flutter's layout model and aims to achieve efficient and flexible UI layout.

## Core Principles

1.  **Constraints Down**: Parent Widgets pass layout constraints (Constraints) down to child Widgets. These constraints define the minimum and maximum width and height that child Widgets can occupy.
2.  **Size Up**: Child Widgets calculate their own size based on the received constraints and return the final determined size (Size) to the parent Widget.
3.  **Parent Sets Position**: After receiving the child Widget's size, the parent Widget determines the child Widget's position in its own coordinate system.

## BoxClamp

Layout constraints are represented by the `BoxClamp` struct. It contains four values:
*   `min_width`, `max_width`
*   `min_height`, `max_height`

`BoxClamp` defines an allowed size range. The final size of a child Widget must be within this range.

*   **Loose Constraints**: `min` is 0, `max` is some finite value. Child Widgets can be any size between 0 and max.
*   **Tight Constraints**: `min` equals `max`. Child Widgets are forced to a specific size.
*   **Unbounded Constraints**: `max` is infinity. Child Widgets can extend infinitely (usually appears in scrolling containers).

## Layout Process

Each Widget must implement the `perform_layout` method in the `Render` trait:

```rust ignore
fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size
```

In this method, the Widget needs to do three things:
1.  **Layout children**: Iterate through its child nodes, calculate a new `BoxClamp` for each child (based on the incoming `clamp` and its own layout logic), and call `ctx.perform_child_layout(child, child_clamp)`.
2.  **Determine position**: Get the `Size` returned by the child node, and set the child node's position according to the layout logic `ctx.update_position(child, position)`.
3.  **Return size**: Calculate and return its own final `Size`, and this size must satisfy the incoming `clamp` constraint.

## Using the `clamp` Attribute to Intervene in Layout

Ribir provides a built-in `clamp` attribute that allows you to directly modify the parent constraints a Widget receives when declaring it. This is implemented behind the scenes by wrapping a `ConstrainedBox`.

```rust no_run
use ribir::prelude::*;

fn example() -> Widget<'static> {
    fn_widget! {
        @Container {
            size: Size::new(100., 100.),
            background: Color::RED,
            // Force constraint: no matter what constraint the parent gives, the Container's width must be between 50 and 200
            clamp: BoxClamp {
                min: Size::new(50., 0.),
                max: Size::new(200., f32::INFINITY),
            }
        }
    }.into_widget()
}
```

**Note**: The `clamp` attribute's role is to **further restrict** the constraints passed down from the parent, taking the intersection.

## Common Layout Widgets

*   **Row / Column**: Linear layout. Provides unbounded constraints in the main axis direction (if scrolling or adaptive is allowed), and passes loose or strict constraints in the cross axis.
*   **Stack**: Stack layout. Passes the same constraints to all non-positioned child nodes.
*   **SizedBox**: Forces child nodes to a specific size (by applying Tight Constraints).

## Custom Layout Example

If you need to implement a custom layout Widget, you need to implement the `Render` trait. Here is a simple example that forces child nodes to a fixed size (similar to `SizedBox`):

```rust no_run
use ribir::prelude::*;

#[derive(SingleChild, Declare, Clone)]
struct FixedSizeBox {
    size: Size,
}

impl Render for FixedSizeBox {
    fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
        // 1. Determine the size we want, must be within the parent constraint range
        let my_size = clamp.clamp(self.size);

        // 2. If there is a child node, force the child node to this size as well
        if let Some(child) = ctx.single_child() {
             // Create a tight constraint (Tight Constraint)
            let child_clamp = BoxClamp { min: my_size, max: my_size };

            // Layout child node
            ctx.perform_child_layout(child, child_clamp);

            // Set child node position (usually (0,0))
            ctx.update_position(child, Point::zero());
        }

        // 3. Return final size
        my_size
    }
}
```

By understanding `BoxClamp` and `perform_layout`, you can fully control the layout behavior of your UI.