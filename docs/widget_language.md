# The `widget!` macro

`widget!` macro is a declarative language to help you to build your declare and reactive UI in a easy and expressive way. Which allows user interfaces to be described in terms of their visual struct, and almost all its syntax are expanded from Rust syntax self, so don't worry too much about the learning curve of this DSL language. 

In itself, it's a macro return a widget, so you can use it as an expression in anywhere you want.

Let's learn its syntax by building a greeting application.

## Nested struct literal syntax to describe widget hierarchy

`widget!` macro describe widget use rust struct literal syntax.

```rust ignore
use ribir::prelude::*;

fn main() {
  let hi = widget! {
    Text { text: "Hello world!" }
  };

  app::run(hi);
}
```

At first, we import `ribir::prelude::*`, which is the list of basic things that a `Ribir` application need use. In the `main` function, we declare a `hi` widget and use it as a root widget to run a application. If you run `cargo run`, a window with text `Hello world!` will launch.

`Text` is a widget provide by ribir, which use to display simple text. It have two fields, the `text` use to specify what text to display, and the `style` specify the style of the text. At here we only use `text`, and `style` will be a default value.

> Tips
>
> Any struct can support declare in `widget!` if it derived `Declare` trait. `Declare` provide the default value for `style` of `Text`. See more in [How `Declare` trait work [wip] ?]().

Next step, we decide to let it support to say hi to anyone, user enter what the `Text` display hello to what. So we need a input to accept user enter.

```rust 
  let hi = widget! {
    Column {
      Input { 
        PlaceHolder::new("Enter the name you want to greet.")
      }
      Text { text: "Hello world!" }
    }
  };
```

In `widget!` user interfaces to be described in terms of their visual hierarchy. The above code declare a `Column` widget has two child `Input` and `Text`. And `PlaceHolder` is the child of `Input`. The child always nested declared in parent. Except struct literal syntax, we allow describe leaf widget across tuple struct or function call, like `PlaceHolder` in here.

For now, we have a input to accept user enter and use a `Column` to place the `Input` and `Text` in vertical. But the text of `Text` still not changed follow user enter.

## Use `id` to access and directly reactive to widget modify.

Every object declared in `widget!` can be named by `id` field, and it must be unique in whole `widget!` scope. After a widget named with `id`, its id can be directly accessed like a smart pointer in `widget!` or embed `widget!`. 

`id` implicit described the relationship between objects, for example, if a field of object initialize with a expression contain `id`, that also means the field value will reactive to the modifies of `id` to update.

```rust 
let hi = widget! {
  Column {
    Input {
      id: input,
      Placeholder::new("Enter the name you want to greet.")
    }
    Text { 
      text: {
        let value = &*input.text();
        format!("Hello {value}!")
      } 
    }
  }
};
```

We named the `Input` with `input`, and use its text to format a new text for `Text`. Now, the `Text` will reactive to user input.

> **Tips**
>
> we not directly format the text in one line like `format!("Hello {}!", input.text())`, that because ribir will not deep in external macro to analyze, so if we write `input.text()` in a macro, the expression result will not reactive to the modifies of `input`.
## Built-in abilities compose to any widget.

After these codes work as we expected, let's try to beautify our style.

```rust 
let hi = widget! {
  Column {
    Input {
      id: input,
      h_align: HAlign::Center,
      Placeholder::new("Enter the name you want to greet.")
    }
    Text { 
      text: {
        let value = &*input.text();
        format!("Hello {value}!")
      },
      h_align: HAlign::Center,
      v_align: VAlign::Center,
    }
  }
};
```

Now, we center `Input` horizontally, and center the `Text` both horizontally and vertically, just use the `h_align` and `v_algin`.

But wait, where are `h_align` and `v_align` come from? Both `Input` and `Text` not have these fields. That because Ribir provide dozens of built-in widgets. The fields and methods of the built-in widgets can directly use like itself fields of any widgets which declare in the `widget!`, like `padding` `margin` `background` and so on. [See the full list of built-in fields][builtin] to know what you can use.

