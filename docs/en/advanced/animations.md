---
sidebar_position: 2
---

# Animations

Ribir provides a powerful animation system that allows you to create smooth, interactive UIs. The animation system is built around the `@Animate` widget and various transition mechanisms that enable you to animate any state in your application.

## Animate

The primary way to create animations is through `@Animate`, where you can specify the duration, easing function, and state changes.

### Basic Animation

`@Animate` requires three main properties:
- `state`: The state you want to animate. It must implement the `AnimateState` trait. Basic types `impl StateWriter<T: Clone>` already implement it, so you just need to get the `StateWriter` of the corresponding property.
- `from`: The starting value of the animation.
- `transition`: How the animation should progress over time.

Once the animation is created, simply call the `run` method to start it.

Ribir has predefined some animation Transitions:

- `easing::LINEAR`: Animates at a constant speed
- `easing::EASE_IN`: Starts slowly, accelerates toward the end
- `easing::EASE_OUT`: Starts quickly, decelerates toward the end
- `easing::EASE_IN_OUT`: Starts slowly, accelerates in the middle, then decelerates
- `easing::CubicBezierEasing`: Cubic Bezier easing

In the following example, `SizedBox` will perform a bouncing animation when first loaded.

```rust no_run
use ribir::prelude::*;

fn custom_easing_example() -> Widget<'static> {
    fn_widget! {
        let mut moving_box = @SizedBox {
            size: Size::new(50., 50.),
            background: Color::RED,
            margin: EdgeInsets::horizontal(200.),
        };

        let animate = @Animate {
            state: moving_box.margin(),
            from: EdgeInsets::horizontal(0.),
            transition: EasingTransition {
                duration: Duration::from_millis(1000),
                easing: easing::CubicBezierEasing::new(0.68, -0.55, 0.265, 1.55), // Bounce effect
            }
        };

        @SizedBox {
            size: Size::new(250., 100.),
            @(moving_box) {  
                on_mounted: move |_| animate.run(),
            }
        }
    }.into_widget()
}
```

### Animation Lifecycle and Runtime Behavior

Understanding the animation lifecycle is crucial for effective animation implementation. The Ribir animation system follows specific patterns for how animations interact with state and the rendering pipeline:

1. **Each frame draw**: During rendering, the animation modifies state values to reflect the current progress of the animation. On each frame, the animation system calculates the interpolated value based on the current time and transition function, and temporarily updates the state with that value.

2. **End of drawing**: After the frame is rendered, the animation system restores the original state values. This ensures that the underlying data model remains unchanged once the animation completes.

3. **State propagation**: During animation, the state changes are modified through the `shallow()` method of the `StateWriter` in the reactive system. This method updates the state and notifies the widget system for efficient repainting, but it **does not** trigger a full reactive notification for all listeners (like those in `pipe!` blocks) to avoid performance overhead and potential infinite loops.

   This is why it is critical to bind `Animate` directly to the **Widget property's Writer** (e.g. `widget.map_writer(...)`) rather than a standalone `Stateful` variable. When bound to a widget property's Writer, the `shallow()` update correctly notifies the widget to redraw with the new interpolated value, while other data bound to that state will not receive update notifications.

This behavior ensures that:
- Animations run smoothly at high frame rates
- State changes during animation don't cause UI reflows or unnecessary rebuilds
- The original state value is preserved after animation completion
- Animation performance is optimized by avoiding redundant reactive updates
 - Animations run smoothly at high frame rates
 - State changes during animation don't cause UI reflows or unnecessary rebuilds
 - Interpolated state values are applied only while each animation frame is being rendered; after the frame is drawn the original state is restored, so these interpolated changes do not persist beyond the animation's drawing step
 - Animation performance is optimized by avoiding redundant reactive updates
> **Important**: Bind the animation to the Widget's state (e.g., using `map_writer` or property writers like `.opacity()`). Because `Animate` uses `shallow()` updates, these updates will not trigger linked updates.

A common mistake is to create a standalone `Stateful` variable, animate it, and bind it to a widget using `pipe!`.

```rust ignore
// ❌ WRONG: Do not animate intermediate state
fn_widget! {
    let opacity_state = Stateful::new(0.0);
    let animate = @Animate {
        state: opacity_state.clone_writer(),
        ...
    };

    @SizedBox {
        opacity: pipe!(*$read(opacity_state)),
        on_tap: move |_| animate.run(),
    }
}
```

