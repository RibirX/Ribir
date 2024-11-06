---
sidebar_position: 2
---

# Using Ribir without "DSL"

Whether you want clearer debugging or more Rust-like code, you might choose not to use Ribir's "DSL".

Don't worry. Ribir was designed to use "DSL" as a simple syntax layer. You can use Ribir's API to build UI directly. Even in a single piece of code, you can choose to use the API and macros together. Everything will be simple and natural.

## Core Concepts

In Ribir:

- Views are built as basic units of widgets.
- Widgets are [composed **purely**](./widget_in_depth.md#pure-composition) from other widgets.

So, building a UI with the API mainly involves two key points:

- How to create a widget
- How to compose child widgets

## Creating a widget with the API

Let's take the `FilledButton` widget as an example. Its definition is as follows:

```rust
use ribir::prelude::*;

struct FilledButton {
  color: Color
}
```

This is just like a regular Rust structure. You can directly create an object:

```rust
use ribir::prelude::*;

let button = FilledButton { color: Color::RED };
```

And there you have it, a red button.

### Extending Widget Functionality with `FatObj`

We've created a button, but it doesn't have any API for handling events.

In Ribir, event handling is implemented by separate widgets, not directly in the button itself. Any widget can gain event handling capabilities by compose with it.

For built-in widgets like event handlers, you don't need to compose them to use their features. Ribir offers a generic called `FatObj<T>`, which has initialization APIs for all built-in widgets. Simply wrap your widget with it, and your widget will have all the features of the built-in widgets.

```rust
use ribir::prelude::*;

let button = FilledButton { color: Color::RED };
let button = FatObj::new(button)
  .on_tap(|_| println!("Button tapped"));
```

But in practice, we usually don't write it directly like this. Instead, we create widgets using the `Declare` trait.

```rust
use ribir::prelude::*;

fn button_demo(ctx: &BuildCtx) {
  let btn: FatObj<State<FilledButton>> = FilledButton::declarer()
    .color(Color::RED)
    .on_tap(|_| println!("Button clicked"))
    .finish();
}
```

### Why should we use `Declare` to create widgets?

In the previous example, we used a method similar to the Builder pattern to create a widget. This might seem more complicated, but it actually has several benefits.

#### Interacting with `BuildCtx`

First, let's look at the complete definition of `FillButton`:

```rust
use ribir::core::prelude::*;

#[derive(Declare, Default)]
pub struct FilledButton {
  #[declare(default=Palette::of(BuildCtx::get()).primary())]
  pub color: Color,
}
```

Note the attribute `#[declare(default=Palette::of(BuildCtx::get()).primary())]`. This means that if you don't set a `color` value when creating `FilledButton` with `Declare`, it will use the primary color from the palette as the default.

This is the main reason we use `Declare` to create widgets: it allows widgets to access `BuildCtx` when they're created. This lets widgets automatically configure themselves based on the context, like changing dynamically with the theme.


#### Full Setup API

Another thing to note is that we're creating `FatObj<State<FilledButton>>`, not `FilledButton`. This is because `Declare` allows us to configure properties with methods of the same name and also use `FatObj` to extend the capabilities of built-in widgets. We use `State` because it lets your widget's state be observed and modified, which is a very common capability. For example, we might want the button's color to change to blue when clicked.

```rust
use ribir::prelude::*;

fn button_demo(_: &BuildCtx){
  let mut btn: FatObj<State<FilledButton>> = FilledButton::declarer()
    .color(Color::RED)
    .finish();

  let w = btn.clone_writer();
  btn = btn.on_tap(move |_| w.write().color = Color::BLUE);
}
```

Naturally, whether you're using `FatObj` or `State`, any associated overhead is only added to the final view you build when you utilize their provided capabilities.

#### Supports Initialization with `pipe!` Stream

Another benefit of using `Declare` to create widgets is that it allows for property initialization through the `pipe!` stream. Properties set up with the `pipe!` stream will adapt as the stream changes. For instance, let's say we want to create two `FilledButton`s, and we want `btn2`'s color to change in sync with `btn1`'s color.

```rust
use ribir::prelude::*;

fn button_demo(ctx: &BuildCtx){
  let btn1 = FilledButton::declarer().color(Color::RED).finish();

  let btn2 = FilledButton::declarer()
    .color(pipe!($btn1.color))
    .finish();

  btn1.write().color = Color::BLUE;
}
```

When we change the color of `btn1`, the color of `btn2` will also change accordingly.

#### How to Access Built-in Widget Properties

It's important to note that although widgets created with `Declare` can directly configure all built-in capabilities, if you need to modify the properties of a built-in widget after initialization, you need to first get the corresponding built-in widget and then operate on it. This is because these built-in widgets are composed as needed. In the example below, we create a button and change its margin when clicked.

```rust
use ribir::prelude::*;

fn button_demo(ctx: &BuildCtx){
  let mut btn = FilledButton::declarer()
    .color(Color::RED)
    .finish();

  let m = btn.get_margin_widget().clone_writer();
  btn = btn.on_tap(move |_| m.write().margin = EdgeInsets::all(10.0));
}

```

## Composing Child Widgets

In Ribir, we use the `with_child` method to compose a child widget with a parent widget to form a new widget. And the `@` syntax primarily uses `with_child` for its implementation. You might find yourself using this more often than you'd expect.

For example, for a `FilledButton`, the displayed text is also a child widget, not a property of it. This is because it can either be a text button or an icon button. If these were properties, memory would be allocated for the properties you don't need, whether you're using a text button or an icon button. But if it's a child widget, you can compose as needed.

Here's an example of a text button and an icon button:

```rust
use ribir::prelude::*;

fn button_demo(ctx: &BuildCtx) {
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

## Mixing API and Macros

Ribir's "DSL" is not a new language, but a set of macros. Each macro can be used as an independent expression, so you can freely mix them. Below, we'll implement an example of a counter. We'll create the buttons and the counter text directly through the API, and use `$` when initializing their properties to avoid cloning `cnt`. Finally, we'll use the `@` syntax to combine them into a `Row`:

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

## Conclusion

We hope that everyone using Ribir can choose their preferred way of using it, whether through the "DSL" or directly using the API, to get the best experience.

But what you need to understand is that Ribir's "DSL" is not a new language, we don't even call it "DSL". It's entirely built on the API we introduced above, just a set of macros, aimed at making the UI structure clearer, more readable, and avoiding some obvious repetitive code, such as frequent cloning of State due to move semantics.

In short, you can choose to use it partially, or choose not to use it at all, everything is free, you don't have to be afraid of seeing new syntax. Keep exploring and enjoy your [Ribir journey](../get_started/quick_start.md)!

