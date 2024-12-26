---
sidebar_position: 2
---

# Using Ribir without relying on "DSL"


Perhaps for the sake of more intuitive debugging, or perhaps to give code a more Rust-like feel, some people will prefer to avoid using too many macros and introducing new syntax, and therefore will be reluctant to use [Ribir](https://github.com/RibirX/Ribir)'s "DSL".

This is fine, because Ribir was designed from the beginning to position the "DSL" as a lightweight syntax transformation layer, and you can use Ribir's APIs directly to build the UI. You can even choose to use the APIs partly, the macros partly, and the two intertwined in a single code snippet. Everything will be simple and natural.

## Core Concepts

In Ribir:

- Views are built with widgets as their base unit.
- Widgets are linked to each other by [**pure composition**](. /widget_in_depth.md#composition) to form new widgets.

Therefore, there are two key points involved in building the UI through the API:

- How to create widgets
- How to compose child widgets

## Creating widgets via API

As an example, the `Radio` control is defined as follows:

```rust
use ribir::prelude::*;

pub struct Radio {
  pub selected: bool,
  pub value: Box<dyn Any>
}
```

This is no different than a regular Rust struct, you can just create an object:

```rust
use ribir::prelude::*;

let radio = Radio { selected: true, value: Box::new(1.) };
```

This gives us a selected `Radio`.

### Extending the capabilities of a widget with `FatObj`

We've created a `Radio`, but it doesn't provide any API for responding to events.

This is because in Ribir, event response is implemented by a separate widget. Any widget can be composed with it to gain the ability to respond to events.

And, for built-in widgets such as event responses, we can get them without composing them; Ribir provides a `FatObj<T>` generic, which provides the initialization API for all built-in widgets, and wrapping our widgets in it gives the widgets all of the capabilities of the built-in widgets.

```rust
use ribir::prelude::*;

let radio = Radio { selected: true, value: Box::new(1.) };
let radio = FatObj::new(radio)
  .on_tap(|_| println!("Radio tapped"));
```

But in practice, instead of writing it this way, we usually create the widget via the `Declare` trait.

```rust
use ribir::prelude::*;

let btn: FatObj<State<Radio>> = Radio::declarer()
  .selected(true)
  .on_tap(|_| println!("Radio clicked"))
  .finish();
```

### Why should we use `Declare` to create widgets?

In the above example, we created the widgets in a similar way to the Builder pattern, which makes the process seem more complicated. However, this approach actually brings more advantages.


#### Complete Initialization API

Note that we end up creating a `FatObj<State<Radio>>` instead of a `Radio`. This is because with `Declare`, we can not only configure properties using the method of the same name, but also extend the capabilities of the built-in widget with `FatObj`. As for why we use `State`, it's because `State` allows you to have the state of your control listened to and modified.

```rust
use ribir::prelude::*;

let mut radio: FatObj<State<Radio>> = Radio::declarer()
  // We can use the built-in ability
  .on_tap(|_| println!("taped!"))
  .finish();

watch!($radio.selected)
  .subscribe(|selected| println!("The radio state change to {selected}"));
```

Of course, both `FatObj` and `State` only affect the overhead of the final constructed view if you use the capabilities they provide.

#### Support for initialization using `pipe!` streams

Another advantage of using `Declare` to create widgets is that it supports initializing properties with a `pipe!` stream. Properties initialized by a `pipe!` stream will change as the stream changes. For example, we want to create two `Radio`s, where the state of one follows the state of the other.


```rust
use ribir::prelude::*;

let mut radio1: FatObj<State<Radio>> = Radio::declarer()
  .selected(true)
  .finish();
let radio2 = Radio::declarer()
  .selected(pipe!($radio1.selected))
  .finish();

let _row = Row::declarer()
  .finish()
  .with_child(radio1)
  .with_child(radio2)
  .into_widget();
```

#### Support for accessing built-in widget properties

Note that while widgets created with `Declare` can be configured with all built-in capabilities directly, if you need to modify the properties of a built-in widget after initialization, you need to get the corresponding built-in widget before doing so. This is because these built-in widgets are composed on demand. In the following example, we create a button and change its margins when clicked:


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

## Composing child widgets

In Ribir, we use the `with_child` method to compose child widgets with their parent widgets to form new widgets. `@` syntax is also primarily implemented using `with_child`. In fact, you'll probably use it more often than you think.

For example, for a `Button`, the text it displays is even a child widget, not its property. This is because it can be either a text button or an icon button. If these were properties, then whether you use a text button or an icon button, memory would be allocated for properties you don't need. But if it's a child widget, it can be composed depending on the usage.

Here's an example of a text button and an icon button:

```rust
use ribir::prelude::*;

let text_btn = Button::declarer()
  .finish()
  .with_child("Text Button");

let icon_btn = Button::declarer()
  .finish()
  .with_child(Icon.with_child(named_svgs::get_or_default("search")));
```

## A mix of APIs and macros

Ribir's "DSL" is not an entirely new language, but rather a set of macros. Each macro can be used as a standalone expression, so you can mix and match them freely. Below we will implement a counter example. We'll create the text for the button and the counter directly through the API, and use `$` when initializing their properties to avoid cloning `cnt`. Finally, we'll combine them into a `Row` using the `@` syntax:

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

## Conclusion

We want everyone who uses Ribir to be able to choose how they want to use it, whether it's through the "DSL" or using the API directly, to get the best experience possible.

But what you need to understand is that Ribir's "DSL" is not a new language - we don't even call it a "DSL". It's built entirely on the API we described above, and is just a set of macros designed to make the UI structure clearer and more readable, and to avoid some obvious duplication of code, such as the need to clone State frequently because of move semantics.

In short, you can choose to use it partially or not at all, everything is free and there is no need to be intimidated by seeing new syntax. Continue your [Ribir journey](../get_started/quick_start.md)!