> **Tips**
>
>Ribir provide some built-in fields to extends the struct literal syntax. But remember **widget fields and built-in fields not belong same widget**. In essence, they work together in a compose way, that means if user use a builtin field in a widget, the builtin widget compose the widget to a new widget. In this way, any widget get many abilities from builtin widgets and pay the memory overhead only if user really use it. 

## Use states to declare more stateful object.

This app always immediately change the greet text after user entered every char even if the name not enter finished. In this section, we'll add a button to let user manually submit the name after enter finished and count how many people did we already greet.

```rust 
let hi = widget! {
  states { counter: Stateful::new(0) }
  Column {
    Row {
      align_items: Align::Center,
      Input {
        id: input,
        Placeholder::new("Enter the name you want to greet.")
      }
      Button {
        tap: move |_| {
          let name = &*input.text();
          greet.text = format!("Hello {name}!").into();
          input.set_text("");
          *counter += 1;
        },
        ButtonText::new({
          let counter = counter.to_string();
          format!("Greet!({counter})")
        })
      }
    }
    Text { 
      id: greet,
      text: "Hello world!",
      h_align: HAlign::Center,
      v_align: VAlign::Center,
    }
  }
};
```

The most important change is we add `states { counter: Stateful::new(0) }` before all. We call it `states` block. It's start with a keyword `states`, and declare stateful object in a `key: value` mode, when the stateful object declared in this block modified, it'll also be reactive by others that used it in the `widget` macro, have same behavior like a object named by `id`.

The other change is we add `Button` after the `Input`, and update the `Input`, `Text` and `counter` when user tapped it.

`states` block it very useful to tell `widget!` macro we have other stateful objects need to reactive. At here, we defined a stateful object `counter`, and use as part text of `Button`. We declare counter as stateful, so when we modified the `counter` after user tapped the button, the button text will auto update.
## Use `DynWidget` to declare a dynamic widget hierarchy.

You may have found, our application has static widget tree, and even if user not enter anything, there was a "Hello world!" displayed. In this section, we'll introduce a special widget `DynWidget` and use it to conditionally generate the `greet` widget.

`DynWidget` has a `dyns` field to accept dynamic widgets, `dyns` value will replace `DynWidget` self as its parent children. When dyns modified, the widgets updated. The `dyns` value type limited by `DynWidget`'s parent, usually it can be three kinds:

- The child type of `DynWidget`'s parent want. 
- A `Option` of `C`, if `DynWidget`'s parent accept one or multi `C` as its child.
- A type which implemented `IntoIterator` and its iterate item is `C`, if `DynWidget`'s parent accept  multi `C` as its children.

Now, we use `DynWidget` to control if we need a greet `Text`:

```rust
DynWidget {
  dyns: (*counter > 0).then(|| {
    widget! {
      Text { 
        id: greet,
        text: "Hello world!",
        h_align: HAlign::Center,
        v_align: VAlign::Center,
      }
    }
  })
}
``` 

This `DynWidget` generate an optional widget detect by if `counter` is greater than zero. 

> **Tips**
>
> `DynWidget` also support accept child depends on if the type of `dyns` allow.  Its children is static. The dynamic part only limit in `dyns` field.
>```rust
>Column {
>  DynWidget {
>    dyns: (*counter > 0)
>      // this `Row` is dynamic.
>      .then(||Row::default())
>    // this `Row` is always exist.
>    // if the 'dynamic' Row is Some-value, this `Row` is the child of it. 
>    // otherwise, it's the child of the `Column`
>    Row {}
>  }
>}
>```


We putting all code together:

```rust
use ribir::prelude::*;

fn main() {
  let hi = widget! {
    states { counter: Stateful::new(0) }
    Column {
      Row {
        align_items: Align::Center,
        Input {
          id: input,
          Placeholder::new("Enter the name you want to greet.")
        }
        Button {
          tap: move |_| {
            let name = &*input.text();
            greet.text = format!("Hello {name}!").into();
            input.set_text("");
            *counter += 1;
          },
          ButtonText::new({
            let counter = counter.to_string();
            format!("Greet!({counter})")
          })
        }
      }

      DynWidget {
        dyns: (*counter > 0).then(|| {
          widget! {
            Text {
              id: greet,
              text: "Hello world!",
              h_align: HAlign::Center,
              v_align: VAlign::Center,
            }
          }
        })
      }
    }
  };

  app::run(hi);
}
```

