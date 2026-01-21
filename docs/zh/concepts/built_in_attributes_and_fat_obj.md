---
sidebar_position: 3
---

# 内置属性和 FatObj

Ribir 提供了一个强大的内置属性系统，让您可以为任何 Widget 添加常用功能，如布局控制（margin、alignment）、视觉效果（background、border、opacity、transform）和交互事件（on_tap、on_hover）。这些功能并非由每个 Widget 单独实现，而是通过一个称为 `FatObj` 的通用包装器统一提供。

## `@` 实例化过程

当您在 `fn_widget!` 中使用 `@` 语法通过类型来声明（例如 `@Text { ... }`）时，Ribir 会执行以下步骤来构建 Widget：

1.  **获取 Builder**: 调用 `Declare` trait 的 `declarer()` 方法来获取该 Widget 的 Builder。
2.  **初始化字段**: 对于 `{ ... }` 块中指定的每个字段，调用 Builder 上相应的 `with_xxx()` 方法（例如 `with_text(...)`）。
3.  **完成构建**: 最后，调用 Builder 的 `finish()` 方法（Builder 实现了 `ObjDeclarer` trait）来完成构建并返回已声明的 Widget。

### `#[declare]` 选项

`#[declare]` 宏支持多项选项来自定义其行为：

- **默认**: 生成一个完整的 Builder，返回 `FatObj<Stateful<T>>`，启用所有内置属性和响应式状态。
- **`#[declare(stateless)]`**: 生成一个完整的 Builder，返回 `FatObj<T>`。支持内置属性，但 Widget 本身不是有状态的。
- **`#[declare(simple)]`**: 生成一个简化的 Builder，返回 `Stateful<T>`（如果 struct 没有字段则返回 `T`）。这适用于不需要内置属性的 Widget。
- **`#[declare(simple, stateless)]`**: 与 `simple` 类似，但始终返回原始对象 `T`。
- **`#[declare(validate)]`**: 为 Widget 启用 `declare_validate` 验证方法。

> [!NOTE]
> `#[simple_declare]` 现在已被废弃，推荐使用 `#[declare(simple)]`。

## 什么是 FatObj？

`FatObj<T>` 是 Ribir 核心库中的一个泛型结构体，其作用是在构建阶段临时包装一个 Widget，并为其附加各种内置属性，例如 `margin`、`background`、`on_tap` 等。

### 工作原理

1.  **惰性初始化**: `FatObj` 在内部维护所有内置属性（如 `margin`、`padding` 等）的状态，但它们默认为空。只有在您显式使用某个属性时，相关的 Widget 才会被初始化。这确保了未使用的功能不会产生额外的性能开销。
2.  **组合**: 在 Widget 构建的最后阶段，`FatObj` 会将它包装的 Widget 与已启用的内置功能（如 `Padding`、`Container`、`MixBuiltin` 等）组合成最终的 Widget 树。

例如：`Margin(MixBuiltin(Text))`

## 常见内置属性

内置属性主要分为两类：**属性**和**事件**。

### 1. 属性

这些属性用于控制 Widget 的外观和布局。

*   **布局**:
    *   `margin`: 设置外边距。
    *   `padding`: 设置内边距。
    *   `x` / `y`: 使用 `PosX` 和 `PosY` 设置水平/垂直定位。
    *   `clamp`: 强制对 Widget 大小范围的约束（布局约束）。
    *   `box_fit`: 控制子元素如何适应容器空间（如填充、包含等）。
    *   `scrollable`: 控制 Widget 的滚动行为（X轴、Y轴或两者）。
    *   `layout_box`: 控制布局框行为。

*   **视觉**:
    *   `background`: 设置背景（颜色或图像）。
    *   `foreground`: 设置前景（通常覆盖在内容之上）。
    *   `border`: 设置边框。
    *   `box_shadow`: 设置盒阴影（围绕盒子的外部阴影效果）。
    *   `radius`: 设置边框半径。
    *   `backdrop`: 设置背景效果。
    *   `opacity`: 设置不透明度。
    *   `visible`: 控制可见性。
    *   `transform`: 应用图形变换（平移、旋转、缩放）。
    *   `cursor`: 设置悬停时的光标样式。
    *   `backdrop_filter`: 应用背景滤镜效果（如模糊）。
    *   `filter`: 应用视觉滤镜效果（模糊、灰度、亮度等）。
    *   `clip_boundary`: 是否裁剪边界之外的内容。
    *   `painting_style`: 设置绘画样式（填充或描边）。

*   **文本**（通常由子节点继承）：
    *   `text_style`: 设置字体样式。
    *   `text_align`: 设置文本对齐。
    *   `text_line_height`: 设置行高。
    *   `font_size`: 设置字体大小。
    *   `font_face`: 设置字体系列。

