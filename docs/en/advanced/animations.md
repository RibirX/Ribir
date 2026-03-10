---
sidebar_position: 2
---

# Animations

Ribir provides a powerful, multi-purpose animation system to help you build fluid and responsive interactive UIs. Centered around the `@Animate` primitive, it provides granular control over any application state transitions with near-zero performance overhead.

Rather than forcing all use cases into a single monolithic API, Ribir offers a versatile toolkit of animation abstractions—ranging from low-level property interpolation to high-level, business-state choreography. 

## Basic Concepts: The `@Animate` Primitive

Before exploring the advanced animation mechanics, you must first understand the foundational building block: the `@Animate` primitive.

`@Animate` provides the core mechanism to drive a value over time. It requires three core properties:
- `state`: The target you want to animate. It must implement the `AnimateState` trait. Widget properties accessed via their writers (e.g., `widget.margin()`, `widget.opacity()`) automatically implement this.
- `from`: The starting value of the animation.
- `transition`: The timing function and duration (e.g., `EasingTransition`).

Ribir has predefined some animation Transitions in the `easing` module, such as `LINEAR`, `EASE_IN_OUT`, and `CubicBezierEasing`.

### Animation Lifecycle and Runtime Behavior

Understanding how animations interact with the rendering pipeline is crucial to avoid common mistakes:

1. **Per-frame interpolation**: During the rendering phase, the animation calculates the precise value based on elapsed time and the transition curve, writing it temporarily to the target state.
2. **Post-render restoration**: Immediately after the frame is painted, the temporary value is discarded and the original state is restored. This guarantees your core data model remains pure and unmutated throughout the animation lifespan.
3. **Isolated shallow updates**: The animation engine applies changes using `shallow()` writes. This intentionally triggers only localized Widget repaints, aggressively suppressing downstream reactive cascades (such as `pipe!` recalculations) to maintain flawless 60+ FPS performance.

> ⚠️ **CRITICAL RULE**: You **must** bind `@Animate` directly to the target Widget's property Writer! If you bind it to an isolated `Stateful` variable, the `shallow()` update isolation will prevent `pipe!` listeners from receiving the interpolated frames, making your animation appear broken.

```rust ignore
// ❌ WRONG: Do not animate intermediate data states
fn_widget! {
    let opacity_state = Stateful::new(0.0);
    let animate = @Animate {
        state: opacity_state.clone_writer(), // Won't trigger the pipe!
        // ...
    };

    @Container {
        opacity: pipe!(*$read(opacity_state)), // This pipe will not see animation frames
        on_tap: move |_| animate.run(),
    }
}
```

```rust ignore
// ✅ CORRECT: Animate Widget's state directly
fn_widget! {
    let w = @Container { opacity: 1., /* ... */ };
    let animate = @Animate {
        state: w.opacity(),
        // ...
    };

    @(w) { on_tap: move |_| animate.run() }
}
```

This design ensures animations run smoothly at high frame rates without causing UI reflows or redundant rebuilds elsewhere in your app.

---

## Animation Toolkit

Manually triggering `Animate.run()` is tedious for complex interactions. To solve this, Ribir provides distinct abstractions tailored for different UI scenarios.

Choosing the correct tool keeps your code clean and your state predictable. Selecting too low-level of a primitive will force you to manually wire up listeners and rebuild state machines from scratch.

### Manual property animation with `@Animate`

Use this when you need explicit, imperative control over starting and stopping an animation.

```rust no_run
use ribir::prelude::*;

fn manual_animation() -> Widget<'static> {
    fn_widget! {
        let mut moving_box = @Container {
            size: Size::new(50., 50.),
            background: Color::RED,
            margin: EdgeInsets::horizontal(200.),
        };

        let animate = @Animate {
            state: moving_box.margin(),
            from: EdgeInsets::horizontal(0.),
            transition: EasingTransition {
                duration: Duration::from_millis(1000),
                easing: easing::CubicBezierEasing::new(0.68, -0.55, 0.265, 1.55),
            }
        };

        @Container {
            size: Size::new(250., 100.),
            @(moving_box) {
                // Manually start the animation
                on_mounted: move |_| animate.run(),
            }
        }
    }
    .into_widget()
}
```

