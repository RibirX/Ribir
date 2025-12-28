---
sidebar_position: 4
---

# 状态管理

Ribir 采用数据驱动的方式进行状态管理。您无需手动更新 Widget，只需修改数据（状态），Ribir 就会自动更新依赖于该数据的 UI 部分。

## Stateful 对象

Ribir 中状态的核心原语是 `Stateful<T>`。它包装一块数据 `T` 并使其可观察。当 `Stateful` 对象内的数据被修改时，Ribir 会通知并更新所有依赖于该数据的 UI 部分。

要创建一个状态对象，请使用 `Stateful::new(value)`：

```rust ignore
use ribir::prelude::*;

fn main() {
    let count = Stateful::new(0);
}
```

`Stateful<T>` 实际上实现了 `StateReader<T>`、`StateWatcher<T>` 和 `StateWriter<T>` trait，正是通过这些 trait 提供了对状态的访问。

### StateReader<T>

`StateReader<T>` trait 提供了对状态的只读访问。通过 `StateReader<T>` 的实现，您可以获取对状态的读引用。

### StateWatcher<T>

`StateWatcher<T>` trait 提供了对状态的只读访问。但不同于 `StateReader<T>`，通过 `StateWatcher<T>` 的实现，您获取到对宿主 `T` 的状态变化的订阅（即当宿主 `T` 的状态发生变化时，您将收到通知）。

### StateWriter<T>

`StateWriter<T>` trait 提供了对状态的写访问。通过 `StateWriter<T>` 的实现，您可以获取对宿主 `T` 的状态的写引用，当完成对 mut ref 的修改后，Ribir 会自动通知所有依赖于该数据的 UI 部分。

## 读写状态

在 `fn_widget!` DSL 中，您可以使用特定的语法助手来访问状态：

- **`$read(state)`**: 通过 `StateReader<T>` 获取对状态的读引用。
- **`$write(state)`**: 通过 `StateWriter<T>` 获取对状态的写引用。通过此引用修改数据将触发更新。
- **`$reader(impl StateReader<T>)`**: 获取 `StateReader<T>` 的克隆，通常用于在闭包中持有读权限。
- **`$watcher(impl StateWatcher<T>)`**: 获取 `StateWatcher<T>` 的克隆，通常用于在闭包中持有订阅权限。
- **`$writer(impl StateWriter<T>)`**: 获取 `StateWriter<T>` 的克隆，通常用于在闭包中持有写权限。
- **`pipe!(expr)`**: 捕捉 expr 中通过 `$read`、`$write` 对状态的访问，通过 `StateWatcher<T>` 订阅状态变化，调用 expr 并返回 expr 的值。
- **`watch!(expr)`**: 捕捉 expr 中通过 `$read`、`$write` 对状态的访问，通过 `StateWatcher<T>` 订阅状态变化，并调用 expr。

**重要**: `$read`、`$write`、`pipe!`、`watch!`、`$reader`、`$watcher`、`$writer` 只在 Ribir 的 DSL 内工作，如 `fn_widget!` 和 `rdl!`。这些运算符在这些宏之外不是有效的 Rust 语法，如果在常规 Rust 代码或第三方宏中使用，将导致编译错误。

**注意**: 在 DSL 宏之外，您可以在 `Stateful` 对象上使用 `.read()` 和 `.write()` 方法，但这些不会自动建立响应式依赖。

## 第三方宏中的 DSL 运算符

DSL 运算符（`@`、`$read`、`$write` 等）在嵌套在第三方宏内时**无效**。这是因为我们无法预期第三方宏的处理逻辑。例如：

**❌ 错误用法:**
```rust ignore
fn_widget! {
    ...
    // 这不会生效 - $read 由 println! 处理,它不理解 DSL 语法
    println!("{}", $read(some_state));
    ...
}
```

**✅ 正确用法:**
```rust ignore
fn_widget! {
    ...
    let val = $read(some_state);
    println!("{}", val);
    ...
}
```

## `fn_widget!` 闭包中的状态访问

在 `fn_widget!` 中使用事件处理程序（如 `on_tap`）时，您经常需要修改状态。Ribir 的辅助宏（`$write`、`$read`、`$writer`、`$reader`、`$watcher`）旨在与 `move` 闭包无缝配合。

它们会自动检测是否在闭包内使用,并处理底层状态写入器/读取器的必要克隆。这意味着您很少需要在闭包之前手动调用 `.clone_writer()`。

**详细（旧方法）：**
```rust ignore
let writer = state.clone_writer();
@Button {
    on_tap: move |_| {
        *writer.write() += 1; // 在此处使用手动克隆的写入器
    }
}
```

**简化方式（推荐）:**
```rust ignore
@Button {
    on_tap: move |_| {
        *$write(state) += 1; // $write 自动处理克隆
    }
}
```

## 使用 `pipe!` 的响应式绑定

