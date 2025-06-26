---
sidebar_position: 2
---

# Quick Start

This chapter will introduce you to all the syntax and common basic concepts of Ribir.

> You will learn
>
> - How to create and compose widgets
> - How to respond to events and manipulate data
> - How to make views automatically respond to data changes
> - How to build dynamic widgets
> - How to map your own data structures to views
> - How to use built-in widgets as part of other widgets
> - How to transform, separate, and trace the state — facilitating state transfer and controlling the scope of view updates

## What is a widget?

In Ribir, a widget is the basic unit that describes a view. It can be a button, a text box, a list, a dialog, or even the entire application interface. In code, it can be a function, a closure, or a data object.

If you don't quite understand the above, don't worry, because you don't need to focus on the widget construction process, and Ribir prohibits interference in this process. You just need to understand that Ribir divides all widgets into four categories:

- function widget
- `Compose` widget
- `Render` widget
- `ComposeChild` widget

This chapter will only introduce function widgets and `Compose` widgets. Because in most scenarios, these two types of widgets are sufficient to meet our needs. As advanced content, we will cover `Render` widgets and `ComposeChild` widgets in [Widgets In-depth](../understanding_ribir/widget_in_depth.md).

Please note the difference between `Widget` and widget in the entire context of Ribir. Widget is a generic term, while the capitalized `Widget` is a specific widget, also the pass for all widgets to enter the view.

## Function widget

A function or closure that returns a `Widget` is called a function widget. A function widget that can be called multiple times can be transformed into a `GenWidget`, and the root widget of our application requires a `GenWidget`.

Defining widgets through functions is the simplest and fastest way. In [Try Ribir](./try_it.md), you have already seen an example of a `Hello World!` function widget. In this section, we will continue the introduction using the `Hello World!` example.

### Defining widgets through function

Let's start by defining a `hello_world` function to complete our example.

```rust no_run
use ribir::prelude::*;

fn hello_world() -> Widget<'static> {
  let mut text = Text::declarer();
  text.with_text("Hello World!");
  text.finish().into_widget()
}

fn main() { 
  App::run(hello_world);
}
```

Because the `Text` widget only provides a declarative API creation method, we need to create its declarer with `Text::declarer()` and finish the creation with `finish()`. Then, we convert it to a `Widget` type using `into_widget()`.

For declarative widgets, we can also simplify their writing with `rdl!`.

```rust
use ribir::prelude::*;

fn hello_world() -> Widget<'static> {
  rdl!{ Text { text: "Hello World!" } }
    .into_widget()
}
```