This fails because the value of `opacity_state` is modified during the animation process, but it does not propagate, so `pipe!` will not perceive it and trigger an update.

The correct approach is to bind the animation directly to the Widget's state, as follows:

```rust ignore
// ✅ CORRECT: Animate Widget's state
fn_widget! {
    let w = @SizedBox {
        opacity: 1.,
        ...
    };
    let animate = @Animate {
        state: w.opacity(),
        ...
    };

    @(w) { on_tap: move |_| animate.run() }
}

```

### Auto-binding Animations

`Animate` provides the most basic capabilities, allowing you to manually control the start and stop of animations. In addition to this, Ribir also provides a convenient way to bind animations to the property itself, automatically triggering the transition animation when the property value changes.

```rust no_run
use ribir::prelude::*;
use ribir::material::md;
fn writer_animate() -> Widget<'static> {
    fn_widget! {
        let mut w = @Container { size: Size::new(40., 20.) };
        w.opacity()
            .transition(EasingTransition{
                easing: md::easing::STANDARD_ACCELERATE,
                duration: md::easing::duration::SHORT2
            });

        let cnt = Stateful::new(0);

        @(w) {
            on_tap: move |_| {
                *$write(cnt) += 1;
                if (*$read(cnt) % 2 == 0) {
                    *$write(w.opacity()) = 1.;
                } else {
                    *$write(w.opacity()) = 0.5;
                }
            },
            background: Color::RED,
        }

    }.into_widget()
}
```

Here, the `StateWriter` returned by `w.opacity()` implements the `AnimateState` trait. By setting the animation property through the `transition()` method, the animation is automatically triggered when the value is modified via the `StateWriter`.


## Advanced Animations

### Keyframe Animations

Keyframes allow you to specify intermediate steps in an animation, providing fine-grained control over complex animations.

```rust no_run
use ribir::prelude::*;

fn keyframes_example() -> Widget<'static> {
    fn_widget! {
        let mut box_widget = @SizedBox {
            size: Size::new(50., 50.),
            background: Color::GREEN,
        };

        let animate = @Animate {
            state: keyframes! {
                state: box_widget.map_writer(|w| PartMut::new(&mut w.size)),
                0.25 => Size::new(100., 50.),  // Stretch horizontally at 25% progress
                0.5 => Size::new(100., 100.),  // Stretch vertically at 50% progress
                0.75 => Size::new(50., 100.),  // Shrink horizontally at 75% progress
                1.0 => Size::new(50., 50.),    // Return to original at 100% progress
            },
            from: Size::new(50., 50.),
            transition: EasingTransition {
                duration: Duration::from_millis(1000),
                easing: easing::EASE_IN_OUT,
            }
        };

        @(box_widget) { 
            on_tap: move |_| animate.run(),
        }
    }.into_widget()
}
```

Keyframes can be defined using either decimal values (0.0 to 1.0) or percentages:

```rust ignore
// Using percentage syntax
let keyframe_state = keyframes! {
    state: opacity_writer,
    20% => 0.2,
    50% => 0.5,
    80% => 0.8,
};
```

### Complex Animations with Stagger

For coordinating multiple animations, Ribir provides the `Stagger` animation controller. This allows you to create sequences where animations start at timed intervals, creating sophisticated visual effects:

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

        // Add animations to the stagger
        stagger.write().push_state(text1.opacity(), 0.);
        stagger.write().push_state(text2.opacity(), 0.);
        stagger.write().push_state(text3.opacity(), 0.);

        @Column {
            on_mounted: move |_| stagger.run(),
            @{ [text1, text2, text3] }
        }
    }.into_widget()
}
```

#### Advanced Stagger Features

Stagger animations provide additional control options:

- **Different staggers**: Use `push_animation_with()` to specify different time intervals for each animation
- **Mixed animations**: Combine state-based animations with complete `@Animate` widgets in the same sequence
- **Runtime control**: Access stagger status using methods like `is_running()`, `run_times()`, and `has_ever_run()`

```rust no_run
use ribir::prelude::*;

