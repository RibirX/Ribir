---
sidebar_position: 2
---

# 不依赖 "DSL" 使用 Ribir

无论是为了更直观的调试，还是为了让代码更具 Rust 风格，你可能会选择不使用 [Ribir](https://github.com/RibirX/Ribir) 的 "DSL"。

这并无问题，得益于 Ribir 在设计初期就将 "DSL" 定位为一个轻量级的语法转换层，你完全可以直接使用 Ribir 的 API 来构建 UI。甚至在一个代码片段中，你可以选择部分使用 API，部分使用宏，两者交织在一起使用。一切都将简单而自然。

## 核心概念

在 Ribir 中：

- 视图是由 widget 作为基本单位构建的。
- widget 之间通过[**纯组合**](./widget_in_depth.md#纯组合)方式组成新的 widget。

因此，通过 API 构建 UI 主要涉及两个关键点：

- 如何创建 widget
- 如何组合子 widget

## 通过 API 创建 widget

以 `FilledButton` 控件为例，其定义如下：

```rust
use ribir::prelude::*;

struct FilledButton {
  color: Color
}
```

这与常规的 Rust 结构体无异，你可以直接创建一个对象：

```rust
use ribir::prelude::*;

let button = FilledButton { color: Color::RED };
```

这样，你就得到了一个红色的按钮。

### 通过 `FatObj` 扩展控件的能力

我们已经创建了一个按钮，但是它并没有提供任何响应事件的 API。

这是因为在 Ribir 中，事件响应是由独立的控件负责实现，而非直接在按钮本身实现。任何控件都可以通过与它组合来获取事件响应的能力。

并且，对于内建控件如事件响应，无需通过组合方式即可获取。Ribir 提供了一个名为 `FatObj<T>` 的泛型，它提供了所有内建控件的初始化 API。只需用它包裹我们的控件，即可让控件获得所有内建控件的能力。

```rust
use ribir::prelude::*;

let button = FilledButton { color: Color::RED };
let button = FatObj::new(button)
  .on_tap(|_| println!("Button tapped"));
```

但在实际使用中，我们通常不直接这样写，而是通过 `Declare` 这个 trait 来创建 widget。

```rust
use ribir::prelude::*;

fn button_demo(ctx: &BuildCtx) {
  let btn: FatObj<State<FilledButton>> = FilledButton::declarer()
    .color(Color::RED)
    .on_tap(|_| println!("Button clicked"))
    .finish();
}
```

### 为何我们应使用 `Declare` 创建 widget？

在上述示例中，我们通过类似 Builder 模式来创建 widget，这使得过程看起来更复杂。然而，这种方式实际上带来了更多的优势。

#### 与 `BuildCtx` 交互

首先，我们来看一下 `FillButton` 的完整定义：

```rust
use ribir::prelude::*;

#[derive(Declare, Default)]
pub struct FilledButton {
  #[declare(default=Palette::of(BuildCtx::get()).primary())]
  pub color: Color,
}
```

需要注意的是，它有一个 attribute：`#[declare(default=Palette::of(BuildCtx::get()).primary())]`。这意味着，如果在使用 `Declare` 创建 `FilledButton` 时，没有设置 `color` 值，那么将使用调色板的主色作为默认值。

这就是我们为何要通过 `Declare` 创建控件的首要原因：它允许控件在创建时访问 `BuildCtx`，使得控件能够根据上下文自动配置，例如，随着主题的变化而动态变化。

#### 完整的初始化 API

另一个要注意的是，我们最终创建的是 `FatObj<State<FilledButton>>`，而不是 `FilledButton`。这是因为通过 `Declare`，我们不仅可以使用同名方法配置属性，还可以利用 `FatObj` 扩展内建 widget 的能力。至于为什么要使用 `State`，这是因为 `State` 可以让你的控件状态被监听和修改，这是一个非常常用的能力。例如，我们希望点击按钮后，按钮的颜色变为蓝色：

```rust
use ribir::prelude::*;

fn button_demo(ctx: &BuildCtx){
  let mut btn: FatObj<State<FilledButton>> = FilledButton::declarer()
    .color(Color::RED)
    .finish();

  let w = btn.clone_writer();
  btn = btn.on_tap(move |_| w.write().color = Color::BLUE);
}
```

当然，无论是 `FatObj` 还是 `State`，只有在你用到它们提供的能力时，相关的开销才会被添加到你最终构建的视图中。

#### 支持使用 `pipe!` 流进行初始化

使用 `Declare` 创建 widget 的另一个优点是，它支持通过 `pipe!` 流来初始化属性。通过 `pipe!` 流初始化的属性会随着流的变化而变化。例如，我们想要创建两个 `FilledButton`，其中 `btn2` 的颜色会随着 `btn1` 的颜色而变：

```rust
use ribir::prelude::*;

fn button_demo(){
  let btn1 = FilledButton::declarer().color(Color::RED).finish();

  let btn2 = FilledButton::declarer()
    .color(pipe!($btn1.color))
    .finish();

  btn1.write().color = Color::BLUE;
}
```

当我们改变 `btn1` 的颜色时，`btn2` 的颜色也会相应地变化。

#### 如何访问内建 widget 属性

需要注意的是，虽然通过 `Declare` 创建的 widget 可以直接配置所有内建能力，但如果你需要在初始化后修改内建  widget 的属性，你需要先获取对应的内建控件再进行操作。这是因为这些内建控件是按需组合得到的。下面的例子中，我们创建一个按钮，并在点击时更改其边距：

```rust
use ribir::prelude::*;

fn button_demo(){
  let mut btn = FilledButton::declarer()
    .color(Color::RED)
    .finish();

  let m = btn.get_margin_widget().clone_writer();
  btn = btn.on_tap(move |_| m.write().margin = EdgeInsets::all(10.0));
}

```

## 子控件的组合

在 Ribir 中，我们使用 `with_child` 方法将子 widget 和父 widget 组合成新的 widget。`@` 语法主要也是利用 `with_child` 来实现的。实际上，你可能会比想象中更频繁地使用它。

例如，对于一个 `FilledButton`，它显示的文本甚至都是一个子 widget，而不是它的属性。这是因为它既可以是一个文本按钮，也可以是一个图标按钮。如果这些都是属性，那么无论你是使用文本按钮还是图标按钮，都会为你不需要的属性分配内存。但如果作为一个子控件，就可以根据使用情况来组合。

这是一个文本按钮和图标按钮的例子：

```rust
use ribir::prelude::*;

fn button_demo(){
  let text_btn = FilledButton::declarer()
    .color(Color::RED)
    .finish()
    .with_child(Label::new("Text Button"));

  let icon_btn = FilledButton::declarer()
    .color(Color::RED)
    .finish()
    .with_child(svgs::ADD);
}
```

## API 和宏的混合使用

Ribir 的 "DSL" 并不是一种全新的语言，而只是一组宏。每个宏都可以作为一个独立的表达式使用，因此你可以自由地混合使用它们。下面我们将实现一个计数器的例子。我们将直接通过 API 创建按钮和计数的文本，并在初始化它们的属性时使用 `$` 来避免克隆 `cnt`。最后，我们将使用 `@` 语法将它们组合成一个 `Row`：

```rust
use ribir::prelude::*;

let counter = fn_widget! {
  let cnt = Stateful::new(0);
  let btn = FilledButton::declarer()
    .on_tap(move |_| *$cnt.write() += 1)
    .finish()
    .with_child(Label::new("Inc"));

  let label = H1::declarer()
    .text(pipe!($cnt.to_string()))
    .finish();

  @Row {
    @ { btn }
    @ { label }
  }
};
```
## 结语

我们希望每个使用 Ribir 的人都能根据自己的喜好选择使用方式，无论是通过 "DSL" 还是直接使用 API，都能获得最佳的体验。

但你需要明白的是，Ribir 的 "DSL" 并不是一种新的语言，我们甚至不把它称为 "DSL"。它完全是基于我们在上文中介绍的 API 构建的，只是一组宏，目的是让 UI 结构更清晰，更易读，并避免一些明显的重复代码，比如因为 move 语义而需要频繁克隆 State。

总之，你可以选择部分使用它，也可以选择完全不使用它，一切都是自由的，不必因为看到新的语法而感到恐惧。继续你的 [Ribir 之旅](../get_started/quick_start.md)吧！

