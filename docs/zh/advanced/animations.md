---
sidebar_position: 2
---

# 动画

Ribir 提供了一个强大且全面的动画系统，助您构建流畅响应的交互体验。系统以底层的 `@Animate` 组件为核心，通过提供多维度的抽象工具，以极低的性能开销实现了界面状态的弹性动画，并且巧妙地避免了数据流触发意外的全局重建。

我们不会强迫您用单一的底层 API 解决所有复杂的动画需求。针对不同的逻辑场景，Ribir 提供了一套丰富的动画工具箱：从底层的属性插值引擎，一直到完全基于业务状态枚举的高级组合编排机制。

## 基本概念：`@Animate` 原语

在探索高级动画机制之前，我们需要先了解整个动画系统的核心基石：`@Animate` 原语。

`@Animate` 提供了推动变量随时间运行的核心机制，它需要至少配置以下内容：
- `state`: 要被动画驱动的目标，必须实现 `AnimateState` trait。您组件中的各种属性可以通过调用其专属的 writer（如 `widget.margin()`）自动满足此要求。
- `from`: 本次动画的起点值。
- `transition`: 描述这次动画的时长和缓动方式（例如 `EasingTransition`）。

Ribir 已经在 `easing` 模块里预制了诸多物理函数，比如 `LINEAR`, `EASE_IN_OUT`, 或 `CubicBezierEasing` 可供使用。

### 需要理解的重要生命周期和渲染行为

理解动画和渲染管线的交互是避免意外错误的第一步：

1. **逐帧插值**：在每一帧渲染期间，动画会根据当前时间和缓动函数计算出插值进度，并将其临时写入（浅覆盖）至目标状态中。
2. **渲染后恢复**：当前帧绘制完成后，临时插值会被立刻恢复。这保证了底层数据模型在动画期间不会被污染，保持了数据流的纯洁性。
3. **浅更新隔离**：动画产生的插值是通过 `shallow()` 方法写入的。这意味着它只会触发当前绑定 Widget 的重绘，而绝不会向下游广播，避免了触发挂载在 `pipe!` 上的全量响应式刷新。

> ⚠️ **关键准则**：`Animate` **必须**直接绑定在 Widget 属性的 Writer 上！如果将其绑定到一个游离的独立 `Stateful` 变量上，由于 `shallow()` 的隔离特性，它产生的变化将无法被下游响应，导致动画“看起来失效”。

```rust ignore
// ❌ 错误做法：不要向独立的下游数据插值，这将无法驱动 pipe 刷新界面
fn_widget! {
    let opacity_state = Stateful::new(0.0);
    let animate = @Animate {
        state: opacity_state.clone_writer(), // 不会引发 pipe! 的深入通知
        // ...
    };

    @Container {
        opacity: pipe!(*$read(opacity_state)), // 此处将在动画运行中看起来失效
        on_tap: move |_| animate.run(),
    }
}
```

```rust ignore
// ✅ 正确做法：直接捕捉要运行的界面组件对象，对它的属性调用 Writer
fn_widget! {
    let w = @Container { opacity: 1., /* ... */ };
    let animate = @Animate {
        state: w.opacity(),
        // ...
    };

    @(w) { on_tap: move |_| animate.run() }
}
```

此设计可以大幅节约响应成本负担，确保即便是全屏动画也能以令人舒适的高帧率流畅运行。

---

## 动画开发工具箱

对于复杂的交互，纯命令式地调用 `Animate.run()` 并非长久之计。Ribir 因此为您设计了面向不同维度的动画封装工具。

选对工具，代码会很短，状态模型也会很干净；如果没有选对，往往就会需要手写一堆的监听或重置逻辑。

### 用 `@Animate` 手动驱动属性动画

也就是我们上方演示过的能力，当你确实需要代码中的显式按钮来控制动画何时开始时，它是最好的工具。

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
                // 手动启动
                on_mounted: move |_| animate.run(),
            }
        }
    }
    .into_widget()
}
```

### 用 `transition(...)` 自动补间属性变化

如果您不想手动管理动画实例，而是希望在**数据发生改变的瞬间**，界面能平滑地过渡到新状态。

```rust no_run
use ribir::material::md;
use ribir::prelude::*;

