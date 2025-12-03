# Widget 系统

Ribir 的 Widget 系统建立在三个核心 trait 之上：`Render`、`Compose` 和 `ComposeChild`。理解这些 trait 是创建自定义 Widget 和理解 Ribir 如何构建 UI 树的关键。

## 1. Render 与 Compose

### `Render` trait

`Render` 是实际在屏幕上绘制内容或直接管理布局的 Widget 的低级接口。如果一个 Widget 是绘制像素的“叶子”节点（如 `Text` 或 `Rectangle`），或者是计算其子项位置的容器（如 `Row` 或 `Column`），它就会实现 `Render`。

`Render` 的主要职责:
- **布局**: 计算其自身大小和子项位置（`perform_layout`）。
- **绘制**: 将内容绘制到画布上（`paint`）。
- **命中测试**: 确定点是否与 Widget 交互（`hit_test`）。

```rust ignore
// Render Widget 的简化概念
impl Render for MyCustomPainter {
    fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
        // 计算大小...
        Size::new(100., 100.)
    }

    fn paint(&self, ctx: &mut PaintingCtx) {
        // 绘制某些内容...
        let rect = Rect::from_size(ctx.box_size().unwrap());
        ctx.painter().rect(&rect).fill();
    }
}
```

### `Compose` trait

`Compose` 是用于通过组合其他 Widget 构建 UI 的高级 Widget。它们自身不绘制任何内容，只是展开为其他 Widget 的树。这类似于 React 或 Vue 中的“组件”。

大多数应用程序级 Widget（如 `UserProfile` 或 `LoginForm`）实现 `Compose`。

```rust no_run
use ribir::prelude::*;

#[derive(Declare)]
pub struct WelcomeCard;

impl Compose for WelcomeCard {
    fn compose(this: impl StateWriter<Value = Self>) -> Widget<'static> {
        fn_widget! {
            @Column {
                @Text { text: "Welcome!" }
                @Button { @{ "Click me" } }
            }
        }.into_widget()
    }
}
```

## 2. ComposeChild 与子项结构

Ribir 使用严格的父子关系系统。不是所有 Widget 都可以接受子项，有些接受特定类型的子项。

### `ComposeChild` trait

`ComposeChild` 是 `Compose` 的变体，用于包装或修改子 Widget。它定义了父项和子项如何组合。

```rust ignore
pub trait ComposeChild<'c>: Sized {
    type Child;
    fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'static>;
}
```

### SingleChild 与 MultiChild

`SingleChild` 和 `MultiChild` trait 用于标识 Widget 接受子项数量的类型，通常用于布局器。

- **SingleChild**: 接受恰好一个子项的 Widget。
  - 示例：`SizedBox`、`Padding`、`Container`。
  - 在 DSL 中：`@Container { @Text { ... } }`

- **MultiChild**: 接受子项列表的 Widget。
  - 示例：`Row`、`Column`、`Stack`。
  - 在 DSL 中：
    ```rust ignore
    @Column {
        @Text { ... }
        @Text { ... }
    }
    ```

通常只需在定义类型时指定即可：
```rust ignore
#[derive(SingleChild, Declare)]
pub struct Container;
```

```rust ignore
#[derive(MultiChild, Declare)]
pub struct Row;
```