When we compile, the compiler complained not found `greet`:

```
error[E0425]: cannot find value `greet` in this scope
  --> ribir/examples/greet.rs:16:13
   |
16 |  greet.text = format!("Hello {name}!").into();
   |  ^^^^^ not found in this scope
```

That because the `tap` callback of `Button` try to access `greet`, but `greet` declare in a embed `widget!` in `DynWidget`. `widget!` can access any `id` in same `widget!` scope or outside `widget!`, but not allow the embed `widget!`. In this case, when we think deeply, we'll find `greet` not always exists for outside `widget!` scope.

In next section, we'll resolve it across access `Button` and `input` in the embed `widget!` instead of access `greet` text in outside `widget!`.

## `init` and `finally` block

As before, we introduced `states` block to declare more stateful object. There are two other blocks we can used in `widget!`, that `init` and `finally` block.

`init` and `finally` are blocks only accept statements and run these statements in `widget!`.

`init` run statements after `states` block and before any others in `widget!`, and the variable in `init` block can be accessed in whole `widget!` scope except `states`.

`finally` run statements after everything declare in `widget!` scope but before `widget!` compose the finally widget to return.

### Subscribe `tap` event in `finally` block

Back to our greet app, first, we update the `tap` callback and only use it to update the `counter` and give an name `submit` to `Button`, so we can access it in the embed `widget!`.

```rust
Button {
  id: submit,
  tap: move |_| *counter += 1,
  ButtonText::new({
    let counter = counter.to_string();
    format!("Greet!({counter})")
  })
}
```

Then we add `finally` block in the embed `widget!`, and subscribe the tap stream of `submit` to update the `greet` text and reset the `input`.

```rust
finally {
  let guard = submit
    .tap_stream()
    .subscribe(move |_| {
      let name = &*input.text();
      greet.text = format!("Hello {name}!").into();
      input.set_text("");
    })
    .unsubscribe_when_dropped();
  move_to_widget!(guard);
}
```

Let’s review this `finally` block in detail.

`finally` is the keyword of the block, all statements wrapped by `{}` it as part of the block.

In this block there are two statements. Let deep in one by one.

In first statement, `submit.tap_stream()` return the tap event as an ReactiveX `Observable` stream, then we subscribe it to update `greet` text and reset `input`. The name of `unsubscribe_when_dropped` self described, it convert the handle to a variable that will auto `unsubscribe` when it dropped. So the whole statement subscribe the tap event of `submit` and create a variable to manage the subscribe lifetime.

The next statement `move_to_widget!(guard);` is very simple. `move_to_widget!` move `guard` to `widget!`, in other word, transfer the ownership of `guard`. So the `guard` will live as long as `widget!`, at here it's the `greet`.

> **Tips**
>
> - ReactiveX is an API for asynchronous programming with observable streams. [See More](reactivex.io)
> - rxRust is the implementation of ReactiveX Ribir used. [See More](github.com/rxRust/rxRust)

### use `watch` and `let_watch!` to watch expression.

In `finally` block, we subscribe the `tap` event stream. Here we provide an other way to do same thing. And we will introduce `watch!` and `let_watch!` macro. They're very useful macro let we can subscribe the modifies of an expression result.

We already have a stateful object `counter`, and it increment self after every tap of `submit` button. So we can update the `greet` text after the `counter` modifies. We'll use `watch!` macro to do it. `watch!` help us to convert a expression to value stream.

> Tips
>
> `watch!` macro subscribe all stateful objects in the expression and recalculate it when any stateful objects modifies. 

We use `watch!(counter)` instead of `submit.tap_stream()` now.

```rust
finally {
  let guard = watch!(counter)
    .subscribe(move |_| {
      let name = &*input.text();
      greet.text = format!("Hello {name}!").into();
      input.set_text("");
    })
    .unsubscribe_when_dropped();
  move_to_widget!(guard);
}
```

