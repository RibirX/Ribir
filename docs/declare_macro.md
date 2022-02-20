# The `declare!` macro

`declare!` macro provide a DSL language to help you to build your declare and reactive UI in a easy and expressive way which base on rust struct literal syntax but with a few extensions.

## Nested struct literal syntax

`declare!` macro support all rust struct literal syntax and can nested other struct literal as its children if it can accept children.

```rust
use ribir::prelude::*;

let _ = declare!{
  SizedBox {
    size: Size::new(100., 100.),
    Row {
      ..<_>::default(),
      Text {
        text: "hello ",
        style: <_>::default()
      }
      Text {
        text: "world",
        style: <_>::default()
      }
    }
  }
};
```

The above code declare a `SizedBox` widget has a `Row` child, and the `Row` widget has two `Text` child. 

Notice, the nested children must declare after the fields.

## Built-in fields to extend the literal syntax

In addition to using own fields of widget, `declare!` provide a dozens of built-in common fields that can used to any widget, like `padding` `margin` `background` and so on. [See the full list of built-in fields][builtin] to know what you can use.

```rust
use ribir::prelude::*;

let _ = declare!{
  SizedBox {
    size: Size::new(100., 100.),
    background: Color::RED,
    on_tap: |_| println!("Tapped!")
  }
};
```

Although, `SizedBox` not have field `background` and `on_tap`, but the code above is valid, because `declare!` macro provide built-in fields as sugar syntax to simplify use the commonly widgets or attributes.

## `if` guard syntax for field.

A `if` guard can be add to use filter the field, the syntax is same with `match` guard. 

```rust
use ribir::prelude::*;

let size  = Size::new(1., 1.);
let need_margin = true;

let _ = declare!{
  SizedBox {
    size,
    margin if need_margin =>: EdgeInsets::all(1.),
  }
};
```
## Expressions as children

At before we use struct literal to declare children, we also have an another option, pass any rust expression as children.

```rust
use ribir::prelude::*;

const ribir_steps: [&'static str; 5] = [
  "declare UI", "compile to international api", "layout and paint", 
  "generate triangles", "submit to gpu"
];

let _ = declare!{
  Row {
    ..<_>::default(),
    ribir_steps.iter().map(|text|  { 
      Text { text: (*text).into(), style: <_>::default() } 
    })
  }
};
```

Expression children can mixed with struct literal children, can declare one child or many children, it is limited by the parent widget implemented `SingleChildWidget` or `MultiChildWidget`. 

Notice, the expression return type must be:
- A widget type. 
- A `Option` of widget type
- A type which implemented `IntoIterator` and its iterate item is a widget type.

## Use `id` to access and follow widget in the whole `declare!`

`id` is a very special built-in field, it's use to named and identify the widget in the whole `declare!` scope and must be unique.
A widget with an `id` can be directly accessed in its `declare!` or embed `declare!` across the `id`. 

```rust
use ribir::prelude::*;

let _ = declare!{
  Row {
    ..<_>::default(),
    Checkbox {
      id: checkbox,
      checked: false,
      ..<_>::default(),
    }
    Text {
      text: "Change text background by checked state.".to_string(),
      style: <_>::default(),
      background: if checkbox.checked { Color::BLUE } else { Color:: RED }
    }
  }
};
```

In above code we declare a `Checkbox` named with `checkbox` and access it in `Text.background` field.

And there is a little more knowledge we need know here, ribir has a "keep the UI always displayed as how its declared" principle. So in the above code `Text.background` will always follow the change of `checkbox`. That means when `checkbox` has changed, the `Text.background` will also assigned across calc a new value from the field value expression. We called this data follow.

### Circular follow

Circular follow in struct literal is not allowed.

```rust ignore
use ribir::prelude::*;

let _ = declare!{
  Row {
    ..<_>::default(),
    Text {
      id: a,
      text: b.text.clone(),
      style: <_>::default()
    }
    Text {
      id: b,
      text: a.text.clone(),
      style: <_>::default()
    }
  }
};
```

Compiler will complain
```ascii
Can't init widget because circle follow: a.text ~> b, b.text ~> a 
```

In some cases, we may want a two-way follow, that it's a circular follow but what we want and is valid in logic. I will introduce it in next section.

### Widget owen fields and built-in fields not belong same widget

We introduced ribir provide some built-in fields to extends the struct literal syntax. But in essence, they do not belong to the same widget，so use same `id` not always mean follow on same widget.

```rust
use ribir::prelude::*;

let _ = declare!{
  SizedBox {
    id: a,
    size: Size::new(100., 100.),
    background: if a.size.area() > 10. { Color::RED } else { Color::BLACK }
  }
};
```
This code work fine, although is looks like a circular follow (a.background ~> a) , but background is not a field of `SizedBox`, so `a.size` and `a.background` belong to different widget, it's not a circular follow.