We will delve into the details of `rdl!` in the section [Using `rdl!` to Create Objects](#using-rdl-to-create-objects). For now, let's put it aside.

> Tip
>
> The framework automatically implements the `into_widget` method for all types of widgets.

### Closures and `fn_widget!`

Since `hello_world` is not called by anyone else, you can rewrite it as a closure:

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

For creating function widgets through closures, Ribir provides a `fn_widget!` macro to simplify this process. Apart from supporting the two syntactic sugars `@` and `$` that we will talk about next in this chapter, you can think of `fn_widget!` as expanding the code like this:

```rust ignore
move || -> Widget {
  {
    // Your code
  }
  .into_widget()
}
```

Using `fn_widget!` to rewrite the `hello_world` example:

```rust no_run
use ribir::prelude::*;

fn main() {
  App::run(fn_widget! { 
    rdl!{ Text { text: "Hello World!" } }
  });
}
```

Usually, declarative widgets provide a macro of the same name, which creates a function widget rooted in itself using `fn_widget!`.

So, our example can be further simplified to:

```rust no_run
use ribir::prelude::*;

fn main() {
  App::run(text! { text: "Hello World!"});
}
```

This is the example we saw in [Try Ribir](./try_it.md).

## Using `rdl!` to Create Objects

`rdl` stands for Ribir Declarative Language, and the `rdl!` macro aims to help you create objects in a declarative way.

> Tips
>
> `rdl!` does not care about types, it only processes syntax at the language level, so it is not limited to widgets only.

### Declaratively Creating Objects

Although `rdl!` supports any Rust expression, when we talk about declarative object creation, we specifically refer to creating objects through struct literals:

```rust ignore
rdl! { 
  ObjectType {
    ... // Field declarations
  } 
}
```

When your expression is a struct literal, `rdl!` creates objects using the `Declare` trait, which requires the object type you create to inherit or implement the `Declare` trait.

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

In the example above, `Counter` inherits `Declare` and marks `count` with a default value of `1`. Therefore, in `rdl!`, you don't need to assign a value to `count`, as `rdl!` will default it to `1`. `Declare` has some other features, but we won't delve into them here.

### Expression-Based Object Creation

Besides creating objects through struct literals, you can also place any expression inside `rdl!`. This is useful when dealing with nested compositions, and it is only necessary when nesting as children. The following example shows how to use expressions to create objects in `rdl`:

```rust ignore
use ribir::prelude::*;

let _parent = rdl!{
  // You can write any expression here, and the result of the expression will be the child
  if  {
    ...
  } else {
    ...
  }
};
```

### Composing Widgets

Now that you know how to create widgets in `rdl!`, let's compose widgets to create a simple counter application.

You can nest widgets within struct literal declarations to create children using `rdl!`. Note that children are always required to be declared after their parent widget fields, which is a format requirement enforced by `rdl!`.

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

In the example above, we created a `Button` and composed it with a string as a child. `Button` is already defined in the `ribir_widgets` library.

`rdl!` also allows you to declare children for widgets that have already been created:

```rust no_run
use ribir::prelude::*;

fn main() {
  let counter = fn_widget! {
    let btn = rdl! { Button {} };
    rdl!{ 
      (btn) {
        rdl!{ "0" }
      }
    }
  };

  App::run(counter);
}
```

Notice the `rdl!{ $btn { ... } }` syntax? Similar to struct literal syntax, but with `$` in front, it indicates that the parent is a variable rather than a type, so it doesn't create a new widget but directly uses that variable to compose with the child.

> Tips
>
> In Ribir, the composition of parent and child is not arbitrary but is restricted by type, ensuring the correctness of the composition. 
> In our example, `Button` specifies that it can accept two optional children: a string as a label and a `Widget` as an icon.
> **Why is the label of the Button designed to be a child rather than its own field?** This is because, if it were a field of `Button`, it would occupy memory regardless of whether `Button` has a label or not. By making it a child, there is no memory overhead for this field when the Button doesn't have a label.

> We will delve into how to constrain the type of children for widgets in [Understanding Ribir](../understanding_ribir/widget_in_depth.md).


## `@` Syntactic Sugar

In the process of compose widgets, we used multiple `rdl!`. On one hand, it helps us have a clear declarative structure when interacting with Rust syntax (especially in complex examples) - when you see `rdl!`, you know that the composition or creation of a widget node has begun; on the other hand, when every node is wrapped with `rdl!`, it may appear too verbose, making it hard to see the key information.

Fortunately, Ribir provides an `@` syntactic sugar for `rdl!`, and in actual use, you mostly use `@` instead of `rdl!`. There are three main cases:

- `@Button {...}` as a struct literal's syntactic sugar, expands to `rdl!{ Button {...} }`
- `@ $btn {...}` as syntactic sugar for variable struct literals, expands to `rdl!{ $btn {...} }`
- `@ { ... }` is syntactic sugar for expressions, expanding to `rdl!{ ... }`

Now, let's rewrite the above counter example using `@`:

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

## State - Making Data Watchable and Shareable

Although we have created a counter, it always displays `0`, and clicking the button does not do anything. In this section, you will learn how to make your counter work using state.

State is a wrapper that makes data watchable and shareable.

The complete lifecycle of an interactive Ribir widget is as follows:

1. Convert your data into state.
2. Declare a mapping from state to view to build the view.
3. During interaction, modify the data through state.
4. Receive data changes through state and update the view point-to-point based on the mapping.
5. Repeat steps 3 and 4.

Now, let's introduce state to transform our example.

```rust no_run
use ribir::prelude::*;

fn main() {
  App::run(fn_widget! {
    // Change 1: Create a state using `State::value`
    let count = State::value(0);
    @Button {
      // Change 2: Modify the state on tap
      on_tap: move |_| *$count.write() += 1,
      // Change 3: Display data using state and keep the view updated.
      // For macros or function calls, you can omit the curly braces after `@`
      @ pipe!($count.to_string())
    }
  });
}
```

By making these 3 changes, the small counter example is complete. However, in changes 2 and 3, new elements are introduced — `$` and `pipe!`. They are crucial in Ribir, and understanding them will help you work effectively with state.

## $ Syntactic Sugar

In Ribir, there are two important syntactic sugars, one is the [`@` Syntactic Sugar](#-syntactic-sugar) introduced earlier, and the other is the `$` syntactic sugar.

### State Read and Write References

`$` indicates a read or write reference to the state that follows it. For example, `$count` represents a read reference to `count`, and when followed by a `write()` call, it represents a write reference to `count`, such as `$count.write()`.

Besides `write`, Ribir also provides a `silent` write reference, where modifying data through `silent` write does not trigger view updates.

The expansion logic of state's `$` syntactic sugar is as follows:

- `$counter.write()` expands to `counter.write()`
- `$counter.silent()` expands to `counter.silent()`
- `$counter` expands to `counter.read()`

### Automatic Sharing of States

When `$` is inside a `move` closure, it points to a cloned version of the state (read/write). The closure captures a clone of the state, allowing you to use the state directly and easily share it without needing to clone it explicitly.

```rust ignore
move |_| *$count.write() += 1
```

Expands roughly to

```rust ignore
{
  let count = count.clone_writer();
  move |_| *count.write() += 1
}
```

### Priority of Syntactic Sugar Expansion

Remember we also used `$` in [Composing Widgets](#composing-widgets)? For example, `rdl!{ $btn { ... } }` or `@ $btn { ... }`. This is not a reference to state data, as `rdl!` assigns a different semantic meaning to it — creating a parent widget using a variable declaration.

Whether it's `@` or `$`, they should first follow the semantics of the macro they are in, and then be considered as Ribir's syntactic sugar. When using `@` or `$` inside an external macro, they no longer act as Ribir's syntactic sugar, as the external macro likely gives them special meanings.

```rust ignore
use ribir::prelude::*;

fn_widget!{
  user_macro! {
    // `@` is not a syntactic sugar here, its semantics 
    // depend on the implementation of `user_macro!`
    @Button { ... }
  }
}
```


## `Pipe` stream -- keep responding to data

A `Pipe` stream is a continuously updated data stream with an initial value. It can be decomposed into an initial value and an rxRust stream -- the rxRust stream can be subscribed. It is also the only channel for Ribir to update data changes to the view.

Ribir provides a `pipe!` macro to help you quickly create a `Pipe` stream. It accepts an expression and monitors all states marked with `$` in the expression to trigger the recalculation of the expression.

In the following example, `sum` is a `Pipe` stream of the sum of `a` and `b`. Whenever `a` or `b` changes, `sum` can send the latest result to its downstream.

```rust 
use ribir::prelude::*;

let a = State::value(0);
let b = State::value(0);

let sum = pipe!(*$a + *$b);
```

When declaring an object, you can initialize its property with a `Pipe` stream, so that its property will continue to change with this `Pipe` stream. As we have seen in [State - Making Data Watchable and Shareable](#state---making-data-watchable-and-shareable)

```rust ignore
  @Text { text: pipe!($count.to_string()) }
```

### Rendering widgets dynamically

Up until now, the structure of all the views you've created has been static, with only the properties changing with the data, but the structure of the widgets does not change with the data. In fact, you can also create a continuously changing widget structure through the `Pipe` stream.

Let's say you have a counter, and instead of displaying the number in text, the counter counts the number in little red squares:


![box counter](../assets/box_counter.gif)

Code:

```rust no_run
use ribir::prelude::*;
fn main() {
  App::run_with_data(
    || Stateful::new(0),
    move |cnt: &'static Stateful<i32>| {
      row! {
        @Button {
          on_tap: move |_| *$cnt.write() += 1,
        @ { "Increment" }
        }
        @ {
          pipe!(*$cnt).map(move |cnt| {
            (0..cnt).map(move |_| {
              @Container {
                margin: EdgeInsets::all(2.),
                size: Size::new(10., 10.),
                background: Color::RED
              }
            })
          })
        }
      }
    },
  );
}
```

### Try to keep `pipe!` to the smallest possible expression.

Although `pipe!` can contain as many expressions as you like, it is recommended that you try to include only the smallest expressions in `pipe!` and then use `map` to complete the transformation. This allows you to see the source of changes in `pipe!` more clearly and avoids unnecessary dependencies in complex expressions. So, the example above writes

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

instead of:

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

### Operators for rxRust on `Pipe` chains

The update push of the `Pipe` stream is built on top of the rxRust stream, so `Pipe` also provides the method `value_chain` that lets you manipulate the rxRust stream. So you can use rxRust operators such as `filter`, `debounce` `distinct_until_change` and so on to reduce the frequency of updates.

Suppose you have a simple auto-summing example:


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
          .transform(|s| s.distinct_until_changed().box_it()),
        on_tap: move |_| {
          *$a.write() += 1;
          *$b.write() -= 1;
        }
      }
    }
  });
}
```

In the above example, the first two `Text`s are updated as `a` and `b` are modified, even if the values of `a` and `b` do not change - e.g., by setting the same values for them. The last `Text`, on the other hand, filters out updates with duplicate values via `distinct_until_changed`, and will only update if the result of the sum of `a` , `b` changes.

So when we click on the last `Text`, only the first two `Texts` will be marked as updated, and the last `Text` will not.


> Tip
> In general, to find out which part of the view is dynamically changing, you just need to look for where the `pipe!` is.


## Listening for expression changes with `watch!`

`watch!` is a macro that listens for changes to an expression, takes in an expression, and monitors all of the `$`-marked state in the expression to trigger a recalculation of the expression and push the latest results to downstream subscribers.


`watch!` listens for changes to the expression and has the same syntax as `pipe!`, but `pipe!` is initialized and behaves more like a continuously changing value than a subscribable stream, whereas `watch!` is only a subscribable stream, so `pipe!` can be used as a value to initialize the properties of the widget, whereas `watch!` cannot.

In short:

- `pipe!` = (initial value + rxRust stream)
- `watch!` = rxRust stream

You can also use `watch!` to implement your counter manually:

  
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

Once we call `subscribe`, we create a subscription to the expression in `watch!`. This subscription will exist until you manually call `unsubscribe`, or until the State that `watch!` is listening to no longer has a write source.

In the above example, we don't need to call `unsubscribe` because the subscription needs to exist throughout the application's lifecycle.

Typically, there are two cases where you need to call `unsubscribe` manually:

In the first case, you want the subscription to have a shorter lifecycle than the state it is listening to. A typical example of this situation is building widgets using external state, for example:


```rust
use ribir::prelude::*;

