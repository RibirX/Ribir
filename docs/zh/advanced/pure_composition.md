---
sidebar_position: 4
---

# 纯组合


在 Ribir 中：

- 视图由 widget 作为基本单位构建。
- widget 之间通过**纯组合**的方式组成新的 widget。

 Ribir 的独特之处在于它是通过**纯组合**的方式来组成新的 widget。

## 纯组合

当我们说**纯组合**时，意味着: widget 之间的父子关系并不涉及所有权，父 widget 通过 trait 来约定可以有子 widget，但并不拥有子 widget。

通常，其它框架的数据结构是这样的，父亲通过一个类似 `children` 的属性持有孩子：

```rust
struct Parent {
  property: &'static str,
  children: Vec<Child>,
}

struct Child {
  property: &'static str,
}

let widget = Parent {
  property: "parent",
  children: vec![
    Child { property: "child1" },
    Child { property: "child2" },
  ],
};
```

而在 Ribir 中，数据结构却是类似这样的：

```rust ignore
struct Parent {
  property: &'static str,
}

struct Child {
  property: &'static str,
}

let parent = Parent { property: "parent" };
let child1 = Child { property: "child1" };
let child2 = Child { property: "child2" };

let widget = MultiPair {
  parent,
  children: vec![child1, child2],
};
```

父 widget 和子 widget 之间是完全透明和独立的，我们并不将子 widget 添加到父 widget 中。

当然，这只是一个简化的例子。实际上，Ribir 的组合方式更加灵活，而且在实际使用中，你不会接触到 `MultiPair` 这样的中间数据结构。

这种组合方式的优点是，它产生的 widget 更小，更纯粹，更容易复用，可以根据需要进行组合。让我们以 Ribir 的内建 widget 为例来说明这一点。

在传统的 GUI 框架中，我们通常通过继承一个基础对象（或类似的方式）来获得一组常用的基础功能。这个基础对象通常包含许多属性，因此并不小。然而，在 Ribir 中，我们通过按需组合一组迷你的内建 widget 来实现这些功能。以 `Opacity` 为例，在 Ribir 中，它只有一个 `f32` 类型的属性。当你需要改变 widget 的透明度时，你可以直接使用 `Opacity` 来组合你的 widget：

```rust ignore
use ribir::prelude::*;

// Opacity 的定义是这样的：
// struct Opacity { opacity: f64 }

let w = Opacity { opacity: 0.5 }.with_child(Void, ctx);
```

当然，实际代码中，你可以直接写成 `@Void { opacity: 0.5 }`。


## 四种基础 widget

> 即将推出

- [ ] render widget
- [ ] compose widget
- [ ] compose child widget
- [ ] function widget 
