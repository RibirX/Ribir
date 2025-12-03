# 指南

本指南介绍 Ribir 的基本概念和使用模式，Ribir 是一个非侵入式的 Rust GUI 框架，让您能够从单一代码库构建跨平台应用程序。

## 核心概念

### 声明式 UI
Ribir 使用声明式方法进行 UI 开发，您可以根据当前状态描述 UI 应该是什么样子。当状态改变时，框架会自动处理渲染和更新。

### Widget 和组合
Ribir 中的一切都是由 Widget 构建的。您可以使用 `fn_widget!` 宏和 `@` 语法将简单的 Widget 组合成复杂的 UI：

```rust no_run
use ribir::prelude::*;
fn_widget! {
  @Container {
    size: Size::new(100., 100.),
    background: Color::RED,
    @Text { text: "Hello, Ribir!" }
  }
};
```

### 状态管理
Ribir 通过 `Stateful` 对象提供响应式状态管理。您可以使用 `$read` 读取状态，使用 `$write` 写入状态，并使用 `pipe!` 和 `watch!` 创建响应式绑定：

```rust no_run
use ribir::prelude::*;
let counter = Stateful::new(0);
fn_widget! {
  @Column {
    @Text { text: pipe!($read(counter).to_string()) }
    @Button { 
      on_tap: move |_| *$write(counter) += 1,
      @{ "Increment" }
    }
  }
};
```

### 布局系统
Ribir 的布局系统使用约束向下流动到 Widget 树，尺寸信息向上回流。`clamp` 属性允许您控制 Widget 在可用空间内如何调整尺寸。

### 数据共享和事件
使用 `Provider` 通过 Widget 树自上而下地共享数据，使用 `自定义事件` 从子 Widget 向上冒泡事件到父 Widget。

## 入门

要开始使用 Ribir 构建应用，请详细了解这些核心概念：
- **[声明式 UI](./core_concepts/declarative_ui.md)**: 了解 DSL 语法和 Widget 组合
- **[内置属性和 FatObj](./core_concepts/built_in_attributes_and_fat_obj.md)**: 使用内置功能
- **[状态管理](./core_concepts/state_management.md)**: 管理应用程序状态
- **[数据共享和事件](./core_concepts/data_sharing_and_events.md)**: 组件间的通信
- **[Widget 系统](./core_concepts/widgets_composition.md)**: 了解架构
- **[布局系统](./core_concepts/layout.md)**: 控制 UI 的排列方式

一旦您熟悉了这些基础知识，可以继续学习高级主题以扩展您的知识：
- **[自定义 Widget](./advanced/custom_widgets.md)**: 构建可重用组件
- **[动画](./advanced/animations.md)**: 为 UI 添加动态效果
- **[主题](./advanced/theming.md)**: 自定义外观和风格
- **[Widget 组件](./advanced/widgets.md)**: 详细了解 Widget 组件