fn show_name(name: State<String>) -> Widget<'static> {
  fn_widget!{
    let mut text = @Text { text: "Hi, Guest!" };
    let u = watch!($name.to_string()).subscribe(move |name| {
      $text.write().text = format!("Hi, {}!", name).into();
    });

    // `name` is a shareable state that can be held by other people, 
    // making its lifecycle longer than that of the widget 
    // so we need to unsubscribe when the widget is destroyed.

    @(text) { on_disposed: move |_| u.unsubscribe() }
  }
  .into_widget()
}
```

In the second case, the downstream of `watch!` performs a write operation on the listened state. Because `watch!` relies on the listened state no longer having a write source to automatically unsubscribe, this constitutes a circular reference when its downstream holds a write source for the listened state. At this point, the subscription must be manually unsubscribed or a memory leak will result. Example:


```rust
use ribir::prelude::*;

let even_num = State::value(0);

// Respond to changes in even_num, ensure it is even. 
// If even_num is odd, add 1 to make it even
let u = watch!(*$even_num).subscribe(move |v| {
  if v % 2 == 1 {
    *even_num.write() = v + 1;
  }
});

// The following code needs to be called at the right time, otherwise it will result in circular references
u.unsubscribe()
```

## `Compose` widget - describing your data structure

Often, in complex real-world scenarios, you can't do it all by just creating some localized data and using simple function widgets. You need your own data structures, and you can map your data structures to views with the `Compose` widget.

Rewrite the counter example to use the `Compose` widget:

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

In the above example, when you implement `Compose` for `Counter`, `Counter` and all writable states of `Counter` are now a legal widget.


## Built-in widgets

Ribir provides a set of built-in widgets that allow you to configure the underlying styles, events, lifecycle, etc. The important difference between built-in widgets and regular widgets is that when you create a widget declaratively, you can use the fields and methods of the built-in widgets as if they were your own, and Ribir does the work of creating and compose the built-in widgets for you.

Let's take `Margin` for example, suppose you want to set a margin of 10 pixels for a `Text`, the code would look like this:


```rust no_run
use ribir::prelude::*;