## Declare a data follow individual

We can declare a data follow implicitly in field value, but there is a explicit way to declare data follow after all widget declare.  Multi explicit data follow can split by `;`.

```rust
use ribir::prelude::*;

let _ = declare! {
  Column {
    ..<_>::default(),
    Row {
      ..<_>::default(),
      Checkbox { id: task , ..<_>::default() }
      Text { text: "Task", style: <_>::default() }
    }
    Row {
      margin: EdgeInsets::only_left(16.),
      ..<_>::default(),
      Checkbox { id: sub_task1 , ..<_>::default() }
      Text { text: "SubTask 1", style: <_>::default() }
    }
    Row {
      margin: EdgeInsets::only_left(16.),
      ..<_>::default(),
      Checkbox { id: sub_task2 , ..<_>::default() }
      Text { text: "SubTask 2", style: <_>::default() }
    }
  }
  data_flow!{
    sub_task1.checked && sub_task2.checked ~> task.checked;
    sub_task1.checked != sub_task2.checked ~> task.indeterminate
  }
};
```
Above code we implement a nested checkboxes which parent follow children change.
### Use `#[skip_nc]` to break circular follow.

A circular follow in struct literal is not allowed but it's allowed if some part of the circle declare with a `#[skip_nc]`. `#[skip_nc]` means skip no change, that tell the compiler check if the be followed expression result really different to the self value, modify the self value only if it's really changed.

A two-way follow work fine with `#[skip_nc]`

```rust
use ribir::prelude::*;

let _ = declare!{
  Row {
    ..<_>::default(),
    Text {
      id: a,
      text: "Hi",
      style: <_>::default()
    }
    Text {
      id: b,
      text: a.text.clone(),
      style: <_>::default()
    }
  }
  data_flow! { #[skip_nc] b.text.clone() ~> a.text }
};
```

`#[skip_nc]` can also be used in field.

```rust
use ribir::prelude::*;


let _ = declare!{
  Row {
    ..<_>::default(),
    Text {
      id: a,
      text: "Hi",
      style: <_>::default()
    }
    Text {
      id: b,
      #[skip_nc]
      text: a.text.clone(),
      style: <_>::default()
    }
  }
  data_flow! { b.text.clone() ~> a.text }
};
```
### Silent follow to avoid widget rebuild or layout.

A data follow works in a very simple way, follow a expression, when its result value may change(the `id` in the expression change),  calc the expression and assign the result to the target expression.

In some specific scenarios，we know some data follow need't effect the widget rebuild or layout. For example we have a `List` and its data follow ist child widget value, when its child changed the data follow modify back to `List` data. `List` data changed but needn't rebuild or relayout it. In this case we can use silent follow. Just call a `silent` method for the target `id`

There is a simple todo example, to show how it use.

```rust
#![feature(trivial_bounds, negative_impls)]

use ribir::prelude::*;


struct Todo  {
  finished: bool,
  label: String,
}

#[stateful(custom)]
struct Todos {
  tasks: Vec<Todo>
};

impl CombinationWidget for StatefulTodos {
   fn build(&self, ctx: &mut BuildCtx) -> BoxedWidget {
    let self_ref = self.state_ref();
    declare! {
      Column {
        cross_align: CrossAxisAlign::Start,
        ..<_>::default(),
        self.tasks.iter().enumerate().map(|(idx, task)|{
          let self_ref = self_ref.clone();
          declare!{
            Row {
              margin: EdgeInsets::vertical(4.),
              ..<_>::default(),
              Checkbox{
                id: checkbox,
                checked: task.finished,
                style: ctx.theme().checkbox.clone(),
                ..<_>::default(),
              }
              Text{
                text:task.label.clone(),
                style: <_>::default(),
                margin: EdgeInsets::vertical(4.),
              }
            }
            data_flow!{ checkbox.checked ~> self_ref.silent().tasks[idx].finished }
          }
        })
      }
    }
   }
}
```

See the `data_follow`, `self_ref` with a `silent` method call, this means when `checkbox` change, modify back  to `self_ref.silent().tasks[idx].finished`, but this modify not effect the `StatefulTodos` widget to rebuild.

## How to support widget work in `declare!` syntax ?

Every widget can be supported to use in `declare!` macro if it implemented the [`Declare`](declare). 

The easiest way it to derive the `Declare` trait. See more detail in the [`mod level document`][mod].

[declare]: ../ribir/declare/trait.Declare.html
[builtin]: #full-builtin-fields-list
[mod]: ../ribir/declare/index.html


[ ] introduction default attr