fn advanced_stagger_example() -> Widget<'static> {
    fn_widget! {
        let stagger = Stagger::new(
            Duration::from_millis(100),
            EasingTransition {
                duration: Duration::from_millis(300),
                easing: easing::EASE_IN_OUT,
            }
        );

        let mut box1 = @SizedBox { size: Size::new(50., 50.), background: Color::RED, opacity: 0. };
        let mut box2 = @SizedBox { size: Size::new(50., 50.), background: Color::GREEN, opacity: 0. };
        let mut box3 = @SizedBox { size: Size::new(50., 50.), background: Color::BLUE, opacity: 0. };

        // Add boxes with different stagger intervals
        stagger.write().push_state(box1.opacity(), 0.);
        stagger.write().push_state_with(Duration::from_millis(200), box2.opacity(), 0.); // Wait 200ms
        stagger.write().push_animation({
            let animate = @Animate {
                state: box3.opacity(),
                from: 0.,
                transition: EasingTransition {
                    duration: Duration::from_millis(300),
                    easing: easing::EASE_IN_OUT,
                }
            };
            animate
        });

        @Row {
            on_mounted: move |_| stagger.run(),
            @{ [box1, box2, box3] }
        }
    }.into_widget()
}
```

### Animation Control

Animations can be controlled programmatically using the animation instance:

- `run()`: Starts or restarts the animation
- `stop()`: Stops the animation and restores the state to its final value
- `is_running()`: Checks if the animation is currently running

```rust no_run
use ribir::prelude::*;

fn animation_control_example() -> Widget<'static> {
    fn_widget! {
        let mut box_widget = @SizedBox {
            size: Size::new(100., 100.),
            background: Color::PURPLE,
            opacity: 0.0,
        };

        let tap_animation = @Animate {
            state: box_widget.opacity(),
            from: 0.,
            transition: EasingTransition {
                duration: Duration::from_millis(2000),
                easing: easing::EASE_IN_OUT,
            }
        };

        let animation = @Animate {
            state: box_widget.opacity(),
            from: 0.,  // Start from current value is done dynamically
            transition: EasingTransition {
                duration: Duration::from_millis(2000),
                easing: easing::EASE_IN_OUT,
            }
        };

        @Column {
            @Row {
                @Button {
                    on_tap: move |_| {
                        let val = *$read(box_widget.opacity());
                        *$write(box_widget.opacity()) = 1.0 - val;
                        // the on_tap handler will take the ownership of animation, here use the $writer to auto clone
                        $writer(animation).run(); 
                    },
                    @Text { text: "Start" }
                }
                @Button {
                    on_tap: move |_| {
                        animation.stop();
                    },
                    @Text { text: "Stop" }
                }
            }
            @ { box_widget }
        }
    }.into_widget()
}
```

### Animation Composition

Animations can be combined and layered to create complex effects. You can:
- Run multiple animations in parallel using different state values
- Modify multiple properties in a single animation

```rust no_run
use ribir::prelude::*;

fn composition_example() -> Widget<'static> {
    fn_widget! {
        let mut box_widget = @SizedBox {
            size: Size::new(50., 50.),
            background: Color::BLUE,
            opacity: 0.,
            transform: Transform::identity(),
        };

        let opacity_size_anim = @Animate {
            state: (box_widget.opacity(), box_widget.map_writer(|w| PartMut::new(&mut w.size))),
            from: (0., Size::new(50., 50.)),
            transition: EasingTransition {
                duration: Duration::from_millis(1000),
                easing: easing::EASE_IN_OUT,
            }
        };

        let rotation_anim = @Animate {
            state: box_widget.transform(),
            from: Transform::identity(),
            transition: EasingTransition {
                duration: Duration::from_millis(2000),
                easing: easing::LINEAR,
            }
        };

        @(box_widget) {
            on_tap: move |_| {
                opacity_size_anim.run();
                rotation_anim.run();
            },
        }
    }.into_widget()
}
```


### Advanced Transition Modifiers

Animations can be enhanced using various transition modifiers that provide additional functionality. Two common modifiers are `repeat` and `delay`.

#### Repeating and Delayed Animations

Animations can combine the `repeat` and `delay` transition modifiers. The following example shows an animation that waits 1000ms before starting, then repeats three times, blinking by animating opacity from 0 to 1.

```rust no_run
use ribir::prelude::*;

fn transition_modifiers_example() -> Widget<'static> {
    fn_widget! {
        let mut box_widget = @SizedBox {
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
            .repeat(3.) // Then repeat 3 times
            .delay(Duration::from_millis(1000)) // Wait 1000ms before starting
        };

        @(box_widget) {
            on_mounted: move |_| animate.run(), // Start the animation after delay with repetitions
        }
    }.into_widget()
}
```

Animations are a powerful tool for creating engaging, intuitive user experiences. By mastering the animation system in Ribir, you can create smooth, responsive applications that feel alive and interactive.