`pipe!` 宏是将状态绑定到 Widget 属性的主要方式。它会求值一个表达式，并在表达式内用 `$read` 或 `$write` 标记的任何状态发生变化时重新求值。

`pipe!` 创建单向数据流：从状态到视图。

```rust no_run
use ribir::prelude::*;

fn counter_example() -> Widget<'static> {
    fn_widget! {
        let count = Stateful::new(0);

        @Column {
            // 将文本属性绑定到 count 状态
            @Text {
                text: pipe!($read(count).to_string())
            }
            @Button {
                // 点击时增加 count
                on_tap: move |_| *$write(count) += 1,
                @Text { text: "Increment" }
            }
        }
    }.into_widget()
}
```

在此示例中：
1. `pipe!($read(count).to_string())` 创建一个动态值。
2. 最初，它读取 `count`（0）并返回 "0"。
3. 当 `on_tap` 执行 `*$write(count) += 1` 时，`count` 变化。
4. `pipe!` 检测到变化，重新运行 `.to_string()`，并更新 `Text` Widget。

### 重要:避免在 `pipe!` 表达式中使用 `BuildCtx`

`pipe!` 表达式会在其依赖的状态变化时重新求值。然而，`BuildCtx`（构建上下文）仅在 Widget 的构建阶段有效。在 `pipe!` 表达式中使用 `BuildCtx::get()` 会在表达式重新求值时导致运行时错误，因为它尝试访问无效的上下文。

 > [!警告]
 > **运行时错误风险**: 永远不要在 `pipe!` 表达式内直接使用 `BuildCtx::get()`。当管道更新时它会崩溃。请参见 [故障排除](../getting_started/troubleshooting.md#buildctxget-inside-pipe) 了解详情。

**错误示例：**

```rust no_run
/// 这是一个错误示例，会导致运行时错误！
use ribir::prelude::*;

fn bad_example() -> Widget<'static> {
    fn_widget! {
        let count = Stateful::new(0);
        @Text {
            // 错误：BuildCtx::get() 在 pipe! 重新评估时可能无效
            text: pipe!(*$read(count)).map(move |c| format!("tap {} on windows {:?}", c, BuildCtx::get().window().id())),
            on_tap: move |_| *$write(count) += 1,
        }
    }.into_widget()
}
```

**正确方法:**

如果需要访问来自 `BuildCtx` 的信息，请在 `fn_widget!` 的顶层捕获它，并在 `pipe!` 表达式中将其作为依赖或常量使用。

```rust no_run
use ribir::prelude::*;

fn good_example() -> Widget<'static> {
    fn_widget! {
        let count = Stateful::new(0);
        // 在构建阶段捕获窗口 ID，并在 pipe! 中用作常量
        let window_id = BuildCtx::get().window().id();
        @Text {
            text: pipe!(*$read(count)).map(move |c| format!("tap {} on windows {:?}", c, window_id)),
            on_tap: move |_| *$write(count) += 1,
        }
    }.into_widget()
}
```

## 使用 `watch!` 响应变化

虽然 `pipe!` 用于将值绑定到属性，但 `watch!` 用于在状态变化时执行操作（如日志记录、网络请求或复杂逻辑）。

`watch!` 创建一个可观察流（rxRust 流）。您必须 `.subscribe()` 它来执行代码。

```rust no_run
use ribir::prelude::*;

fn watch_example() {
    let count = Stateful::new(0);

    // 监视变化并打印它们
    let _subscription = watch!(*$read(count))
        .subscribe(|val| println!("Count changed to: {}", val));

    *count.write() = 1; // 打印：Count changed to: 1
    *count.write() = 2; // 打印：Count changed to: 2
}
```

### `pipe!` 与 `watch!`

- **`pipe!(expr)`**: 返回一个值和一个流，用于**初始化和绑定状态**。它总是有初始值。
- **`watch!(expr)`**: 返回一个流。它用于**在状态变化时执行操作**。您必须显式订阅它。

## 高级：映射和去重管道

`pipe!` 可以与 rxRust 运算符结合以进行更多控制。由于 `Pipe` 包装了底层流，您可以使用 `.transform()` 访问 rxRust 运算符的全部功能。

```rust no_run
use ribir::prelude::*;

fn advanced_pipe() -> Widget<'static> {
    fn_widget! {
        let count = Stateful::new(0);
        @Row {
            @Text {
                // 仅在值为偶数时更新文本
                text: pipe!(*$read(count))
                    .transform(|s| s.filter(|v| v % 2 == 0).box_it())
                    .map(|v| format!("Even number: {}", v))
            }
            @Button {
                on_tap: move |_| *$write(count) += 1,
                @{ "Increment" }
            }
        }
    }.into_widget()
}
```

常见运算符包括 `.map()`、`.filter()`、`.distinct_until_changed()` 等。当您需要超出简单映射的流结构或逻辑运算符时，请使用 `.transform()`。