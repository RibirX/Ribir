# Custom widget declare in macro.

Any widget implement `Declare` trait can be declared in `widget!`. And Ribir provide a derive macro make implement `Declare` trait super easy.

> Tips
> 
> Almost all widget provide by `Ribir` implement `Declare` across derive it. If we have no special reason，we shouldn't implement by self, just derive it directly. To implement by self, see the [derive macro document](declare_derive) to know how it work.

In this chapter, we'll learn how to use the meta of `#[declare(...)]` attribute on the field. And it help us improve the declaration user experience about our widget.

Let's build a hero card widget which contain name, email and telephone. And the name is required, email and telephone are optional.

 ```rust
#[derive(Declare)]
struct HeroCard {
  name: String,
  tel: Option<String>,
  email: Option<String>,
}

impl Compose for HeroCard {
  fn compose(this: State<Self>) -> Widget {
    unreachable!("We don't care how implement `HeroCard` here, but focus on how to use it.")
  }
}

fn main() {
  let _ = widget!{
    HeroCard {
      name: "Mr Ribir".to_string(),
      tel: None,
      email: Some("ribir@XXX.com".to_string()),
      margin: EdgeInsets::all(8.)
    }
  };
}
```
After `HeroCard` derived `Declare`, it can be declared in `widget!` and builtin fields can be used in it. Looks good but not enough, the declaration of  `HeroCard` is too verbose.

- Does we can initialize `name` by `&str` directly ? Because `&str` can convert to `String`.
- `tel` is initialized by None, does we can omit it?
-  can we strip the `Some` of `email` ? If we use a value to initialize a option type, it's implicitly contain the mean that it's a `Some-Value`.

Of course we can, just use meta of `#[declare(...)]`, let's introduce one by one.

## use `convert` meta to convert value from another type.

Let's answer the problems of `name` accept `&str` and `tel` stripe the `Some` wrap first. Both they are type convert problem.

```rust
#[derive(Declare)]
struct HeroCard {
  #[declare(convert=into)]            // new!
  name: String,
  #[declare(convert=strip_option)]    // new!
  tel: Option<String>,
  #[declare(convert=strip_option)]    // new!
  email: Option<String>,
}
```
We just add `#[declare(convert=into)]` or `#[declare(convert=strip_option)]` for the fields, that all. Except `into` and `strip_option` there are two other value can be used in `#[declare(convert=...)]`. Let's introduce one by one.

- `#[declare(convert=into)]`, use `std::convert::Into` convert the value before initialize the field. With this meta, any type implemented `Into` trait to the field type can be used to initialize the field. 
- `#[declare(convert=strip_option)]`, wrap `V` to `Some(V)` before initialize `Option<V>`, of course `Option<V>` also be accepted.
- `#[declare(convert=box_trait(...))]`, convert the value to a box dyn type, also provide an optional `wrap_fn` argument to warp the box dyn type as finally result. For example, the `debug` filed accept any type that implemented `Debug` trait and auto wrap with `Box` and `RefCell`.
  ```rust
  #[Derive(Declare)]
  struct Printer {
    #[declare(convert=box_trait(Debug, wrap_fn=RefCell::new))]
    debug: RefCell<Box<dyn Debug>>
  }
  ```
- `#[declare(convert=custom)]`, implement the filed build by self, implement the same name method for its declarer to accept the initialize type. Then implement a `set_declare_xxx` method  for its host type, so it can be updated by the type you want.  For example
  ```rust
  #[Derive(Declare)]
  struct Printer {
    #[declare(convert=custom)]
    debug: RefCell<Box<dyn Debug>>
  }

  impl PrinterDeclarer {
    pub fn debug(mut self, debug: impl Debug + 'static) -> Self {
      self.debug = Some(RefCell::new(Box::new(debug)));
      self
    }
  }

  impl Printer {
    pub fn set_declare_cursor<C: IntoCursorIcon>(&mut self, icon: C) {
      self.debug = RefCell::new(Box::new(debug));
    }
  }
  ```

## use `default` meta provide a default value for field when declare it.

we can use `#[declare(default)]` to mark the field can be initialized by [`std::default::Default`]! if user not initialized it.

So update our code

 ```rust
#[derive(Declare)]
struct HeroCard {
  #[declare(convert=into)]
  name: String,
  #[declare(convert=strip_option, default)]    // edit!
  tel: Option<String>,
  #[declare(convert=strip_option, default)]    // edit!
  email: Option<String>,
}
```

Now, we can declare `HeroCard` as what we want.

```rust
let _ = widget!{
  HeroCard {
    name: "Mr Ribir",
    email: "ribir@XXX.com".to_string(),
    margin: EdgeInsets::all(8.)
  }
};
```

> Tips
>
> `#[declare(default)]` also support accept an expression as the init value instead of directly use `Default::default`. Three are two identify you can use in your expression.
> - `self` is the declarer, 
> - and `ctx` is build context of the widget,about build context see [`BuildCtx`]!.

There're two other `meta` we not used in `HeroCard` but need to know.

- #[declare(skip)] can use to skip the field that you don't want user to declare it. The field type must implemented `Default` trait or provide default expression by `default` meta.
- #[declare(rename=...)] use another name to declare field. It's useful when your field name conflict with the builtin fields. See [all builtin fields](builtin_fields).

 [declare_derive]: ../ribir/widget_derive/Declare.html
 [builtin_fields]: ../ribir/widget_derive/declare_builtin_fields.html