### Auto-animate a property with `transition(...)`

When you simply want a property to glide smoothly to its new value whenever it changes, attach a transition directly to its `StateWriter`.

```rust no_run
use ribir::material::md;
use ribir::prelude::*;

fn writer_bound_animation() -> Widget<'static> {
    fn_widget! {
        let mut w = @Container { size: Size::new(40., 20.) };
        w.opacity().transition(EasingTransition {
            easing: md::easing::STANDARD_ACCELERATE,
            duration: md::easing::duration::SHORT2,
        });

        let cnt = Stateful::new(0);

        @(w) {
            background: Color::RED,
            on_tap: move |_| {
                *$write(cnt) += 1;
                // Writing to opacity automatically triggers the transition animation
                *$write(w.opacity()) = if *$read(cnt) % 2 == 0 { 1.0 } else { 0.5 };
            },
        }
    }
    .into_widget()
}
```

**When to use `transition` vs `transition_with_init`**
- `transition(...)`: The animation naturally departs from the property's actual current value.
- `transition_with_init(init, ...)`: Use this when a component first appears and you want its entrance animation to start from a virtual 'zero' state (e.g. `opacity: 0.0 -> 1.0`).

💡 *Rule of Thumb*: A transition modifier must ALWAYS be installed *before* the first value write occurs.

### Business-state orchestration with `@AnimateMatch`

Complex widgets like Buttons often have distinct business states (`Idle`, `Hover`, `Active`), each requiring vastly different combinations of colors, borders, and shadows. `@AnimateMatch` eradicates manual listener resets by declaratively projecting 'business-state enums' directly into 'absolute visual target arrays'.

**Example**

```rust no_run
use ribir::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq)]
enum CardStatus {
    Idle,
    Hover,
    Active,
}

fn card_demo() -> Widget<'static> {
    fn_widget! {
        let card_status = Stateful::new(CardStatus::Idle);

        let mut card = @Container {
            size: Size::new(120., 80.),
            background: Color::from_rgb(0x33, 0x66, 0xFF),
            on_pointer_enter: move |_| *$write(card_status) = CardStatus::Hover,
            on_pointer_leave: move |_| *$write(card_status) = CardStatus::Idle,
            on_pointer_down: move |_| *$write(card_status) = CardStatus::Active,
            on_pointer_up: move |_| *$write(card_status) = CardStatus::Hover,
        };

        let opacity = card.opacity();
        let transform = card.transform();

        let _am = @AnimateMatch {
            value: card_status.clone_watcher(),
            cases: cases! {
                state: (opacity, transform),
                CardStatus::Idle => (1.0, Transform::identity()),
                CardStatus::Hover => (0.92, Transform::scale(1.04, 1.04)),
                CardStatus::Active => (0.72, Transform::scale(0.96, 0.96)),
            },
            transitions: transitions! {
                (_, CardStatus::Active) => EasingTransition {
                    easing: easing::LINEAR,
                    duration: Duration::ZERO,
                },
                _ => EasingTransition {
                    easing: easing::EASE_IN_OUT,
                    duration: Duration::from_millis(180),
                },
            },
            interruption: Interruption::Fluid,
        };

        card
    }
    .into_widget()
}
```

- **`cases!`**: Define your absolute visual destinations cleanly. Ribir groups all targeted properties into a perfectly synchronized, highly optimized single animation loop with exhaustive compiler-time state-matching checks.
- **`transitions!`**: A pattern-matching router `(from_state, to_state)` that assigns the precise easing curve dynamically based on where the user is coming from and where they are going.

If you have highly dynamic durations or simply prefer pure Rust over macros, you can pass a pure closure to `transitions`:

```rust ignore
transitions: |from, to| {
    if to == &CardStatus::Active {
        EasingTransition { easing: easing::LINEAR, duration: Duration::ZERO }
    } else {
        EasingTransition { easing: easing::EASE_IN_OUT, duration: Duration::from_millis(180) }
    }
},
```

### Shape a timeline with `keyframes!`

