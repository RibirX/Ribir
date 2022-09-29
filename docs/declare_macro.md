# The `widget!` macro

`widget!` macro provide a DSL language to help you to build your declare and reactive UI in a easy and expressive way which base on rust struct literal syntax and with a few extensions.

## Nested struct literal syntax

`widget!` macro support all rust struct literal syntax and can nested other struct literal as its children if it can accept children.

```rust
use ribir::prelude::*;

struct T;
impl CombinationWidget for T {
  #[widget]
  fn build(&self, ctx: &mut BuildCtx) -> BoxedWidget {
    widget! {
      declare SizedBox {
        size: Size::new(100., 100.),
        Row {
          Text { text: "hello " }
          Text { text: "world" }
        }
      }
    }
  }
}

```

The above code declare a `SizedBox` widget has a `Row` child, and the `Row` widget has two `Text` child. 

Notice, the nested children must declare after the fields.

## Built-in fields to extend your widget

In addition to using own fields of widget, `widget!` provide a dozens of built-in common fields that can used to any widget, like `padding` `margin` `background` and so on. [See the full list of built-in fields][builtin] to know what you can use.

```rust
use ribir::prelude::*;

struct T;
impl CombinationWidget for T {
  #[widget]
  fn build(&self, ctx: &mut BuildCtx) -> BoxedWidget {
    widget!{
      declare SizedBox {
        size: Size::new(100., 100.),
        background: Color::RED,
        tap: |_| println!("Tapped!")
      }
    }
  }
}
```

Although, `SizedBox` not have field `background` and `tap`, but the code above is valid, because `widget!` macro provide built-in fields as sugar syntax to simplify use the commonly widgets or attributes.

## `if` guard syntax for field.

A `if` guard can be add to use filter the field, the syntax is same with `match` guard. 

```rust
use ribir::prelude::*;

struct T {
  size: Size,
  need_margin: bool
}

impl CombinationWidget for T {
  #[widget]
  fn build(&self, ctx: &mut BuildCtx) -> BoxedWidget {
    widget!{
      declare SizedBox {
        size: self.size,
        margin if self.need_margin =>: EdgeInsets::all(1.),
      }
    }
  }
}

```
## Use `ExprWidget` to dynamic generate widget.

At before we use struct literal to declare children, we also can pass any rust expression as children.

```rust
use ribir::prelude::*;

struct RibirSteps;

impl CombinationWidget for RibirSteps {
  #[widget]
  fn build(&self, ctx: &mut BuildCtx) -> BoxedWidget {
    const ribir_steps: [&'static str; 5] = [
      "declare UI", "compile to international api", "layout and paint", 
      "generate triangles", "submit to gpu"
    ];

    widget!{
      declare Row {
        ExprChild { ribir_steps.iter().map(|&text| declare Text { text }) }
      }
    }
  }
}
```

Expression children can mixed with struct literal children, can declare one or multi, it is limited by the parent widget implemented `SingleChildWidget` or `MultiChildWidget`. 

Notice, the expression return type must be:
- A widget type. 
- A `Option` of widget type
- A type which implemented `IntoIterator` and its iterate item is a widget type.

## Use `id` to access and directly reactive to widget change.

`id` is a very special built-in field, it's use to named and identify the widget in the whole `widget!` scope and must be unique.
A widget with an `id` can be directly accessed in its `widget!` or embed `widget!` across the `id`. 

```rust
use ribir::prelude::*;

struct T;
impl CombinationWidget for T {
  #[widget]
  fn build(&self, ctx: &mut BuildCtx) -> BoxedWidget {
    widget!{
      declare Row {
        Checkbox {
          id: checkbox,
          checked: false,
        }
        Text {
          text: "Change text background by checked state.",
          background: if checkbox.checked { Color::BLUE } else { Color:: RED }
        }
      }
    }
  }
}
```

In above code we declare a `Checkbox` named with `checkbox` and access it in `Text.background` field.

And there is a little more knowledge we need know here, ribir has a "keep the UI always displayed as how its declared" principle. So in the above code `Text.background` will always follow the change of `checkbox`. That means when `checkbox` has changed, the `Text.background` will also assigned across calc a new value from the field value expression. We called this state follow.

### Circular follow

Circular follow in struct literal is not allowed.

```rust compile_fail
use ribir::prelude::*;

struct T;
impl CombinationWidget for T {
  #[widget]
  fn build(&self, ctx: &mut BuildCtx) -> BoxedWidget {
    widget!{
      declare Row {
        Text {
          id: a,
          text: b.text.clone(),
        }
        Text {
          id: b,
          text: a.text.clone(),
        }
      }
    }
  }
}

```

Compiler will complain
```ascii
Can't init widget because circle follow: a.text ~> b, b.text ~> a 
```

In some cases, we may want a two-way follow, that it's a circular follow but what we want and is valid in logic. I will introduce it in next section.

### Widget fields and built-in fields not belong same widget

We introduced ribir provide some built-in fields to extends the struct literal syntax. But in essence, they do not belong to the same widget，so use same `id` not always mean follow on same widget.