`let_watch!` is a more convenient macro if you just want to subscribe an expression. Like `watch!`, it convert an expression to a value stream, but also auto unsubscribe this when the root of `widget!` over. We use `let_watch!` instead of `watch!`, `.unsubscribe_when_dropped()` and `move_to_widget!(guard)` not necessary anymore.
`
```rust
finally {
  let_watch!(counter)
    .subscribe(move |_| {
      let name = &*input.text();
      greet.text = format!("Hello {name}!").into();
      input.set_text("");
    });
}
```

> Tips
>
> In most case, `let_watch!` is a more convenient way, but `watch!` is more flexible in some complex situation.
### Access `BuildCtx` in `init` block

Our `greet` text with a default style, in this section we'll use larger letters to highlight it. Instead of hard code for greet `style`, we'll use the `Theme` to init it. 

First, let's add `init` block after `states`

```rust
init ctx => {
  let style = TypographyTheme::of(ctx).headline1.text.clone();
}

```
`init` block has same syntax with `finally`, but the above code has a little more stuff (`ctx =>`) than what the `finally` block we used before.

`ctx =>` it's a syntax we use to named `BuildCtx`. At here, `ctx` is the name of `BuildCtx`. Because the `finally` block not use `BuildCtx`, so it be omitted. Let deep into the body, we create a style variable, and init it by the predefined theme.

Second, we use `style` to init the `greet` text.

``` rust
Text { 
  id: greet,
  text: "Hello world!",
  style,  // new line.
  h_align: HAlign::Center,
  v_align: VAlign::Center,
}
```

> **Tips**
>
> - `TypographyTheme` config the text style used in application, it as a part of `Theme`. Click [`Introduction to Theme[wip]`] to learn more.
> - `BuildCtx` provide many context of the application the `widget!` will be used for.

## Animations

We build a greet application, but the greet text transition without animation looks a bit blunt. In this section, we will add animations to transition the greet text and cover the basic knowledge of ribir animations.

At first, we need to split `greet` as three `Text`, because the `Hello` and `!` part will never change, our animate only work on the name part.

We change the single `Text`

``` rust
Text { 
  id: greet,
  text: "Hello world!",
  style,
  h_align: HAlign::Center,
  v_align: VAlign::Center,
}
```

To a `Row` widget with three `Text`, and apply `h_align` and `v_align` to `Row`.

``` rust
Row {
  h_align: HAlign::Center,
  v_align: VAlign::Center,
  Text { text: "Hello ", style: style.clone() }
  Text {
    id: greet,
    text: "World",
    style: style.clone()
  }
  Text { text: "!" , style }
}
```

Then, we can directly update the `greet` text by the text of `input`.

```rust
finally {
  let_watch!(counter)
    .subscribe(move |_| {
      greet.text = input.text();
      input.set_text("");
    });
}
```

After widgets ready, we will add a "ease in" animate to transition `greet` text change. Ribir do animate all across the `Animate` object. Let's declare an `Animate` object after the `Row` widget.

```rust
Animate {
  id: greet_new,
  transition: Translation {
    delay: None,
    duration: Duration::from_mills(200),
    easing: easing::EASE_IN,
    repeat: None,
  },
  prop: prop!(greet.transform),
  from: Transform::translation(0., greet.layout_height() * 2.)
}
```

We already introduced `id` before. At here we use it named the object as `greet_new`. `Animate` has three public fields `transition`, `prop` and `from`. These three fields described how does an animate run. 

- `transition` field accept a type which implemented `Roc` (Rate of change) trait, Ribir provide `Transition` as the standard implementation. We use `Transition` describe how property animate smoothly from another value to current value over time. 
> **Tips**
>
> `Transition` also support declare individual, Animate can use it across its `id`.
>```rust
>Transition {
>   id: ease_in,
>   duration: Duration::from_mills(200),
>   easing: easing::EASE_IN,  
>}
>Animate {
>  id: greet_new,
>  transition: ease_in.clone_stateful(),
>  prop: prop!(greet.transform),
>  from: Transform::translation(0., greet.layout_height() * 2.)
>}
>```

- `prop` field accept the animate property need transition, animate property is a type that implemented `AnimateProperty`. At here we want to do animate for `transform` of `greet`. `prop!(greet.transform)` help us to construct an animate property by a chaining path of field, and the path must be start with a stateful object.

> **Tips**
>
> - `transform` is a builtin field, so we can directly used it even if it's not a field of `Text`.
> - In fact, `prop!` also has a second argument called lerp function. Lerp function require to implement `Fn(&P, &P, f32) -> P`, it use to linearly interpolate between two property value. If we provide a lerp function argument, that means this property should use this function to calculate interpolate value. Otherwise the type of the property must implement `Lerp`. So we can write the property as
> ```rust
>// Do same thing as `prop!(greet.transform)`.
>prop!(greet.transform, |from, to, rate| from.lerp(to, rate))
>```
> - We also can use tuple to group multi property. For example 
>```rust ignore
>Animate {
>   transition: ..,
>   prop: (
>     prop!(greet.transform), 
>     prop!(greet.opacity),
>   ), 
>   from: (Transition::scale(0., 0.), 0.)
>}
>
>```

- `from` field accept a value of property which the animate start from.

Someone may be confused, we have a `from` field describe animate come from, but why not have a `to` field describe where to go. In Ribir,  animations are only visual effect, and should not effect any data of the application. Ribir animate always finished at the value of the animate property, that's where the animation is going.

> Tips
>
> Because animations are only visual effect,  the animate property must be a field of `Render` widget.

We have learned how to declare an `Animate` object, the next step is trigger it. We need to trigger it when `greet` text modified, so add this statement to `finally` block.
```rust
let_watch!(greet.text)
  .subscribe(move |_| greet_enter.run());
