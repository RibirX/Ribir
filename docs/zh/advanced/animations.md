---
sidebar_position: 2
---

# 动画

Ribir 提供了一个强大的动画系统，让您能够创建流畅、交互式的 UI。动画系统建立在 `@Animate` Widget 和各种过渡机制之上，这些机制使您能够动画化应用程序中的任何状态。

## Animate

创建动画的主要方式是通过 `@Animate` 创建，您能指定动画的持续时间、缓动函数和状态变化。

### 基本动画

`@Animate` 需要三个主要属性:
- `state`: 您想要动画化的状态，必须实现 `AnimateState` trait，基本类型的 `impl StateWriter<T: Clone>` 已经实现，所以使用中只要获取相应属性的 `StateWriter` 即可。
- `from`: 动画的起始值
- `transition`: 动画应如何随时间推移进行

创建了动画后，只需调用 `run` 方法即可开始动画的运行。


Ribir 中已经预定义了一些动画 Transition

- `easing::LINEAR`: 以恒定速度动画化
- `easing::EASE_IN`: 开始缓慢，加速到结束
- `easing::EASE_OUT`: 开始快速，减速到结束
- `easing::EASE_IN_OUT`: 开始缓慢，中间加速，然后减速
- `easing::CubicBezierEasing`: 三次贝塞尔缓动

下例中，`Container` 会在首次加载时实现一个跳动的动画。

```rust no_run
use ribir::prelude::*;

fn custom_easing_example() -> Widget<'static> {
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
                easing: easing::CubicBezierEasing::new(0.68, -0.55, 0.265, 1.55), // 弹跳效果
            }
        };

        @Container {
            size: Size::new(250., 100.),
            @(moving_box) {
                on_mounted: move |_| animate.run(),
            }
        }
    }.into_widget()
}
```

### 动画生命周期和运行时行为

理解动画生命周期对于有效的动画实现至关重要。Ribir 动画系统遵循动画与状态和渲染管道交互的特定模式：

1. **每帧绘制**: 在渲染期间，动画修改状态值以反映动画的当前进度。在每一帧中，动画系统根据当前时间和过渡函数计算插值，并用该值临时更新状态。

2. **绘制结束**: 渲染帧后，动画系统恢复原始状态值。这确保了动画完成后基础数据模型保持不变。

3. **状态传播**: 在动画期间，状态更改通过 `StateWriter` 的 `shallow()` 方法在反应系统中进行修改。此方法更新状态并通知 Widget 系统进行高效重绘，但它**不会**触发所有监听器（如 `pipe!` 块中）的完整反应通知，以避免性能开销和潜在的无限循环。

   这就是为什么将 `Animate` 直接绑定到 **Widget 属性的 Writer**（例如 `widget.map_writer(...)`）而非独立的 `Stateful` 变量至关重要的原因。当绑定到 Widget 属性的 Writer 时，`shallow()` 更新正确通知 Widget 使用新的插值重绘，而其他绑定到该状态的数据则不会收到更新通知。

此行为确保了:
- 动画以高帧率流畅运行
- 动画期间的状态更改不会导致 UI 重排或不必要的重建
 - 插值状态值仅在每帧绘制期间被应用；绘制完成后会恢复原始状态，因此这些插值更改不会在动画的绘制步骤之外持续存在
- 通过避免冗余的响应式更新优化动画性能

> **重要**: 将动画绑定到 Widget 的状态（例如使用 `map_writer` 或属性写入器如 `.opacity()`）。因为 `Animate` 使用 `shallow()` 更新，这些更新不会触发联动的更新。

一个常见错误是创建一个独立的 `Stateful` 变量，动画化它，并使用 `pipe!` 将其绑定到 Widget。

```rust ignore
// ❌ 错误：不要动画化中间状态
fn_widget! {
    let opacity_state = Stateful::new(0.0);
    let animate = @Animate {
        state: opacity_state.clone_writer(),
        ...
    };

    @Container {
        opacity: pipe!(*$read(opacity_state)),
        on_tap: move |_| animate.run(),
    }
}
```

这会失败，因为动画过程中 `opacity_state` 的值会被修改，但不会扩散，所以 `pipe!` 不会感知到并触发更新。

正确的做法是直接对 Widget 的状态进行动画的绑定，如下：

```rust ignore
// ✅ 正确：动画化 Widget 的状态
fn_widget! {
    let w = @Container {
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

### 自动绑定动画

`Animate` 提供了最基础的能力，允许您手动控制动画的开始与停止。此外，Ribir 还提供一种便捷方式，可将动画绑定到属性本身，当属性值通过 `StateWriter` 更改时自动触发过渡动画。

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

这里，`w.opacity()` 返回的 `StateWriter` 实现了 `AnimateState` trait。通过 `transition()` 方法设置动画属性后，当通过该 `StateWriter` 修改值时，动画会自动触发。

## 高级动画

### 关键帧动画

关键帧允许您在动画中指定中间步骤，提供对复杂动画的细粒度控制。

```rust no_run
use ribir::prelude::*;