*   **其他**:
    *   `keep_alive`: 保持 Widget 状态即使从视图中移除。
    *   `tooltips`: 设置工具提示文本。
    *   `disabled`: 禁用 Widget 及其子项的交互。
    *   `providers`: 为 Widget 设置提供者上下文。
    *   `class`: 应用样式类。
    *   `reuse`: 通过设置 reuse 属性，可以重用 Widget。如果是局部的复用，可结合 `LocalWidgets` 使用。

### 2. 事件

这些属性用于处理用户交互。所有事件回调都会接收一个事件对象。

*   **指针事件**:
    *   `on_pointer_down`: 在指针（鼠标按钮、触摸接触、笔）按下时触发。
    *   `on_pointer_move`: 在指针移动时触发。
    *   `on_pointer_up`: 在指针释放时触发。
    *   `on_pointer_cancel`: 在指针事件被取消时触发（例如，触摸中断）。
    *   `on_pointer_enter`: 在指针进入 Widget 区域时触发。
    *   `on_pointer_leave`: 在指针离开 Widget 区域时触发。
    *   `on_tap`: 在点击或轻触（按下和释放序列）时触发。
    *   `on_tap_capture`: `on_tap` 的捕获阶段版本。
    *   `on_double_tap`: 在双击时触发。
    *   `on_triple_tap`: 在三击时触发。
    *   `on_x_times_tap`: 在指定次数的点击时触发。

*   **滚轮事件**:
    *   `on_wheel`: 在鼠标滚轮滚动时触发。
    *   `on_wheel_capture`: `on_wheel` 的捕获阶段版本。
    *   `on_wheel_changed`: 在滚轮增量变化时触发。

*   **键盘事件**:
    *   `on_key_down`: 在按键按下时触发。
    *   `on_key_down_capture`: `on_key_down` 的捕获阶段版本。
    *   `on_key_up`: 在按键释放时触发。
    *   `on_key_up_capture`: `on_key_up` 的捕获阶段版本。

*   **焦点事件**:
    *   `on_focus`: 在 Widget 获得焦点时触发。
    *   `on_blur`: 在 Widget 失去焦点时触发。
    *   `on_focus_in`: 在 Widget 或其后代之一获得焦点时触发（冒泡）。
    *   `on_focus_out`: 在 Widget 或其后代之一失去焦点时触发（冒泡）。

*   **生命周期事件**:
    *   `on_mounted`: 在 Widget 挂载到 Widget 树时触发。
    *   `on_performed_layout`: 在 Widget 布局完成后触发。
    *   `on_disposed`: 在 Widget 从 Widget 树中移除时触发。

*   **IME 事件**:
    *   `on_ime_pre_edit`: 在 IME 预编辑期间触发（例如，组成文本）。
    *   `on_chars`: 在接收文本字符时触发。

## 使用场景

### 场景 1: 声明一个新 Widget

在大多数情况下，widgets 都是用 `#[declare]` 宏定义的。这意味着您可以在使用 `@` 语法声明 Widget 时，直接使用内置属性。

例如，`Text` 组件本身并不包含 `margin` 或 `background` 字段，但通过 `#[declare]` 和 `FatObj` 机制，您可以在声明时直接使用它们：

```rust no_run
use ribir::prelude::*;

fn simple_card_traditional() -> Widget<'static> {
    fn_widget! {
        @Text {
            text: "Hello, Ribir!",
            // 内置属性：布局
            margin: EdgeInsets::all(10.),
            padding: EdgeInsets::symmetrical(10., 5.),
            x: AnchorX::center(),

            // 内置属性：视觉
            background: Color::from_u32(0xFFEEAA00),
            border: Border::all(BorderSide::new(2., Color::BLACK.into())),
            radius: Radius::all(4.),

            // 内置属性：交互
            on_tap: |_: &mut PointerEvent| println!("Card Tapped!"),
            cursor: CursorIcon::Pointer,
        }
    }.into_widget()
}
```

### 场景 2: 包装一个存在 Widget

当您需要为已经构建好的 Widget 实例（例如函数参数传入的 Widget，或变量中的 Widget）添加内置属性时，可以使用 `@FatObj { ... }` 语法。

```rust no_run
use ribir::prelude::*;

fn simple_card(w: Widget<'static>) -> Widget<'static> {
    fn_widget! {
        // 用 FatObj 包装 Widget 以添加内置属性
        @FatObj {
            margin: EdgeInsets::all(10.),
            padding: EdgeInsets::symmetrical(10., 5.),
            x: AnchorX::center(),
            background: Color::from_u32(0xFFEEAA00),
            border: Border::all(BorderSide::new(2., Color::BLACK.into())),
            radius: Radius::all(4.),
            on_tap: |_: &mut PointerEvent| println!("Card Tapped!"),
            cursor: CursorIcon::Pointer,
            // 嵌入子 Widget
            @ { w }
        }
    }.into_widget()
}
```

这种方式非常清晰且地道，推荐使用 `@FatObj { ... }` 而不是手动创建 `FatObj::new(w)`。

## FatObj 核心机制