fn main() {
  App::run(fn_widget! {
    // Declare `Margin` as the parent of `Text`
    @Margin {
      margin: EdgeInsets::all(10.),
      @Text { text: "Hello World!" }
    }
  });
}
```

But you don't have to explicitly declare a `Margin`, you can write it directly as:

```rust no_run
use ribir::prelude::*;

fn main() {
  App::run(fn_widget! {
    // Use the `Margin::margin` field directly in `Text`
    @Text {
      margin: EdgeInsets::all(10.),
      text: "Hello World!"
    }
  });
}
```


When you create a widget declaratively, you can access the fields of the built-in widgets directly, even if you don't show them declared (if you use them in your code, the corresponding built-in widgets will be created). For example:

```rust no_run
use ribir::prelude::*;

fn main() {
  App::run(fn_widget! {
    // `margin` is not declared
    let mut hello_world = @Text { text: "Hello World!" };
    // But you can still access the `margin` field,
    // It's created with default value when you use it. 
    $hello_world.write().margin = EdgeInsets::all(10.);
    hello_world
  });
}
```

This is extended by the generic type `FatObj`, refer to the API documentation for [`FatObj`](https://docs.rs/ribir_core/@RIBIR_VERSION/ribir_core/builtin_widgets/struct.FatObj.html) to see all the extensibility it provides.

## State Transitions, Separation, and Traceability

As you have learned after the previous chapters:

- State modifications to data cause dependent views to be updated directly
- The mapping of data to views is accomplished through `Compose`.

Assuming that `AppData` is the data for your entire application, you can map it to a view with `Compose`. However, if `AppData` is a complex piece of data, mapping the entire application view with a single `Compose` would be a disaster in terms of code organization; and relying on a single state for the entire application view would result in any modification to `AppData` updating the entire dynamic portion of the view, which in most cases would result in less than optimal interactive performance.

Luckily, Ribir provides a set of mechanisms for transforming, separating, and tracing state. It lets you start with a complete application state, then transform or separate the application state into smaller sub-states, which in turn can continue to transform or separate...; and within the sub-states you can transform or separate... ; and within the sub-states, you can access the source of your own transitions or separations through the traceability mechanism.

### Transitions and Separation, converting states into sub-states

**Transitions** are the transitions from a parent state to a child state. The parent and child states share the same data, and modifying the parent state is equivalent to modifying the child state, and vice versa. It simply reduces the visible scope of the data, making it easier if you want to use and pass only part of the state.
**Separation is the separation of child states from a parent state, where the parent and child states share the same data. The difference is that changing data in the child state does not trigger dependent view updates for the parent state.


You should note that the parent and child states share the same data regardless of whether they are transformed or separated. Therefore, their modifications to the data affect each other, but the scope of the data changes they push may be different.

Carefully reading the following examples will help you better understand how state transitions and separations work:

```rust
use ribir::prelude::*;

