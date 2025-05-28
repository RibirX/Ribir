---
sidebar_position: 2
---

# 快速入门

本章节将为你介绍 Ribir 的全部语法和常用的基本概念。

> 你将了解
>
> - 如何创建和组合 widget
> - 如何响应事件并操作数据
> - 如何让视图自动响应数据变更
> - 如何构建动态 widget
> - 如何将自己的数据结构映射为视图
> - 如何将内建 widget 当做其它 widget 的一部分来使用
> - 如何对状态进行转换，分离和溯源——方便状态的传递和控制视图的更新范围

## 什么是 widget？

在 Ribir 中，widget 作为核心概念，它是对视图进行描述的基本单元。在形式上它可以是一个按钮，一个文本框，一个列表，一个对话框，甚至是整个应用界面。在代码上，它可以是一个函数，一个闭包或者一个数据对象。

如果你不是特别理解上面的话，不用在意，因为你完全不需要关注 widget 的构建过程，Ribir 也禁止干涉这个过程。你只需明白，Ribir 将所有的 widget 分成四类：

- 函数 widget
- `Compose` widget
- `Render` widget
- `ComposeChild` widget

本章将只会介绍函数 widget 和 `Compose` widget。因为在大部分场景中这两种 widget 已经足够满足我们的需求了。作为进阶的内容，我们将在[深入 widget](../understanding_ribir/widget_in_depth.md)中覆盖 `Render` widget 和 `ComposeChild` widget。

注意 `Widget` 和 widget 的差别，在整个 Ribir 的语境中，widget 是一个泛称，而大写开头的 `Widget` 是一个具体的 widget，也是所有 widget 构建进入视图的通行证。

## 函数 widget

返回 `Widget` 的函数或闭包被称为函数 widget ，一个可以被多次调用的函数 widget ，可以转换为一个 `GenWidget`, 而我们应用的根 widget 就要求是一个 `GenWidget`.

通过函数来定义 widget 是最简单快速的一种方式。在[体验 Ribir](./try_it.md)中，你已经见过一个 `Hello world!` 的函数 widget 了。本节中，我们仍通过 `Hello world!` 的例子来展开介绍。

### 通过函数来定义 widget 

我们先通过定义一个 `hello_world` 函数来完成我们的例子。

```rust no_run
use ribir::prelude::*;

fn hello_world() -> Widget<'static> {
  let mut text = Text::declarer();
  text.text("Hello World!");
  text.finish().into_widget()
}

fn main() { 
  App::run(hello_world);
}
```

因为 `Text` 这个 widget ，只提供了声明式 API 创建一种方法，所以我们需要以 `Text::declarer()` 来创建它的声明器，并以 `finish` 来完成创建。紧接着，我们通过 `into_widget` 来将它转化为 `Widget` 类型。

对于声明式 widget 我们还可以通过 `rdl!` 来简化它的写法。

```rust
use ribir::prelude::*;

fn hello_world() -> Widget<'static> {
  rdl!{ Text { text: "Hello World!" } }
    .into_widget()
}
```

