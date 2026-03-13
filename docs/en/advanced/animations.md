---
sidebar_position: 2
---

# Animations

Ribir provides a multi-layer animation system centered around the `@Animate` primitive. It ranges from low-level property interpolation to high-level state orchestration.

## Basic Concept: `@Animate`

`@Animate` drives a value over time using three core properties:
- `state`: The target writer (e.g., `widget.margin()`, `widget.opacity()`).
- `from`: The starting value.
- `transition`: The timing function (e.g., `EasingTransition`).

### Runtime Behavior

1. **Per-frame interpolation**: Calculating and writing values to the state during rendering.
2. **Post-render restoration**: Discarding temporary values after painting to keep the data model pure.
3. **Shallow updates**: Using `shallow()` writes to trigger repaints without redundant `pipe!` recalculations.

> ⚠️ **IMPORTANT**: Bind `@Animate` directly to a Widget's property Writer. Binding to an isolated `Stateful` variable will fail to trigger UI updates due to shallow write isolation.

```rust ignore
// ✅ CORRECT: Animate Widget's state directly
fn_widget! {
  let w = @Container { opacity: 1. };
  let animate = @Animate {
    state: w.opacity(),
    from: 0.,
    transition: EasingTransition { 
      duration: Duration::from_millis(300), 
      easing: easing::LINEAR 
    }
  };
  @(w) { on_tap: move |_| animate.run() }
}
```

---

## Animation Toolkit

### Auto-animate with `transition()`
Attach a transition directly to a `StateWriter` to animate every value change automatically.

- `transition(...)`: Animates from the current value.
- `transition_with_init(init, ...)`: Useful for entrance animations starting from a custom value (e.g., `0.0` opacity).

```rust no_run
use ribir::prelude::*;

fn demo() -> Widget<'static> {
  fn_widget! {
    let mut w = @Container { size: Size::new(40., 20.) };
    w.opacity().transition(EasingTransition {
      easing: easing::LINEAR,
      duration: Duration::from_millis(150),
    });
    // Writing to opacity now triggers the animation automatically.
    w
  }
  .into_widget()
}
```

### State Orchestration with `@AnimateMatch`
Map business states (enums) to visual targets. It synchronizes multiple properties in a single optimized loop.

```rust no_run
use ribir::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq)]
enum CardStatus {
  Idle,
  Active,
}

fn demo() -> Widget<'static> {
  fn_widget! {
    let card_status = Stateful::new(CardStatus::Idle);
    let mut card = @Container { size: Size::new(80., 40.) };
    let opacity = card.opacity();
    let transform = card.transform();

    // cases! defines targets, transitions! defines routing.
    let _am = @AnimateMatch {
      value: card_status.clone_watcher(),
      cases: cases! {
        state: (opacity, transform),
        CardStatus::Idle => (1.0, Transform::identity()),
        CardStatus::Active => (0.7, Transform::scale(0.9, 0.9)),
      },
      transitions: transitions! {
        (_, CardStatus::Active) => EasingTransition {
          easing: easing::LINEAR,
          duration: Duration::ZERO,
        },
        _ => EasingTransition {
          easing: easing::LINEAR,
          duration: Duration::from_millis(200),
        },
      },
    };

    card
  }
  .into_widget()
}
```

### Lifecycle: `AnimatedVisibility` vs `AnimatedPresence`

| Widget | Usage | Persistence |
| --- | --- | --- |
| `AnimatedVisibility` | Toggling frequent UI (sidebars, menus) | Stays mounted in the tree. |
| `AnimatedPresence` | Dynamic content (lists, conditional blocks) | Mounted/Unmounted structurally. |

Both use the same API:
- `cases`: Visual targets for `true` (shown) and `false` (hidden).
- `enter` / `leave`: Optional transitions for each direction.

### Timeline & Sequences
- `keyframes!`: Script precise intermediate waypoints.
- `Stagger`: Offset start times for multiple elements (e.g., cascading lists).

---

## Summary Guide

1. **Mount/Dispose animation?** → `AnimatedPresence` (or `AnimatedVisibility` if static).
2. **Enum-driven visuals?** → `@AnimateMatch`.
3. **Simple property glide?** → `transition(...)`.
4. **Manual control?** → `@Animate`.
5. **Complex timeline?** → `keyframes!` or `Stagger`.
