---
sidebar_position: 2
---

# 不依赖 "DSL" 使用 Ribir


或许是为了更直观的调试，或许是为了让代码更具 Rust 风格，有些人会更倾向于避免使用过多的宏和引入新的语法，因此也就不愿意使用 [Ribir](https://github.com/RibirX/Ribir) 的 "DSL"。

这并无问题，得益于 Ribir 在设计初期就将 "DSL" 定位为一个轻量级的语法转换层，你完全可以直接使用 Ribir 的 API 来构建 UI。甚至在一个代码片段中，你可以选择部分使用 API，部分使用宏，两者交织在一起使用。一切都将简单而自然。

## 核心概念

在 Ribir 中：

- 视图是由 widget 作为基本单位构建的。
- widget 之间通过[**纯组合**](./widget_in_depth.md#纯组合)方式组成新的 widget。

因此，通过 API 构建 UI 主要涉及两个关键点：

- 如何创建 widget
- 如何组合子 widget

## 通过 API 创建 widget

以 `Radio`  widget 为例，其定义如下：

```rust
use ribir::prelude::*;

pub struct Radio {
  pub checked: bool,
}
```

这与常规的 Rust 结构体无异，你可以直接创建一个对象：

```rust
use ribir::prelude::*;

let radio = Radio { checked: true };
```

这样，我们就得到了一个选中的 `Radio`.

### 通过 `FatObj` 扩展 widget 的能力

我们已经创建了一个 `Radio`，但是它并没有提供任何响应事件的 API。

这是因为在 Ribir 中，事件响应是由独立的 widget 负责实现。任何 widget 都可以通过与它组合来获取事件响应的能力。

并且，对于内建 widget 如事件响应，我们可以无需通过组合方式即可获取。Ribir 提供了一个 `FatObj<T>` 的泛型，它提供了所有内建 widget 的初始化 API。只需用它包裹我们的 widget，即可让 widget 获得所有内建 widget 的能力。

```rust
use ribir::prelude::*;

let radio = Radio { checked: true };
let radio = FatObj::new(radio)
  .on_tap(|_| println!("Radio tapped"));
```

但在实际使用中，我们通常不直接这样写，而是通过 `Declare` 这个 trait 来创建 widget。

```rust
use ribir::prelude::*;

let btn: FatObj<State<Radio>> = Radio::declarer()
  .checked(true)
  .on_tap(|_| println!("Radio clicked"))
  .finish();
```

### 为何我们应使用 `Declare` 创建 widget？

在上述示例中，我们通过类似 Builder 模式来创建 widget，这使得过程看起来更复杂。然而，这种方式实际上带来了更多的优势。


#### 完整的初始化 API

要注意的是，我们最终创建的是 `FatObj<State<Radio>>`，而不是 `Radio`。这是因为通过 `Declare`，我们不仅可以使用同名方法配置属性，还可以利用 `FatObj` 扩展内建 widget 的能力。至于为什么要使用 `State`，这是因为 `State` 可以让你的 widget 状态被监听和修改。

```rust
use ribir::prelude::*;

let mut radio: FatObj<State<Radio>> = Radio::declarer()
  // 我们可以使用内建能力
  .on_tap(|_| println!("taped!"))
  .finish();

watch!($radio.checked)
  .subscribe(|checked| println!("The radio state change to {checked}"));
```

当然，无论是 `FatObj` 还是 `State`，只有在你用到它们提供的能力时，才会影响到最终构建的视图的开销。

#### 支持使用 `pipe!` 流进行初始化

使用 `Declare` 创建 widget 的另一个优点是，它支持通过 `pipe!` 流来初始化属性。通过 `pipe!` 流初始化的属性会随着流的变化而变化。例如，我们想要创建两个 `Radio`，其中一个的状态会跟随着另一个的状态而变化。

```rust
use ribir::prelude::*;

let mut radio1: FatObj<State<Radio>> = Radio::declarer()
  .checked(true)
  .finish();
let radio2 = Radio::declarer()
  .checked(pipe!($radio1.checked))
  .finish();

let _row = Row::declarer()
  .finish()
  .with_child(radio1)
  .with_child(radio2)
  .into_widget();
```

#### 支持访问内建 widget 属性

需要注意的是，虽然通过 `Declare` 创建的 widget 可以直接配置所有内建能力，但如果你需要在初始化后修改内建 widget 的属性，你需要先获取对应的内建 widget 再进行操作。这是因为这些内建 widget 是按需组合得到的。下面的例子中，我们创建一个按钮，并在点击时更改其边距：

```rust
use ribir::prelude::*;

fn radio_btn() -> Widget<'static> {
  let mut btn = Radio::declarer().finish();
  
  let m = btn.get_margin_widget().clone_writer();
  btn
    .on_tap(move |_| m.write().margin = EdgeInsets::all(10.0))
    .into_widget()
}
```

## 子 widget 的组合

在 Ribir 中，我们使用 `with_child` 方法将子 widget 和父 widget 组合成新的 widget。`@` 语法主要也是利用 `with_child` 来实现的。实际上，你可能会比想象中更频繁地使用它。

例如，对于一个 `Button`，它显示的文本甚至都是一个子 widget，而不是它的属性。这是因为它既可以是一个文本按钮，也可以是一个图标按钮。如果这些都是属性，那么无论你是使用文本按钮还是图标按钮，都会为你不需要的属性分配内存。但如果作为一个子 widget ，就可以根据使用情况来组合。

这是一个文本按钮和图标按钮的例子：

```rust
use ribir::prelude::*;

let text_btn = Button::declarer()
  .finish()
  .with_child("Text Button");

let icon_btn = Button::declarer()
  .finish()
  .with_child(Icon.with_child(named_svgs::get_or_default("search")));
```

## API 和宏的混合使用

Ribir 的 "DSL" 并不是一种全新的语言，而只是一组宏。每个宏都可以作为一个独立的表达式使用，因此你可以自由地混合使用它们。下面我们将实现一个计数器的例子。我们将直接通过 API 创建按钮和计数的文本，并在初始化它们的属性时使用 `$` 来避免克隆 `cnt`。最后，我们将使用 `@` 语法将它们组合成一个 `Row`：

```rust
use ribir::prelude::*;

let counter = fn_widget! {
  let cnt = Stateful::new(0);
  let btn = Button::declarer()
    .on_tap(move |_| *$cnt.write() += 1)
    .finish()
    .with_child("Inc");

  let label = H1::declarer()
    .text(pipe!($cnt.to_string()))
    .finish();

  @Row {
    align_items: Align::Center,
    @ { btn }
    @ { label }
  }
};
```

## 结语

我们希望每个使用 Ribir 的人都能根据自己的喜好选择使用方式，无论是通过 "DSL" 还是直接使用 API，都能获得最佳的体验。

但你需要明白的是，Ribir 的 "DSL" 并不是一种新的语言，我们甚至不把它称为 "DSL"。它完全是基于我们在上文中介绍的 API 构建的，只是一组宏，目的是让 UI 结构更清晰，更易读，并避免一些明显的重复代码，比如因为 move 语义而需要频繁克隆 State。

总之，你可以选择部分使用它，也可以选择完全不使用它，一切都是自由的，不必因为看到新的语法而感到恐惧。继续你的 [Ribir 之旅](../get_started/quick_start.md)吧！

