---
sidebar_position: 1
---

# Widget 组件

Widget 是 Ribir 应用程序的构建块。在 Ribir 中，一切都是 Widget，从最基本的文本节点到复杂的布局容器，再到完整的应用程序 - 所有这些都是通过组合不同的 Widget 构建的。

本文档介绍 Ribir 提供的核心和常用 Widget，以帮助您快速构建用户界面。在阅读本文档之前，建议首先了解 [声明式 UI](../core_concepts/declarative_ui.md) 和 [内置属性](../core_concepts/built_in_attributes_and_fat_obj.md) 的概念。

## Widget 类别

在 Ribir 中，Widget 主要分为两类（详情请参见 [自定义 Widget](./custom_widgets.md)）：

1.  **Compose Widget**: 通过组合其他 Widget 构建 UI。这是最常见的 Widget 类型，例如 `Button`、`List` 等。
2.  **Render Widget**: 负责特定的布局和绘制逻辑。例如 `Text`、`Container` 等。

无论类型如何，您都可以使用统一的 `fn_widget!` 宏和 `@` 语法来声明和使用它们。

## 通用功能（FatObj）

所有 Widget 都通过 `FatObj` 机制获得一组通用的内置属性。这意味着您可以直接在任何 Widget 上使用布局（如 `margin`）、视觉（如 `background`、`border`）和交互（如 `on_tap`）属性，而无需 Widget 本身显式实现这些功能。

有关详细列表，请参见 [内置属性](../core_concepts/built_in_attributes_and_fat_obj.md)。

```rust no_run
use ribir::prelude::*;

fn common_props_example() -> Widget<'static> {
    fn_widget! {
        @Text {
            text: "I am a text with background and margin",
            // 直接使用的内置属性
            margin: EdgeInsets::all(10.),
            background: Color::YELLOW,
            padding: EdgeInsets::all(5.),
            border: Border::all(BorderSide::new(1., Color::BLACK.into())),
        }
    }.into_widget()
}
```

## 核心 Widget

这些 Widget 由 `ribir_core` 提供，构成 UI 的基本骨架。

### 基本显示

*   **`Text`**: 用于显示文本。支持丰富的文本样式、对齐等。
    *   属性：`text`（内容）、`text_style`（样式）、`text_align`（对齐）。
    *   **排版 Widget**：`H1`、`H2`、`H3`、`H4`、`H5`、`H6` 是具有预设主题样式的 `Text` 变体。

### 布局容器

*   **`Container`**: 最常用的盒模型容器。可以设置大小、背景、边框、半径等。虽然这些也可以通过内置属性设置，但在组合它们时（如需要特定大小和背景）显式使用 `Container` 更清晰。
*   **`SizedBox`**: 强制固定大小的盒子。通常用作占位符或强制子元素大小。
*   **`ConstrainedBox`**: 对子元素应用额外的布局约束（如最大/最小宽度/高度）。
*   **`UnconstrainedBox`**: 从父元素对子元素移除某些约束，允许子元素以自己的大小绘制。

### 线性布局

*   **`Row`**: 水平排列子元素。
*   **`Column`**: 垂直排列子元素。
*   **`Flex`**: 更通用的灵活布局容器，`Row` 和 `Column` 都是基于它的包装。
    *   关键属性：`align_items`（交叉轴对齐）、`justify_content`（主轴对齐）、`wrap`（是否换行）。

### 弹性控制

*   **`Expanded`**: 在 `Row`、`Column` 或 `Flex` 中使用，强制子元素填充剩余空间。

### 堆叠布局

*   **`Stack`**: 允许子元素重叠放置。通常与 `Positioned`（通过内置属性 `anchor` / `global_anchor` 实现）一起使用。

### 变换与效果

*   **`TransformWidget`**: 对子元素应用矩阵变换（平移、旋转、缩放）。通常通过内置的 `transform` 属性使用。
*   **`Opacity`**: 设置子元素的不透明度。通常通过内置的 `opacity` 属性使用。
*   **`Clip`**: 裁剪子元素的内容。有 `ClipRect`、`ClipRRect`、`ClipPath` 等变体。

## 常见 Widget

这些 Widget 位于 `ribir_widgets` 库中，提供丰富的高级 UI 控件。

### 按钮

Ribir 提供了一系列符合常见设计规范的按钮：

*   **`Button`**（或 `OutlinedButton`）：带边框的按钮，用于次要操作。
*   **`FilledButton`**: 具有填充背景色的按钮，用于主要操作。
*   **`TextButton`**: 纯文本按钮，用于低优先级操作。
*   **`Fab`**: 浮动操作按钮。

所有按钮都支持灵活的内容组合，可以只包含文本、只包含图标，或两者都包含。

```rust no_run
use ribir::prelude::*;

fn buttons_example() -> Widget<'static> {
    fn_widget! {
        @Row {
            @FilledButton { @{ "Primary Action" } }
            @Button { @{ "Secondary Action" } }
            @TextButton { @{ "Cancel" } }
            // 带图标的按钮
            @FilledButton {
                @Icon { @ { svg_registry::get_or_default("add") } }
                @ { "New" }
            }
        }
    }.into_widget()
}
```

### 输入

*   **`Input`**: 基本文本输入框。
*   **`Checkbox`**: 复选框。
*   **`Switch`**: 开关控件。
*   **`Radio`**: 单选按钮。
*   **`Slider`**: 滑块，用于选择数字范围。

### 列表

*   **`List`**: 垂直列表容器，支持单选和多选模式。
*   **`ListItem`**: 标准列表项，包括前导图标（Leading）、主标题（Headline）、副标题（Supporting）和尾随控件（Trailing）。
*   **`Divider`**: 分隔线。

```rust no_run
use ribir::prelude::*;

fn list_example() -> Widget<'static> {
    fn_widget! {
        @List {
            @ListItem {
                @ListItemHeadline { @{ "List Item 1" } }
                @ListItemSupporting { @{ "This is description information" } }
            }
            @Divider {}
            @ListItem {
                @ListItemHeadline { @{ "List Item 2" } }
                @Trailing { @Switch { checked: true } }
            }
        }
    }.into_widget()
}
```

### 导航与菜单

*   **`Tabs`**: 选项卡切换组件。
*   **`Menu`**: 弹出菜单。

### 显示

*   **`Icon`**: 图标组件，通常与 SVG 一起使用。
*   **`Avatar`**: 头像组件，支持图像或字符。
*   **`Badge`**: 徽章，通常附加到其他 Widget 的角落以显示通知计数。
*   **`Progress`**: 进度条（线性或圆形）。

### 滚动

*   **`Scrollable`**: 为子内容提供滚动功能。通常您可以直接在任何 Widget 上使用内置属性 `scrollable: Scrollable::X` 或 `Scrollable::Y` 启用它。
*   **`Scrollbar`**: 显式滚动条组件。

## 总结

Ribir 提供了丰富的组件库。结合强大的组合能力（`Compose`）和通用属性系统（`FatObj`），您可以高效地构建复杂且美观的用户界面。

*   对于基本布局和绘制，主要使用 `ribir_core` 中的 Widget。
*   对于常见的交互控件，首先检查 `ribir_widgets` 中是否有实现。
*   如果现有组件不能满足您的需求，您可以轻松通过 `fn_widget!` 组合现有组件，或实现 `Render` trait 创建全新的自定义组件。