struct AppData {
  count: usize,
}

let state = State::value(AppData { count: 0 });
let map_count = state.part_writer(PartialId::any(), |d| PartMut::new(&mut d.count));
let split_count = state.part_writer(PartialId::any(), |d| PartMut::new(&mut d.count));

watch!($state.count).subscribe(|_| println!("Parent data"));
watch!(*$map_count).subscribe(|_| println!("Child(map) data"));
watch!(*$split_count).subscribe(|_| println!("Child(split) data"));
state
  .raw_modifies()
  .filter(|s| s.contains(ModifyEffect::FRAMEWORK))
  .subscribe(|_| println!("Parent framework"));
map_count
  .raw_modifies()
  .filter(|s| s.contains(ModifyEffect::FRAMEWORK))
  .subscribe(|_| println!("Child(map) framework"));
split_count
  .raw_modifies()
  .filter(|s| s.contains(ModifyEffect::FRAMEWORK))
  .subscribe(|_| println!("Child(split) framework"));

// Modify data through the split sub-state, the data modification push to both the parent and child state subscribers.
// But only the split sub-state subscribers are pushed framework notifications.
*split_count.write() = 1;
AppCtx::run_until_stalled();
// Print:
// Parent data
// Child(map) data
// Child(split) data
// Child(split) framework

// When data is modified through the parent state, both the data modification and framework notifications are pushed to the subscribers of the parent and child states. However, the split sub-state becomes invalidated.
state.write().count = 3;
// The push is asynchronous, forcing the push to be sent immediately
AppCtx::run_until_stalled();
// Print:
// Parent data
// Child(map) data
// Parent framework
// Child(map) framework

// Modify data through the map sub-state, the data modification push to both the parent and child state subscribers.
*map_count.write() = 2;
AppCtx::run_until_stalled();
// Print:
// Parent data
// Child(map) data
// Parent framework
// Child(map) framework
```

Because Ribir's data modification notifications are sent out in asynchronous batches, we call `AppCtx::run_until_stalled()` every time a data modification is made to force comprehensible sends in the example for ease of understanding, but this shouldn't be in your real code.


If you just want a read-only sub-state, then you can convert it with `part_reader`:

```rust ignore
let count_reader = state.part_reader(|d| &d.count);
```

But Ribir doesn't provide a ``split_reader``, because separating a read-only sub-state is equivalent to converting a read-only sub-state.


## The next step

You have mastered all the syntax and basic concepts needed to develop a Ribir application. Now it's time to put them into practice with [Exercise: Todos application](../practice_todos_app/develop_a_todos_app.md).
