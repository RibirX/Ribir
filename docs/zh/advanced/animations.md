---
sidebar_position: 2
---

# 动画

Ribir 提供了一个多层次的动画系统，以 `@Animate` 原语为核心，涵盖从底层属性插值到高层状态编排的各类需求。

## 基本概念：`@Animate`

`@Animate` 通过三个核心属性驱动数值随时间变化：
- `state`: 目标 Writer（如 `widget.margin()`，`widget.opacity()`）。
- `from`: 动画起点值。
- `transition`: 缓动函数（如 `EasingTransition`）。

### 运行行为

1. **逐帧插值**：在渲染期间计算并写入进度值。
2. **渲染后恢复**：绘制完成后丢弃临时值，保持原始数据模型纯净。
3. **浅更新隔离**：使用 `shallow()` 写入，仅触发重绘，不引发冗余的 `pipe!` 计算。

> ⚠️ **重要规则**：`Animate` **必须**直接绑定到 Widget 属性的 Writer。绑定到孤立的 `Stateful` 变量会导致界面无法响应动画更新。

```rust ignore
// ✅ 正确做法：直接绑定组件属性 Writer
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

## 动画开发工具箱

### 自动补间：`transition()`
直接为 `StateWriter` 挂载过渡效果，使每次数值改变都自动平滑。

- `transition(...)`: 从当前值开始过渡。
- `transition_with_init(init, ...)`: 适合需要自定义入场起点（如从 `0.0` 透明度开始）的场景。

```rust no_run
use ribir::prelude::*;

fn demo() -> Widget<'static> {
  fn_widget! {
    let mut w = @Container { size: Size::new(40., 20.) };
    w.opacity().transition(EasingTransition {
      easing: easing::LINEAR,
      duration: Duration::from_millis(150),
    });
    // 此时写入 opacity 将自动触发平滑动画。
    w
  }
  .into_widget()
}
```

### 状态编排：`@AnimateMatch`
将业务状态（枚举）映射为视觉目标，自动同步多属性动画。

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

    // cases! 定义视觉目标，transitions! 定义过渡路由。
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

### 生命周期：`AnimatedVisibility` vs `AnimatedPresence`

| 组件 | 用途 | 行为 |
| --- | --- | --- |
| `AnimatedVisibility` | 高频切换的 UI（侧边栏、菜单） | 常驻内存，仅切换可见性。 |
| `AnimatedPresence` | 动态内容（列表项、条件渲染） | 物理增删节点。 |

两者共享同一套 API：
- `cases`: 定义 `true`（展现）和 `false`（隐藏）时的视觉状态。
- `enter` / `leave`: 可选的进场/出场过渡。

### 时间轴与序列
- `keyframes!`: 编排精确的中间关键帧。
- `Stagger`: 为多个元素提供错峰入场效果。

---

## 快速决策指南

1. **内容出现/消失动画？** → `AnimatedPresence` (或常驻节点的 `AnimatedVisibility`)。
2. **枚举状态驱动多个样式？** → `@AnimateMatch`。
3. **单个属性改写时平滑？** → `transition(...)`。
4. **手动控制开始/停止？** → `@Animate`。
5. **复杂序列或时间轴？** → `keyframes!` 或 `Stagger`。