fn keyframes_example() -> Widget<'static> {
    fn_widget! {
        let mut box_widget = @Container {
            size: Size::new(50., 50.),
            background: Color::GREEN,
        };

        let animate = @Animate {
            state: keyframes! {
                state: box_widget.size(),
                0.25 => DimensionSize::new(100., 50.),  // 在 25% 进度时水平拉伸
                0.5 => DimensionSize::new(100., 100.),  // 在 50% 进度时垂直拉伸
                0.75 => DimensionSize::new(50., 100.),  // 在 75% 进度时水平缩小
                1.0 => DimensionSize::new(50., 50.),    // 在 100% 进度时返回原始
            },
            from: DimensionSize::new(50., 50.),
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

关键帧可以使用十进制值（0.0 到 1.0）或百分比定义：

```rust ignore
// 使用百分比语法
let keyframe_state = keyframes! {
    state: opacity_writer,
    20% => 0.2,
    50% => 0.5,
    80% => 0.8,
};
```

### 使用 Stagger 编排动画

为了协调多个动画，Ribir 提供了 `Stagger` 动画控制器。这允许您创建动画在定时间隔开始的序列，创建复杂的视觉效果：

```rust no_run
use ribir::prelude::*;

fn stagger_example() -> Widget<'static> {
    fn_widget! {
        let stagger = Stagger::new(
            Duration::from_millis(200), // 每个动画开始之间 200ms
            EasingTransition {
                duration: Duration::from_millis(500),
                easing: easing::EASE_IN_OUT,
            },
        );

        let mut text1 = @Text { text: "One", opacity: 0. };
        let mut text2 = @Text { text: "Two", opacity: 0. };
        let mut text3 = @Text { text: "Three", opacity: 0. };

        // 向 stagger 添加动画
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

#### 高级 Stagger 功能

Stagger 动画提供额外的控制选项：

- **不同 stagger**: 使用 `push_animation_with()` 为每个动画指定不同的时间间隔
- **混合动画**: 在同一序列中组合基于状态的动画与完整的 `@Animate` Widget
- **运行时控制**: 使用 `is_running()`、`run_times()` 和 `has_ever_run()` 等方法访问 stagger 状态

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

        let mut box1 = @Container { size: Size::new(50., 50.), background: Color::RED, opacity: 0. };
        let mut box2 = @Container { size: Size::new(50., 50.), background: Color::GREEN, opacity: 0. };
        let mut box3 = @Container { size: Size::new(50., 50.), background: Color::BLUE, opacity: 0. };

        // 以不同的 stagger 间隔添加框
        stagger.write().push_state(box1.opacity(), 0.);
        stagger.write().push_state_with(Duration::from_millis(200), box2.opacity(), 0.); // 等待 200ms
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

### 动画控制

动画可以通过动画实例进行编程控制：

- `run()`: 开始或重新开始动画
- `stop()`: 停止动画并恢复状态到最终值
- `is_running()`: 检查动画是否正在运行

```rust no_run
use ribir::prelude::*;

fn animation_control_example() -> Widget<'static> {
    fn_widget! {
        let mut box_widget = @Container {
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
            from: 0.,  // 开始从当前值动态完成
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
                        // on_tap 处理程序将获取 animation 的所有权，这里使用 $writer 自动克隆
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

### 动画组合

动画可以组合和分层以创建复杂效果。您可以：
- 并行运行多个动画
- 单次动画同时修改多个属性

```rust no_run
use ribir::prelude::*;

fn composition_example() -> Widget<'static> {
    fn_widget! {
        let mut box_widget = @Container {
            size: Size::new(50., 50.),
            background: Color::BLUE,
            opacity: 0.,
            transform: Transform::identity(),
        };

        let opacity_size_anim = @Animate {
            state: (
                box_widget.opacity(),
                box_widget.width()
            ),
            from: (0., 20_f32.into()),
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


### 高级过渡修饰符

动画可以使用各种过渡修饰符增强以提供额外功能。两个常见的修饰符是 `repeat` 和 `delay`。

#### 重复与延迟动画

动画可以同时使用 `repeat` 和 `delay` 过渡修饰符。下面的示例展示了一个动画：它在开始前等待 1000ms，然后重复三次，通过将不透明度从 0 动画到 1 来实现闪烁效果。

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
            .repeat(3.) // 重复3次
            .delay(Duration::from_millis(1000)) // 延迟1000ms 再执行
        };

        @(box_widget) {
            on_mounted: move |_| animate.run(), // Start the animation after delay with repetitions
        }
    }.into_widget()
}
```

动画是创建引人入胜、直观的用户体验的强大工具。通过掌握 Ribir 中的动画系统，您可以创建流畅、响应式的应用程序，使其感觉生动和交互式。