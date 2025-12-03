# 声明式 UI

Ribir 使用基于 Rust 宏的声明式 DSL（领域特定语言）来定义用户界面。这使您能够描述 UI *应该是什么样子*，而不是 *如何* 一步步构建它。

`fn_widget!` 宏是这个 DSL 的核心。

## `fn_widget!` 宏

`fn_widget!` 是编写 Ribir UI 代码的入口点。它将 DSL 语法转换为构建 Widget 树的实际 Rust 代码。

```rust no_run
use ribir::prelude::*;

fn main() {
    App::run(fn_widget! {
        @Text { text: "Hello!" }
    });
}
```

## 使用 `@` 创建 Widget

要实例化一个 Widget，请使用 `@` 符号后跟 Widget 的类型名称。属性在花括号 `{}` 内使用标准的 Rust 结构初始化语法 `key: value` 定义。

当 `@` 直接跟随一个类型时，它会调用相应的 Builder 来构造对象。这个 Builder 通常由 `#[derive(Declare)]` 宏生成，从而支持内置属性的使用。我们将在 [内置属性和 FatObj](./built_in_attributes_and_fat_obj.md) 部分详细探讨这一机制。

**重要**: `@` 运算符是 **DSL 专用的**，只在支持 Ribir DSL 语法的宏中有效，如 `fn_widget!` 和 `rdl!`。在这些宏之外，该运算符不是有效的 Rust 语法，如果在常规 Rust 代码或第三方宏中使用，将导致编译错误。

```rust no_run
use ribir::prelude::*;

fn example() -> Widget<'static> {
    fn_widget! {
        @Text { text: "I am a Text widget" }
    }.into_widget()
}
```

## 父子组合

Ribir 将 UI 表示为一棵树。您可以通过嵌套来组合它们。支持子节点的 Widget 允许您直接在其代码块内声明它们。

```rust no_run
use ribir::prelude::*;

fn composition_example() -> Widget<'static> {
    fn_widget! {
        @Column {
            @Text { text: "Item 1" }
            @Text { text: "Item 2" }
            @Button {
                @ { "Click Me" }
            }
        }
    }.into_widget()
}
```

这里 `Column` 是一个支持多个子节点的 Widget（`MultiChild`，参见 [Widget 组合](./widgets_composition.md)），允许您直接在其代码块内声明子节点。
`Button` 是一个支持模板子节点的 Widget（`TemplateChild`，参见 [Widget 组合](./widgets_composition.md)）。它通过类型匹配自动设置对应属性，因此设置文本时使用 `@ { "Click Me" }`，而无需使用 `text: "Click Me"`。

## 复用 Widget（静态组合）

当你有一个 `Widget` 表达式时，你可以将它赋值给变量或从函数返回，然后在另一个 `fn_widget!` 块中使用。这有助于提高代码的复用性。

要将 Widget 变量或表达式嵌入到 DSL 中，请使用 `@ { expression }` 语法。

```rust no_run
use ribir::prelude::*;

fn header() -> Widget<'static> {
    fn_widget! {
        @Text { text: "My App Header" }
    }.into_widget()
}

fn app() -> Widget<'static> {
    let footer = fn_widget! {
        @Text { text: "Footer Content" }
    };

    fn_widget! {
        @Column {
            @ header() // 嵌入返回 Widget 的函数
            @Text { text: "title" }
            @fn_widget!{ @Text { text: "Main Content" } }
            @ { footer }   // 嵌入 Widget 变量
        }
    }.into_widget()
}
```


**注意:** `fn_widget` 顾名思义 — 它是一个返回 widget 的函数。虽然它在语义上可以被当作 `Widget` 使用，但它实际上是一个函数 `fn -> FatObj<Stateful<Text>>`，其调用时机取决于框架的构建流程。因此，`@fn_widget!{ @Text { text: "Main Content" } }` 与 `@Text { text: "title" }` 不同，后者是一个 `FatObj<Stateful<Text>>`。使用 `let text = @Text { text: "title" }` 时，我们可以访问 `Text` 的结构字段（例如 `$read(text).text`），但对 `@fn_widget!{ @Text { text: "Main Content" } }` 则无法这么做。

## 动态 Widget

Ribir 允许您创建能够在数据变化时自动更新的 Widget。`pipe!` 宏是实现这一功能的关键工具，它会创建一个值流，并可将其转换为 Widget。

要创建动态 Widget，您可以使用 `pipe!` 宏，并通过 `@ { ... }` 语法将其嵌入到 UI 中。

```rust no_run
use ribir::prelude::*;

fn dynamic_widget_example() -> Widget<'static> {
    let count = Stateful::new(0);

    fn_widget! {
        @Column {
            @{
                pipe!(*$read(count)).map(move |c| {
                    if c % 2 == 0 {
                        @H1 { text: "Even" }.into_widget()
                    } else {
                        @H2 { text: "Odd" }.into_widget()
                    }
                })
            }
            @Button {
                on_tap: move |_| *$write(count) += 1,
                @{ "Increment" }
            }
        }
    }.into_widget()
}
```
这里有两点需要注意：
1. `pipe!` 管道根据条件返回不同类型的 Widget，可以使用 `.into_widget()` 将它们统一为单一的 `Widget` 类型。
2. `pipe!` 会监控表达式中使用的状态变量（如 `$read(count)`）。当状态发生变化时，表达式会重新计算，Widget 也会随之更新。

示例中使用了 `.map()` 对监听到的结果进行转换，构建出对应的 Widget。
不过 Ribir 也支持将所有操作都放在 `pipe!` 中，如下所示：
```rust ignore
@ {
    pipe! {
        if *$read(count) % 2 == 0 {
            @H1 { text: "Even" }.into_widget()
        } else {
            @H2 { text: "Odd" }.into_widget()
        }
    }
}
```
Ribir 会自动分析并监听 `pipe!(expr)` 中的 State 变化，并在变化时重新求值 `expr`。**但是 `pipe!(state_expr)).map(move|v| expr)` 可以明确指定监听的 State，在复杂场景下会有更好的性能,是更推荐的方式**。