```

The animate work as we expected now. In practical we usually use the predefined `Transition` in theme. We polish over animate code.

Add a `init` block, before `Row` widget.

```
init ctx => {
  let ease_in = transitions::EASE_IN.of(ctx);
}
```
Then use `ease_in` to initialize the `Animate`.

```
Animate {
  id: greet_new,
  transition: ease_in,
  prop: prop!(greet.transform),
  from: Transform::translation(0., greet.layout_height() * 2.)
}
```

Let review the all code of the widget.

```rust 
 let hi = widget! {
    states { counter: Stateful::new(0) }
    init ctx => {
      let style = TypographyTheme::of(ctx).headline1.text.clone();
    }
    Column {
      Row {
        align_items: Align::Center,
        Input {
          id: input,
          Placeholder::new("Enter the name you want to greet.")
        }
        Button {
          tap: move |_| *counter += 1,
          ButtonText::new({
            let counter = counter.to_string();
            format!("Greet!({counter})")
          })
        }
      }
      DynWidget {
        dyns: (*counter > 0).then(|| {
          widget! {
            init ctx => {
              let ease_in = transitions::EASE_IN.of(ctx);
            }
            Row {
              Text { text: "Hello ", style: style.clone() }
              Text {
                id: greet,
                text: "World",
                style: style.clone()
              }
              Text { text: "!" , style }
            }
            Animate {
              id: greet_new,
              transition: ease_in,
              prop: prop!(greet.transform),
              from: Transform::translation(0., greet.layout_height() * 2.)
            }
            finally {
              let_watch!(counter)
                .subscribe(move |_| {
                  greet.text = input.text();
                  input.set_text("");
                });
              let_watch!(greet.text)
                .subscribe(move |_| greet_new.run());
            }
          }
        })
      }
    }
  };
```

> **Tips** 
> 
>In addition to a "standard" trigger animate that declare an `Animate` then manual trigger it, there is a syntax sugar to quick transition property change, it start with keyword  `transition`. It give an animate effect when its value changed.
>
>```rust ignore
>transition prop!(greet.background) {
>  duration: Duration::from_mills(200),
>  easing: easing::EASE_IN,
>}
>```
>
>Animate property followed the `transition` keyword, and the next block declare a `Transition` object.
>
>Instead of declare a new `Transition` object, we also can use `by` field to provide a `Transition` object by a expression.
>
>```rust ignore
>transition prop!(greet.background) {
>  // assume `ease_in` is a `Transition` variable.
>  by: ease_in
>}
>```

That's all, we've covered all the syntax of `widget!`.