`keyframes!` allows you to script precise intermediate waypoints (via percentages or `0.0` - `1.0` floats) across a single overarching animation timeline.

```rust ignore
let animate = @Animate {
    state: keyframes! {
        state: box_widget.size(),
        0.25 => DimensionSize::new(100., 50.),
        0.5 => DimensionSize::new(100., 100.),
        0.75 => DimensionSize::new(50., 100.),
        1.0 => DimensionSize::new(50., 50.),
    },
    from: DimensionSize::new(50., 50.),
    transition: EasingTransition {
        duration: Duration::from_millis(1000),
        easing: easing::EASE_IN_OUT,
    }
};
```

💡 *When to use*: If your design asks "What should this look like halfway through?", use keyframes. If it asks "What should this look like when the user hovers?", fallback to `@AnimateMatch`.

### Sequence multiple animations with `Stagger`

`Stagger` provides powerful group orchestration, letting you elegantly offset the start times of multiple visual elements—perfect for cascading list items, staggered reveals, and waterfall effects.

```rust no_run
use ribir::prelude::*;

fn stagger_example() -> Widget<'static> {
    fn_widget! {
        let stagger = Stagger::new(
            Duration::from_millis(200), // 200ms between each animation start
            EasingTransition {
                duration: Duration::from_millis(500),
                easing: easing::EASE_IN_OUT,
            },
        );

        let mut text1 = @Text { text: "One", opacity: 0. };
        let mut text2 = @Text { text: "Two", opacity: 0. };
        let mut text3 = @Text { text: "Three", opacity: 0. };

        stagger.write().push_state(text1.opacity(), 0.);
        stagger.write().push_state(text2.opacity(), 0.);
        stagger.write().push_state(text3.opacity(), 0.);

        @Column {
            on_mounted: move |_| stagger.run(),
            @{ [text1, text2, text3] }
        }
    }
    .into_widget()
}
```

### Lifecycle Animations: `AnimatedVisibility` vs `AnimatedPresence`

Mounting and unmounting objects efficiently is a complex challenge. We provide two dedicated widgets to handle appearance lifecycles:

| Need | Choose | Description |
| --- | --- | --- |
| Frequent toggle of a static subtree | `AnimatedVisibility` | Widget stays mounted in memory. Hidden state can still execute its leave animation. Good for panels, drawers, or menus. |
| Real mount / dispose animation | `AnimatedPresence` | Enter runs when mounted; leave runs when disposed. Frees memory after leave. Good for conditional render, lists, or pages. |

## Advanced Transition Modifiers

Transitions can be effortlessly composed with modifiers like `repeat(...)` and `delay(...)` for robust timeline structuring.

```rust no_run
use ribir::prelude::*;

fn transition_modifiers_example() -> Widget<'static> {
    fn_widget! {
        let mut box_widget = @Container {
            size: Size::new(200., 100.),
            background: Color::YELLOW,
            opacity: 1.,
        };

        let animate = @Animate {
            state: box_widget.opacity(),
            from: 0.,
            transition: EasingTransition {
                duration: Duration::from_millis(100),
                easing: easing::steps(2, easing::StepsJump::JumpNone),
            }
            .repeat(3.) // Repeat 3 times
            .delay(Duration::from_millis(1000)) // Wait 1 second before starting
        };

        @(box_widget) {
            on_mounted: move |_| animate.run(),
        }
    }
    .into_widget()
}
```

## Summary: The Quick Choice Guide

If you're still unsure which layer to reach for, run your feature through this quick questionnaire:

1. **Is this about component render lifecycle (mount / dispose)?**
     - yes → `AnimatedPresence` (or `AnimatedVisibility` for persistent DOM-like elements)
2. **Is there already a business-state enum driving multiple visuals?**
     - yes → `@AnimateMatch`
3. **Do I only need a single property to animate gracefully when written?**
     - yes → `transition(...)`
4. **Do I need to programmatically restart or manually trigger an animation?**
     - yes → `@Animate`
5. **Do I need a custom visual progression or sequence?**
     - yes → `keyframes!` / `Stagger`