关于 `rdl!` 的细节，我们将在小节 [使用 `rdl!` 创建对象](#使用-rdl-创建对象) 中详细介绍，现在先将它暂且放一放。


> 小提示
>
> 框架会给所有类型的 widget 自动实现 `into_widget` 的方法。

### 闭包和 `fn_widget!`

因为 `hello_world` 并没有被其它人调用，所以你可以将它改写成一个闭包:

```rust no_run
use ribir::prelude::*;

fn main() {
  let hello_world = || {
    rdl!{ Text { text: "Hello World!" } }
      .into_widget()
  };
  App::run(hello_world);
}
```

而对于通过闭包创建函数 widget ，Ribir 提供了一个 `fn_widget!` 宏来简化这个过程，`fn_widget!` 除了支持我们本章接下来要讲到的两个语法糖 `@` 和 `$` 之外，你可以简单认为它会这样展开代码：

``` rust ignore
move || -> Widget {
  {
    // 你的代码
  }
  .into_widget()
}
```

使用 `fn_widget!` 改写 `hello_world` 例子:


```rust no_run
use ribir::prelude::*;

fn main() {
  App::run(fn_widget! { 
    rdl!{ Text { text: "Hello World!" } }
  });
}
```

通常声明式 widget 都会提供一个同名宏，这个宏会通过 `fn_widget!` 来创建以自己为根节点的函数 widget 。

所以，我们的例子可以进一步简化为:

```rust no_run
use ribir::prelude::*;

fn main() {
  App::run(text! { text: "Hello World!"});
}
```

这便是我们在[体验 Ribir](./try_it.md)中看到的例子了。

## 使用 `rdl!` 创建对象

`rdl` 是  Ribir Declarative Language 的缩写， `rdl!` 宏的目的就是帮助你以声明式的方式来创建对象。

> 注意
>
> `rdl!` 并不关注类型，只在语法层面做处理，所以并不是只有 widget 才可以用它。

### 声明式创建对象

尽管 `rdl!` 支持任意 Rust 表达式，但我们所说的声明式创建对象，特指通过结构体字面量的方式：

```rust ignore
rdl! { 
  ObjectType {
    ... // 字段声明
  } 
}
```


当你的表达式是一个结构体字面量时， `rdl!` 会通过 `Declare` trait 来创建对象，这就要求你所创建的对象的类型必须继承或实现了 `Declare` trait。

```rust
use ribir::prelude::*;

#[derive(Declare)]
pub struct Counter {
  #[declare(default = 1usize)]
  count: usize,
}

fn use_rdl() {
  let _ = rdl!{ Counter { } };
}
```

上面的例子中，`Counter` 继承了 `Declare`， 并标记 `count` 默认值为 `1`。 所以在 `rdl!` 中，你可以不用给 `count` 赋值，`rdl!` 创建它时会默认赋值为 `1`。`Declare` 还有一些其它的特性，我们暂不在这里展开。

### 表达式创建对象

除了通过结构体字面量的方式创建对象以外，你还可以在 `rdl!` 中放置任意表达式，`rdl!`。这种方式的好处是，你可以在 `{...}` 中写任意代码在创建对象。这在嵌套组合中非常有用，也只在嵌套作为孩子时有必要。下面的例子展示如何在 `rdl` 中使用表达式创建对象：

```rust ignore
use ribir::prelude::*;

let _parent = rdl!{
  // 在这里你可以写任意的表达式，表达式的结果将作为孩子
  if  {
    ...
  } else {
    ...
  }
};
```


### 组合 widget

你已经知道如何在 `rdl!` 中创建 widget 了，我们现在通过将 widget 嵌套在另一个 widget 中来组合出一个简单的计数应用。

你可以在结构体字面量声明的 widget 中嵌入其它 `rdl!` 作为孩子，注意孩子总是被要求声明在父 widget 属性的后面，这是 `rdl!` 对格式的强制要求。

```rust no_run
use ribir::prelude::*;

fn main() {
  let counter = fn_widget! { 
    rdl!{ 
      Button {
        rdl!{ "0" }
      }
    }
  };

  App::run(counter);
}
```

上面的例子中，我们创建了一个 `Button`, 并给它组合了一个字符串作为孩子。`Button` 是在 ribir_widgets 库中已定义好的。

`rdl!` 也允许你为已创建好的 widget 声明孩子: 

```rust no_run
use ribir::prelude::*;

fn main() {
  let counter = fn_widget! {
    let btn = rdl! { Button {} };
    rdl!{ 
      $btn {
        rdl!{ "0" }
      }
    }
  };

  App::run(counter);
}
```

注意到 `rdl!{ $btn { ... } }` 了吗？ 它和结构体字面量语法一样，但是加上 `$` 后，它表示作为父亲的是一个变量而不是类型，所以它不会新建一个 widget ，而是直接使用这个变量来和孩子组合。

> 小提示
>
> 在 Ribir 中，父子的组合并不是任意的，而是有类型限制的，父亲可以约束孩子的类型并给出组合逻辑。这确保了组合的正确性。
>
> 在我们上面的例子中，`Button` 规定好了可以接受两个可选的孩子，一个字符串作为标签，一个 `Widget` 作为 icon。
>
> **为什么 Button 的标签要设计为是一个孩子而不是一个自己的字段呢？**
> 这是因为，如果设计为 `Button` 自己的字段，那么无论 `Button` 在使用时是否有一个标签，这个字段都要占用内存。而如果是作为一个孩子，则不存在这个字段内存的开销了。
>
> 关于如何约束 widget 的孩子类型，我们将在[深入 widget ](../understanding_ribir/widget_in_depth.md)中展开介绍。

## `@` 语法糖

在组合 widget 的过程中，我们用到了多个 `rdl!`。一方面，它让你在与 Rust 语法交互时（特别是复杂的例子）能有一个清晰的声明式结构——当你看到 `rdl!` 时，你就知道一个 widget 节点的组合或创建开始了；另一方面，当每一个节点都用 `rdl!` 包裹时，它又看上去太冗长了，无法让你一眼看到重点信息。

好在，Ribir 为 `rdl!` 提供了一个 `@` 语法糖，在实际使用的过程中，基本上用的都是 `@` 而非 `rdl!`。总共有三种情况：

- `@Button {...}` 作为结构体字面量的语法糖，展开为 `rdl!{ Button {...} }`
- `@ $btn {...}` 作为变量结构体字面量的语法糖，展开为 `rdl!{ $btn {...} }`
- `@ { ... } ` 是表达式的语法糖，展开为 `rdl!{ ... }` 

现在用 `@` 改写上面的计数器的例子:

```rust no_run
use ribir::prelude::*;

fn main() {
  App::run(fn_widget! {
    @Button {
      @ { "0" }
    }
  });
}
```

## 状态——让数据变得可被侦和共享

我们虽然创建了一个计数器，但它总是显示 `0`，也不响应按钮做任何事情。在这一节中，你将会了解到如何通过状态让你的计数器工作。

状态是一个将数据变得可被侦听和共享的包装器。

`状态 = 数据 + 可侦听 + 可共享`

一个可交互的 Ribir widget 的完整个生命周期是这样的：

1. 将你的数据转换为状态。
2. 对状态进行声明式映射构建出视图。
3. 在交互过程中，通过状态来修改数据。
4. 通过状态接收到数据的变更，根据映射关系点对点更新视图
5. 重复步骤 3 和 4 。

![状态的生命周期](../assets/data-flows.svg)

现在，让我们引入状态来改造我们的例子。

```rust no_run
use ribir::prelude::*;

fn main() {
  App::run(fn_widget! {
    // 变更 1: 通过 `State::value` 创建一个状态
    let count = State::value(0);
    @Button {
      // 变更 2： 通过点击事件来修改状态
      on_tap: move |_| *$count.write() += 1,
      // 变更 3： 通过状态来显示数据，并保持视图的持续更新。
      // 对于宏或者函数调用，我们可以省略 @ 后面的一对大括号
      @ pipe!($count.to_string())
    }
  });
}
```

通过这 3 处变更，计数器的小例子全部完成了。但是在变更 2 和变更 3 中，有新的东西被引入了 —— `$` 和 `pipe!`。它们是 Ribir 中非常重要的用法，让我们用两个小节来分别展开介绍。

## $ 语法糖

在 Ribir 中有两个重要的语法糖，一个是我们之前介绍的 [@ 语法糖](#语法糖)，另一个就是 `$` 语法糖了。

### 状态的读写引用

`$` 表示对跟随其后的状态做读或写引用。比如 `$count` 表示对 `count` 状态的读引用，而当其后多跟随一个`write()` 调用时，则表示对 `count` 状态的写引用，如 `$count.write()`。

除了 `write` 以外， Ribir 还有一个 `silent` 写引用，通过 `silent` 写引用修改数据不会触发视图更新。

状态的 `$` 语法糖展开逻辑为：

- `$counter.write()` 展开为 `counter.write()`
- `$counter.silent()` 展开为 `counter.silent()`
- `$counter` 展开为 `counter.read()`

### 状态的自动共享

当 `$` 处在一个 `move` 闭包中时，它指向的状态会被克隆（读/写），闭包捕获的是状态的克隆，因此 `$` 让你可以直接使用一个状态，并轻易的完成共享，而不用去额外的克隆它。


```rust ignore
move |_| *$count.write() += 1
```

大致展开成

```rust ignore
{
  let count = count.clone_writer();
  move |_| *count.write() += 1
}
```

### 语法糖展开的优先级

还记得我们在[组合-widget](#组合-widget)中也同样用到了 `$` 吗？
比如 `rdl!{ $btn { ... } }` 或者 `@ $btn { ... }`，这可不是对状态数据的引用哦。因为 `rdl!` 赋予了它其它的语义——通过变量声明父 widget。

无论是 `@` 还是 `$`，它们首先应遵循它们所在宏的语义，其次才是一个 Ribir 语法糖。当我们在一个非 Ribir 提供的宏中使用 `@` 或 `$` 时，它们就不再是 Ribir 的语法糖，因为外部宏很可能为它们赋予了特殊的语义。比如：

```rust ignore
use ribir::prelude::*;

fn_widget!{
  user_macro! {
    // `@` 此时不是语法糖，它的语义取决于 `user_macro!` 的实现
    @Button { ... }
  }
}
```

## `Pipe` 流 —— 保持对数据的持续响应

`Pipe` 流是一个带初始值的持续更新的数据流，它可以被分解为一个初始值和 rxRust 流 —— rxRust 流可以被订阅。它也是 Ribir 将数据变更更新到视图的唯一通道。

Ribir 提供了一个 `pipe!` 宏来辅助你快速创建 `Pipe` 流。它接收一个表达式，并监测表达式中的所有用 `$` 标记出的状态，以此来触发表达式的重算。

在下面的例子中, `sum` 是一个 `a`， `b` 之和的 `Pipe` 流，每当 `a` 或 `b` 变更时，`sum` 都能向它的下游发送最新结果。

```rust 
use ribir::prelude::*;

let a = State::value(0);
let b = State::value(0);

let sum = pipe!(*$a + *$b);
```

在声明一个对象时，你可以通过一个 `Pipe` 流去初始化它的属性，这样它的属性就会持续随着这个 `Pipe` 流变更。如我们在[状态——让数据变得可被侦和共享](#状态——让数据变得可被侦和共享)中见过的：

```rust ignore
  @Text { text: pipe!($count.to_string()) }
```

### 动态渲染 widget

到目前为止，所有你创建的视图的结构都是静态的，它们仅仅只有属性会随着数据变更，但 widget 的结构不会随着数据变更。实际上，你同样可以通过 `Pipe` 流来创建持续变化的 widget 结构。

假设你有一个计数器，这个计数器不是用文字来显示数目，而是通过红色小方块来计数：


![Box counter](../assets/box_counter.gif)

代码：

```rust no_run
use ribir::prelude::*;

fn main() {
  let counter = State::value(0);
  App::run(row! {
    @Button {
      on_tap: move |_| *$counter.write() += 1,
      @ { "Increment" }
    }
    @ {
      pipe!(*$counter).map(move |counter| {
        (0..counter).map(move |_| {
          @Container {
            margin: EdgeInsets::all(2.),
            size: Size::new(10., 10.),
            background: Color::RED
          }
        })
      })
    }
  });
}
```

### 尽量让 `pipe!` 包含最小的表达式

虽然 `pipe!` 可以包含任意的表达式，但是建议你尽可能在 `pipe!` 中只包含最小表达式，然后使用 `map` 来完成转换。这样可以让你更清晰的看到 `pipe!` 的变更源和避免在复杂的表达式中混入了不必要的依赖。所以，上面例子中写的是

```rust ignore
pipe!(*$counter).map(move |counter| {
  move || {
    (0..counter).map(move |_| {
      @Container {
        margin: EdgeInsets::all(2.),
        size: Size::new(10., 10.),
        background: Color::RED
      }
    })
  }
})
```

而不是

```rust ignore
pipe!{
  move || {
    (0..*$counter).map(move |_| {
      @Container {
        margin: EdgeInsets::all(2.),
        size: Size::new(10., 10.),
        background: Color::RED
      }
    })
  }
}
```

### 为 `Pipe` 链上 rxRust 的操作符

`Pipe` 流的更新推送是建立在 rxRust 流之上的，所以 `Pipe` 也提供了方法 `value_chain` 让你可以操作 rxRust 流。因此，你可以使用 rxRust 的操作符来如 `filter`, `debounce` `distinct_until_change` 等等操作来减少更新的频率。

假设你有一个简单的自动求和例子：

```rust no_run
use ribir::prelude::*;

fn main() {
  App::run(fn_widget! {
    let a = State::value(0);
    let b = State::value(0);

    @Column {
      @Text { text: pipe!($a.to_string()) }
      @Text { text: pipe!($b.to_string()) }
      @Text {
        text: pipe!((*$a + *$b).to_string())
          .value_chain(|s| s.distinct_until_changed().box_it()),
        on_tap: move |_| {
          *$a.write() += 1;
          *$b.write() -= 1;
        }
      }
    }
  });
}
```

在上面的例子中， 前面两个 `Text` 会随着 `a` 和 `b` 的修改而更新，即使 `a` 和 `b` 的值没有发生变化——比如对其设同样的值。而最后一个 `Text` 通过 `distinct_until_changed` 来过滤掉重复值的更新，只有当 `a` , `b` 之和的结果发生变化时，它才会更新。

因此，当我们点击最后一个 `Text` 时，只有前面两个 `Text` 会被标记发生更新，而最后一个 `Text` 不会。


> 小贴士
>
> 一般来讲，想知道视图哪部分是动态变化的，你只需要查找哪里有 `pipe!`。


## 用 `watch!` 侦听表达式的变更

`watch!` 是一个用来侦听表达式变更的宏，它接收一个表达式，并监测表达式中的所有用 `$` 标记出的状态，以此来触发表达式的重算，并向下游的订阅者推送最新的结果。


`watch!` 与 `pipe!` 一样侦听着表达式的变更，并有相同的语法，但 `pipe!` 是带初始值的，它表现的更像一个持续变更的值，而不仅仅是一个可订阅的数据流，而 `watch!` 则仅是一个可订阅的数据流，因此 `pipe!` 可以被当做一个值来初始化 widget 的属性，而 `watch!` 则不能。

简而言之：

- `pipe!` = (初始值 + rxRust 流)
- `watch!` = rxRust 流

你也可以用 `watch!` 来手动实现你的计数器：
  
```rust no_run
use ribir::prelude::*;

fn main() {
  App::run(fn_widget! {
    let count = State::value(0);
    let display = @H1 { text: "0" };

    watch!(*$count).subscribe(move |v| {
      $display.write().text = v.to_string().into();
    });

    @Row {
      @Button {
        on_tap: move |_| *$count.write() += 1,
        @ { "Increment" }
      }
      @{ display }
    }
  });
}
```

一旦我们调用了 `subscribe`，就建立了对 `watch!` 中表达式的订阅。这个订阅会一直存在，直到你手动调用 `unsubscribe`，或者 `watch!` 所监听的 State 不再有写入源。

在上述例子中，我们无需调用 `unsubscribe`，因为这个订阅需要在整个应用的生命周期中存在。

通常，有两种情况你需要手动调用 `unsubscribe`：

第一种情况，你希望订阅的生命周期短于所监听的状态。这种情况的典型例子就是使用外部状态来构建 widget，例如：

```rust
use ribir::prelude::*;

fn show_name(name: State<String>) -> Widget<'static> {
  fn_widget!{
    let mut text = @Text { text: "Hi, Guest!" };
    let u = watch!($name.to_string()).subscribe(move |name| {
      $text.write().text = format!("Hi, {}!", name).into();
    });

    // `name` 是一个可共享的状态，它可能被其他人持有，使它的生命周期长于 widget 
    // 所以我们需要在 widget 销毁时取消订阅
    @$text { on_disposed: move |_| u.unsubscribe() }
  }
  .into_widget()
}
```

第二种情况，`watch!` 的下游对所监听的状态进行了写操作。因为 `watch!` 依赖于所监听的状态不再拥有写入源来自动取消订阅，当其下游持有了所监听的状态的写入源时，这就构成了循环引用。这时，必须手动取消订阅，否则会导致内存泄漏。例如：

```rust
use ribir::prelude::*;

let even_num = State::value(0);

// 响应 even_num 的变更，确保其为偶数，当 even_num 为奇数时，将其加 1 使其变为偶数
let u = watch!(*$even_num).subscribe(move |v| {
  if v % 2 == 1 {
    *even_num.write() = v + 1;
  }
});

// 需要在适当的时机调用下面的代码，否则会导致循环引用
u.unsubscribe();
```

## `Compose` widget —— 描述你的数据结构

通常，在复杂的真实场景中，你无法只通过创建一些局部的数据和使用简单的函数 widget 就完成全部开发。你需要自己的数据结构，并通过 `Compose` widget 来完成你的数据结构到视图的映射。

将计数器的例子改写成使用 `Compose` widget 的形式：

```rust no_run
use  ribir::prelude::*;

struct Counter(usize);

impl Counter {
  fn increment(&mut self) {
    self.0 += 1;
  }
}

impl Compose for Counter {
  fn compose(this: impl StateWriter<Value = Self>) -> Widget<'static> {
    button! {
      on_tap: move |_| $this.write().increment(),
      @pipe!($this.0.to_string())
    }
    .into_widget()
  }
}

fn main() { 
  App::run(fn_widget!(Counter(0))); 
}

```

上面的例子中，当你为 `Counter` 实现了 `Compose` 后，`Counter` 以及所有 `Counter` 的可写状态都是一个合法的 widget 了。


## 内建 widget

Ribir 提供了一组内建 widget，让你可以配置基础的样式、响应事件和生命周期等等。内建 widget 和普通的 widget 的重要差别在于——在声明式创建 widget 时，你可以直接将内建 widget 的字段和方法当做是你所创建 widget 自己的一样来使用，Ribir 会帮你完成内建 widget 的创建和组合。

拿 `Margin` 举例，假设你要为一个 `Text` 设置 10 像素的空白边距，代码如下：

```rust no_run
use ribir::prelude::*;

fn main() {
  App::run(fn_widget! {
    // 声明 `Margin` 作为 `Text` 的父亲
    @Margin {
      margin: EdgeInsets::all(10.),
      @Text { text: "Hello World!" }
    }
  });
}
```

你其实不必显示声明一个 `Margin`, 你可以直接写成：

```rust no_run
use ribir::prelude::*;

fn main() {
  App::run(fn_widget! {
    // 在 `Text` 中直接声明 `Margin::margin` 字段
    @Text {
      margin: EdgeInsets::all(10.),
      text: "Hello World!"
    }
  });
}
```

当你通过声明式创建了一个 widget 后，你可以直接访问内建 widget 的字段，即使你并没有显示声明它们（如果你在代码中用到它们，相应的内建 widget 会被创建）。比如：

```rust no_run
use ribir::prelude::*;

fn main() {
  App::run(fn_widget! {
    // 没有声明 `margin`
    let mut hello_world = @Text { text: "Hello World!" };
    // 直接修改 `margin`
    $hello_world.write().margin = EdgeInsets::all(10.);
    hello_world
  });
}
```

这是通过泛型类型 `FatObj` 扩展的，参考 [`FatObj`](https://docs.rs/ribir_core/@RIBIR_VERSION/ribir_core/builtin_widgets/struct.FatObj.html) 的 API 文档，查看它提供的所有扩展能力。


## 状态的转换、分离和溯源

经过前面章节的学习，你已经了解到：

- 状态对数据的修改会导致依赖视图被直接更新
- 通过 `Compose` 可以完成数据到视图的映射

假设 `AppData` 是你整个应用的数据，你可以通过 `Compose` 来完成它到视图的映射。但是，如果 `AppData` 是一个复杂的数据，只用一个 `Compose` 来映射整个应用的视图在代码组织上会是一个灾难；而整个应用视图只依赖一个状态，则会导致任意对 `AppData` 的修改，都会更新视图的全部动态部分，大部分情况下，这导致你的应用无法得到最佳的交互性能。

还好，对于状态，Ribir 提供了一套转换、分离和溯源的机制。它让你可以从一个完整的应用状态开始，然后将应用状态转换或分离成更小的子状态，子状态又可以继续转换或分离...； 而在子状态中，你则可以通过溯源机制来获取自己的转换或分离的来源。

### 转换和分离，将状态转换为子状态

**转换**是将一个父状态转换为子状态，父子状态共享同样的数据，修改父状态等同于修改子状态，反之亦然。它仅仅是缩减了数据的可见范围，方便你只想使用和传递传递部分状态。

**分离**是从一个父状态中分离出子状态，父子状态共享同样的数据。不同的是，通过子状态修改数据不会触发父状态的依赖视图更新。

你要注意的是，不论是转换还是分离，父子状态共享的都是同一份数据。因此，它们对数据的修改会影响到彼此，但它们所推送的数据变更的范围可能不同。

仔细阅读下面的例子，会帮助你更好的理解状态的转换和分离是如何工作的：

```rust
use ribir::prelude::*;

struct AppData {
  count: usize,
}

let state = State::value(AppData { count: 0 });
let map_count = state.part_writer(None, |d| PartMut::new(&mut d.count));
let split_count = state.part_writer(None, |d| PartMut::new(&mut d.count));

watch!($state.count).subscribe(|_| println!("父状态数据"));
watch!(*$map_count).subscribe(|_| println!("子状态（转换）数据"));
watch!(*$split_count).subscribe(|_| println!("子状态（分离）数据"));
state
  .raw_modifies()
  .filter(|s| s.contains(ModifyScope::FRAMEWORK))
  .subscribe(|_| println!("父状态 框架"));
map_count
  .raw_modifies()
  .filter(|s| s.contains(ModifyScope::FRAMEWORK))
  .subscribe(|_| println!("子状态（转换）框架"));
split_count
  .raw_modifies()
  .filter(|s| s.contains(ModifyScope::FRAMEWORK))
  .subscribe(|_| println!("子状态（分离）框架"));

// 通过分离子状态修改数据，父子状态的订阅者都会被推送数据通知，
// 只有分离子状态的订阅者被推送框架通知
*split_count.write() = 1;
// 推送是是异步的，强制推送立即发出
AppCtx::run_until_stalled();
// 打印内容：
// 父状态数据
// 子状态（转换）数据
// 子状态（分离）数据
// 子状态（分离）框架

// 通过父状态修改数据, 分离状态会失效，父子状态的依赖都会被推送
state.write().count = 3;
AppCtx::run_until_stalled();
// 打印内容：
// 父状态数据
// 子状态（转换）数据
// 父状态 框架
// 子状态（转换）框架

// 通过转换子状态修改数据，父子状态的依赖都会被推送
*map_count.write() = 2;
AppCtx::run_until_stalled();
// 打印内容：
// 父状态数据
// 子状态（转换）数据
// 父状态 框架
// 子状态（转换）框架
```

因为 Ribir 的数据修改通知是异步批量发出的，所以在例子中为了方便理解，我们每次数据修改都调用了 `AppCtx::run_until_stalled()` 来强制理解发送，但这不应该出现在你真实的代码中。


如果你仅是想获得一个只读的子状态，那么可以通过 `part_reader` 来转换：

```rust ignore
let count_reader = state.part_reader(|d| &d.count);
```

但 Ribir 并没有提供一个 `split_reader`，因为分离一个只读的子状态，其意义等同于转换一个只读子状态。


## 下一步

至此，你已经掌握了开发 Ribir 引用所需的全部语法和基础概念了。是时候到[实践： Todos 应用](../practice_todos_app/develop_a_todos_app.md)将它们付诸实践了。