```rust
use ribir::prelude::*;

struct T;
impl CombinationWidget for T {
  #[widget]
  fn build(&self, ctx: &mut BuildCtx) -> BoxedWidget {
    widget!{
      declare SizedBox {
        id: a,
        size: Size::new(100., 100.),
        background: if a.size.area() > 10. { Color::RED } else { Color::BLACK }
     }
    }
  }
}
```
This code work fine, although is looks like a circular follow (a.background ~> a.size) , but if you look the [`SizedBox`]! define, you will find background is not a field of `SizedBox`, `background` is a syntax extend builtin field, so `a.size` and `a.background` belong to different widget, it's not a circular follow.

## Declare a data follow individual

We can declare a data follow implicitly in field value, but there is a explicit way to declare data follow after all widget declare.  Multi explicit data follow can split by `;`.

```rust
use ribir::prelude::*;

struct T;
impl CombinationWidget for T {
  #[widget]
  fn build(&self, ctx: &mut BuildCtx) -> BoxedWidget {
    widget! {
      declare Column {
        Row {
          Checkbox { id: task }
          Text { text: "Task" }
        }
        Row {
          margin: EdgeInsets::only_left(16.),
          Checkbox { id: sub_task1 }
          Text { text: "SubTask 1" }
        }
        Row {
          margin: EdgeInsets::only_left(16.),
          Checkbox { id: sub_task2  }
          Text { text: "SubTask 2" }
        }
      }
      dataflows {
        sub_task1.checked && sub_task2.checked ~> task.checked;
        sub_task1.checked != sub_task2.checked ~> task.indeterminate
      }
    }
  }
}
```
Above code we implement a nested checkboxes which parent follow children change.
### Use `#[skip_nc]` to break circular follow.

A circular follow in struct literal is not allowed but it's allowed if some part of the circle declare with a `#[skip_nc]`. `#[skip_nc]` means skip no change, that tell the compiler check if the be followed expression result really different to the self value, modify the self value only if it's really changed.

A two-way follow work fine with `#[skip_nc]`

```rust
use ribir::prelude::*;

struct T;
impl CombinationWidget for T {
  #[widget]
  fn build(&self, ctx: &mut BuildCtx) -> BoxedWidget {
    widget!{
      declare Row {
        Text { id: a, text: "Hi" }
        Text { id: b, text: a.text.clone() }
      }
      dataflows  { #[skip_nc] b.text.clone() ~> a.text }
    }
  }
}
```

`#[skip_nc]` can also be used in field.

```rust
use ribir::prelude::*;


struct T;
impl CombinationWidget for T {
  #[widget]
  fn build(&self, ctx: &mut BuildCtx) -> BoxedWidget {
    widget!{
      declare Row {
        Text { id: a, text: "Hi" }
        Text {
          id: b,
          #[skip_nc]
          text: a.text.clone(),
        }
      }
      dataflows  { b.text.clone() ~> a.text }
    }
  }
}
```
### Silent follow to avoid widget rebuild or layout.

A data follow works in a very simple way, follow a expression, when its result value may change(the `id` in the expression change),  calc the expression and assign the result to the target expression.

In some specific scenarios，we know some data follow need't effect the widget rebuild or layout. For example we have a `List` and its data follow ist child widget value, when its child changed the data follow modify back to `List` data. `List` data changed but needn't rebuild or relayout it. In this case we can use silent follow. Just call a `silent` method for the target `id`

There is a simple todo example, to show how it use.

```rust

use ribir::prelude::*;


struct Todo  {
  finished: bool,
  label: String,
}

struct Todos {
  tasks: Vec<Todo>
};

impl StatefulCombination for Todos {
    #[widget]
    fn build(this: &Stateful<Self>, ctx: &mut BuildCtx) -> BoxedWidget {
      let this_ref = unsafe { this.state_ref() };
      widget! {
        declare Column {
          h_align: CrossAxisAlign::Start,
          ExprChild {
            this_ref.tasks.iter().enumerate().map(|(idx, task)|{
              widget!{
                declare Row {
                  margin: EdgeInsets::vertical(4.),
                  Checkbox{
                    id: checkbox,
                    checked: task.finished,
                  }
                  Text{
                    text:task.label.clone(),
                    margin: EdgeInsets::vertical(4.),
                  }
                }
                dataflows { checkbox.checked ~> this_ref.silent().tasks[idx].finished }
              }
            })
          }
        } 
      }
    }
}
```

See the `data_follow`, `this_ref` with a `silent` method call, this means when `checkbox` change, modify back  to `this_ref.silent().tasks[idx].finished`, but this modify not effect the `StatefulTodos` widget to rebuild.


## use `ctx`
## How to support widget work in `widget!` syntax ?

Every widget can be supported to use in `widget!` macro if it implemented the [`Declare`](declare). 

The easiest way it to derive the `Declare` trait. See more detail in the [`mod level document`][mod].

[declare]: ../ribir/declare/trait.Declare.html
[builtin]: #full-builtin-fields-list
[mod]: ../ribir/declare/index.html