fn writer_bound_animation() -> Widget<'static> {
    fn_widget! {
        let mut w = @Container { size: Size::new(40., 20.) };
        
        // 挂载过渡效果
        w.opacity().transition(EasingTransition {
            easing: md::easing::STANDARD_ACCELERATE,
            duration: md::easing::duration::SHORT2,
        });

        let cnt = Stateful::new(0);

        @(w) {
            background: Color::RED,
            on_tap: move |_| {
                *$write(cnt) += 1;
                // 每次赋值操作都将受到自动的补间计算
                *$write(w.opacity()) = if *$read(cnt) % 2 == 0 { 1.0 } else { 0.5 };
            },
        }
    }
    .into_widget()
}
```

**何时使用 `transition` 和 `transition_with_init`**
- `transition(...)`：状态改变时，从当前界面上的实际值开始平滑过渡。
- `transition_with_init(init, ...)`：有时您希望第一次进场时从一个虚拟的起点开始（例如 `opacity: 0.0 -> 1.0`），请使用此方法。

💡 **经验法则**：过渡规则必须在状态被写入新值**之前**声明绑定。

### 用 `@AnimateMatch` 把业务状态编排成视觉状态

面对 `Idle`, `Hover`, `Active` 这种多状态业务组件，每个状态都对应着 `尺寸`、`透明度`、`颜色` 等多属性的视觉差异。`@AnimateMatch` 能直接将“业务状态枚举”声明式地映射为“最终视觉目标集”。

**例如：**

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

- **`cases!`**：您只需声明最终的绝对视觉目标，无需编写繁琐的回调。底层不仅提供了完善的穷举编译检查，还会自动将多个属性打包进同一个动画时钟，确保多属性动画进度绝对同步且性能最优。
- **`transitions!`**：基于 `(从何处, 到何处)` 的路由匹配，自顶向下寻找并应用指定的缓动过渡函数。

如果你更喜欢写纯粹的原生函数进行计算而不是依靠宏路由，你可以在该属性上向它传入裸闭包：

```rust ignore
transitions: |from, to| {
    if to == &CardStatus::Active {
        EasingTransition { easing: easing::LINEAR, duration: Duration::ZERO }
    } else {
        EasingTransition { easing: easing::EASE_IN_OUT, duration: Duration::from_millis(180) }
    }
},
```

### 用 `keyframes!` 编排单体元素的时间轴曲线

`keyframes!` 允许您使用百分比（或 0.0 到 1.0 的小数）定义一条连续动画中的多个中间停顿点（关键帧）。

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

💡 **何时使用**：当您的思考方式是“动画进行到 50% 时它该长什么样？”时，请使用关键帧；如果是“当用户悬停时它该长什么样？”，请退回使用 `@AnimateMatch`。

### 用 `Stagger` 实现多元素的错峰级联动画

`Stagger` 提供了强大的群体动画控制能力，非常适合为列表项、瀑布流或批量登场的大量视觉元素设置规律的错峰入场时间（延迟级联）。

```rust no_run
use ribir::prelude::*;

fn stagger_example() -> Widget<'static> {
    fn_widget! {
        let stagger = Stagger::new(
            Duration::from_millis(200), // 每个开始的序列动画将间隔 200 ms
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

### 生命周期专属层：`AnimatedVisibility` 和 `AnimatedPresence`

最后，控制物体的出现和退散是极其高频的需求。我们设计了由两个 Widget 提供专属帮助：

| 需求 | 选择 | 解释与说明 |
| --- | --- | --- |
| 同一颗子树需要极度频繁地切换显隐 | `AnimatedVisibility` | 对象仍在内存和节点树中保留。适合需要极速响应或自带折叠动效的浮窗、菜单、抽屉等组件。 |
| 需要真实动态挂载/卸载的生命周期组件 | `AnimatedPresence` | 依附于动态数据流，在真正挂载前播放入场动画，在彻底销毁前播放退场动画，视觉结束后精准释放内存。适合列表数据的增删、条件渲染和路由页面切换。 |

## 高级过渡修饰

过渡函数总是可以任意叠加像 `repeat(...)` 及 `delay(...)` 之类的灵活操作用于更自由的时间延展。

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
            .repeat(3.) // 播放完成后重复 3 回
            .delay(Duration::from_millis(1000)) // 等待整整 1 秒再出场
        };

        @(box_widget) {
            on_mounted: move |_| animate.run(),
        }
    }
    .into_widget()
}
```

## 总结：我到底该选哪一个？

为了避免您陷入选择困难，我们在开始前为您准备了一份“首选决策自问清单”：

1. **您的动画核心焦点是为了某一片内容出现或消失吗？**
     - 是的 → 马上采用 `AnimatedPresence` (或 `AnimatedVisibility` 对付常驻元素)
2. **您的交互中是否有某个单独业务状态导致了很多样式产生联动切换？**
     - 是的 → 使用强大的 `@AnimateMatch`
3. **我仅仅是想要当一个参数或样式修改时，能够看起来自然吗？**
     - 是的 → 一把梭使用 `transition(...)` 安装挂接组件
4. **我在处理由鼠标按下的回调和某段倒计时去手动跑马灯吗？**
     - 是的 → 写一个游标状态再跑 `@Animate`
5. **我在为时间点设计一长串定制连续帧或批量卡片吗？**
     - 是的 → `keyframes!` 与 `Stagger` 会节省您几百行的代码