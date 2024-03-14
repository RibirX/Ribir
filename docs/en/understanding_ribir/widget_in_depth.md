---
sidebar_position: 1
---

# Widgets in Depth

In Ribir:

- Views are built as basic units of widgets.
- Widgets are composed **purely** from other widgets.

What makes Ribir unique is that it uses a **pure composition** method to compose new widgets.

## Pure Composition

In the context of **pure composition**, the parent-child relationship between widgets doesn't imply ownership. A parent widget can associate with child widgets via a trait agreement, without claiming ownership over them.

Typically, in other frameworks, the data structure allows a parent to contain its children via a property, often named `children`. Like this:

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

But in Ribir, the data structure is like this:

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

Parent and child widgets are entirely independent and transparent, with no child widgets being added directly to the parent widget.

Of course, this is just a simplified example. In fact, Ribir's composition method is more flexible, and in actual use, you will not deal with intermediate data structures like `MultiPair`.

The advantage of this composition method is that it produces smaller, purer, and more reusable widgets that can be composed as needed. Let's use Ribir's built-in widgets as an example to illustrate this point.

In traditional GUI frameworks, we often inherit from a base object (or use a similar approach) to gain a set of common features. This base object usually has many properties, making it quite large. Instead, Ribir gets these features by combining small built-in widgets as needed.  For example, the `Opacity` widget in Ribir has just one `f32` property. When you need to adjust a widget's opacity, you can simply compose it with the `Opacity` widget.

```rust ignore
use ribir::prelude::*;

// The definition of Opacity is like this:
// struct Opacity { opacity: f64 }

let w = Opacity { opacity: 0.5 }.with_child(Void, ctx);
```

Of course, in actual code, you can directly write `@Void { opacity: 0.5 }`.

## Four Basic Widgets

- [ ] render widget
- [ ] compose widget
- [ ] compose child widget
- [ ] function widget

Coming soon