### 内置属性的包装顺序

`FatObj` 按照固定顺序应用内置属性。这个顺序决定了最终 Widget 树的结构以及属性之间的相互作用方式。

从**内到外**的包装顺序如下（已简化为常见属性）：

1.  **内容**（被包装的 Widget）
2.  `padding`
3.  `foreground`
4.  `border`
5.  `background`
6.  `backdrop`
7.  `filter`
8.  `clip_boundary`
9.  `box_shadow`
10. `radius`
11. `scrollable`
12. `layout_box`
13. `providers`
14. `class`
15. `clamp` (constrained_box)
16. `tooltips`
17. `margin`
18. `cursor`
19. **事件** (`mix_builtin`: `on_tap`, `on_pointer_move` 等)
20. `transform`
21. `opacity`
22. `visibility`
23. `disabled`
24. `x` / `y` (position)
25. `keep_alive`
26. `reuse`

#### 关键要点

因为包裹是有顺序的，所以外层属性的影响范围会包含内层属性。如果设置了多个内建属性后发现效果不符合预期，可以尝试调整属性的顺序。

*   **事件包含margin**: 由于**事件**包装在**margin**外层，默认情况下 Widget 的交互区域包括其边距。
*   **变换影响所有内容**: `transform` 包装了大部分视觉和布局属性，因此旋转 Widget 时也会旋转其边距、背景和边框。
*   **可见性控制整体**: `visibility` 位于接近最外层的位置，因此将其设置为隐藏会隐藏整个 Widget，包括边距。

### 如何覆盖顺序？

有时默认的包装顺序可能不符合您的需求。例如，您可能希望点击区域（`on_tap`）**排除**边距。

由于 `FatObj` 按固定顺序应用属性，您可以通过手动嵌套 `FatObj` 来改变顺序。先应用内层属性，然后用另一个 `FatObj` 包装外层属性。

**示例：让点击区域排除边距**

如果您直接这样写:
```rust ignore
@FatObj {
    margin: EdgeInsets::all(20.),
    on_tap: |_| println!("Clicked!"),
    @ { w }
}
```
结构是 `MixBuiltin(Margin(w))`，因此点击边距也会触发事件。

要让点击区域排除边距，您需要的结构是 `Margin(MixBuiltin(w))`。可以这样实现：

```rust ignore
fn_widget! {
    // 外部 FatObj 处理边距
    @FatObj {
        margin: EdgeInsets::all(20.),
        // 内部 FatObj 处理点击事件
        @FatObj {
            on_tap: |_| println!("Clicked inside content (excluding margin)!"),
            @ { w }
        }
    }
}
```

通过嵌套 `FatObj`，您可以完全控制属性的组合顺序。

## 进阶：动态访问与修改

内置属性（如 `opacity`、`background`、`margin`）是 `FatObj` 的属性。在声明式 UI 中，您通常在创建时将这些属性绑定到状态。然而，**如果您需要在代码中动态修改它们（例如在事件处理程序内部）或者在 `pipe!` 中使用该字段，您必须访问该字段的 Writer。**

### 简单跟随示例（一个组件跟随另一个）

```rust no_run
use ribir::prelude::*;

/// 一个小示例，其中 `follower` 的 `background` 跟随 `leader` 的
/// `background`。点击 `leader` 会切换其背景颜色；`follower` 自动更新
/// 因为它通过 `pipe!($read(...))` 绑定到 `leader` 的 writer。
fn follower_example() -> Widget<'static> {
    fn_widget! {
        // 创建有状态的句柄，以便我们可以访问内置属性。
        let mut leader = @Text { text: "Leader (click me)" };

        // Follower: 将背景绑定到 leader 的背景，使其跟随。
        let follower = @Text { 
          text: "Follower (follows leader)",
          background: pipe!($read(leader.background()).clone()),
        };

        let seed = Instant::now();

        @Column {
            // Leader: 点击切换其背景色
           @(leader)  {
                cursor: CursorIcon::Pointer,
                on_tap: move |_| {
                    *$write(leader.background()) = Color::from_u32(seed.elapsed().as_millis() as u32).into();
                },
            }

            @ { follower }
        }
    }.into_widget()
}

```
说明：

- `leader` 是通过 `@` 创建的状态句柄。它的 `on_tap` 事件使用 `$write` 切换其 `background`。
- `follower` 通过 `pipe!($read(...)).map(...)` 将其 `background` 绑定到 `leader` 的 `background`，因此每当 leader 的背景发生变化时，它会自动更新。
- 当多个 Widget 应该在视觉上反映单一事实来源（主题、选择、焦点等）而不手动传播事件时，这种模式很有用。

## 总结

`FatObj` 是 Ribir 灵活性的关键。它让任何 Widget 都能拥有丰富的通用功能，同时保持核心 Widget 定义的简洁。通过内置属性，您可以快速构建美观且具有交互性的 UI，而无需为每个 Widget 重复实现这些基础功能。