---
sidebar_position: 6
---

# 布局系统

Ribir 的布局系统采用“约束向下，尺寸向上”的单遍模型。这与 Flutter 的布局模型非常相似，旨在实现高效且灵活的 UI 布局。

## 核心原则

1.  **约束向下**: 父 Widget 将布局约束（Constraints）向下传递给子 Widget。这些约束定义了子 Widget 可以占用的最小和最大宽度和高度。
2.  **尺寸向上**: 子 Widget 根据接收到的约束计算自己的尺寸，并将最终确定的尺寸（Size）返回给父 Widget。
3.  **父项设置位置**: 在接收子 Widget 的尺寸后，父 Widget 确定子 Widget 在其自身坐标系中的位置。

## BoxClamp

布局约束由 `BoxClamp` 结构表示。它包含四个值：
*   `min_width`, `max_width`
*   `min_height`, `max_height`

`BoxClamp` 定义了一个允许的尺寸范围。子 Widget 的最终尺寸必须在此范围内。

*   **宽松约束**: `min` 为 0，`max` 为某个有限值。子 Widget 可以是 0 和最大值之间的任何尺寸。
*   **严格约束**: `min` 等于 `max`。子 Widget 被强制为特定尺寸。
*   **无界约束**: `max` 为无穷大。子 Widget 可以无限扩展（通常出现在滚动容器中）。

## 布局过程

每个 Widget 都必须在 `Render` trait 中实现 `perform_layout` 方法：

```rust ignore
fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size
```

在此方法中，Widget 需要做三件事：
1.  **布局子项**: 遍历其子节点，为每个子节点计算新的 `BoxClamp`（基于传入的 `clamp` 和其自身的布局逻辑），并调用 `ctx.perform_child_layout(child, child_clamp)`。
2.  **确定位置**: 获取子节点返回的 `Size`，并根据布局逻辑设置子节点的位置 `ctx.update_position(child, position)`。
3.  **返回尺寸**: 计算并返回自身的最终 `Size`，并且此尺寸必须满足传入的 `clamp` 约束。

## 使用 `clamp` 属性干预布局

Ribir 提供了一个内置的 `clamp` 属性，允许您在声明它时直接修改 Widget 接收的父约束。这在后台通过包装 `ConstrainedBox` 实现。

```rust no_run
use ribir::prelude::*;

fn example() -> Widget<'static> {
    fn_widget! {
        @Container {
            size: Size::new(100., 100.),
            background: Color::RED,
            // 强制约束：不管父级给出什么约束，Container 的宽度必须在 50 到 200 之间
            clamp: BoxClamp {
                min: Size::new(50., 0.),
                max: Size::new(200., f32::INFINITY),
            }
        }
    }.into_widget()
}
```

**注意**: `clamp` 属性的作用是**进一步限制**从父级向下传递的约束，取交集。

## 常见布局 Widget

*   **Row / Column**: 线性布局。在主轴方向提供无界约束（如果允许滚动或自适应），在交叉轴传递宽松或严格约束。
*   **Stack**: 堆叠布局。为所有非定位子节点传递相同的约束。
*   **Wrap**: 流式布局。自动换行。
*   **SizedBox**: 强制子节点为特定尺寸（通过应用严格约束）。

## 自定义布局示例

如果您需要实现自定义布局 Widget，您需要实现 `Render` trait。这里是一个简单的示例，强制子节点为固定尺寸（类似于 `SizedBox`）：

```rust no_run
use ribir::prelude::*;

#[derive(SingleChild, Declare, Clone)]
struct FixedSizeBox {
    size: Size,
}

impl Render for FixedSizeBox {
    fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
        // 1. 确定我们想要的尺寸，必须在父约束范围内
        let my_size = clamp.clamp(self.size);

        // 2. 如果有子节点，也将子节点强制为此尺寸
        if let Some(child) = ctx.single_child() {
             // 创建一个严格约束
            let child_clamp = BoxClamp { min: my_size, max: my_size };

            // 布局子节点
            ctx.perform_child_layout(child, child_clamp);

            // 设置子节点位置（通常为 (0,0)）
            ctx.update_position(child, Point::zero());
        }

        // 3. 返回最终尺寸
        my_size
    }
}
```

通过理解 `BoxClamp` 和 `perform_layout`，您可以完全控制 UI 的布局行为。