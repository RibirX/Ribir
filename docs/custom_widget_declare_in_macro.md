# Custom widget declare in macro.

You can declare any widget in `widget!` that implements the `Declare` trait. And Ribir provides a derive macro that makes implementing the `Declare` trait super easy.

> Some tips
> 
> Almost all widgets provided by `Ribir` implement `Declare` via derive. We shouldn't implement it ourselves unless we have a special reason. You can derive it directly. If you want to implement it yourself, read the [derive macro document](declare_derive) to see how it works.

In this chapter, we'll learn how to use the meta of the `#[declare(...)]` attribute on the field. And it will help us improve the declaration user experience on our widget.

Let's create a hero card widget. It contains a name, email, and phone. And the name is required, email and phone are optional.

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
After `HeroCard` derives `Declare`, it can be declared in `widget!` and the built-in fields can be used in it. Looks good, but not enough. The declaration of `HeroCard` is too lengthy.

- Can we initialise `name` directly with `&str`? Because `&str` can be converted to `String`.
- `tel` is initialised with None. Can we omit it?
- Can we remove the `Some` from `email`? When we use a value to initialise an option type, it implicitly implies that it's a `Some-value`.

Of course we can use the meta of `#[declare(...)]`. Let's introduce them one by one.

## Use `convert` meta to convert value from one type to another.

Let's start with the problems of `name` accepting `&str` and `tel` removing the `Some` wrap. Both are type conversion problems.

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
We just add `#[declare(convert=into)]` or `#[declare(convert=strip_option)]` for the fields. That is all. Besides `into` and `strip_option` there are two other values that can be used in `#[declare(convert=...)]`. Let's look at them one by one.

- `#[declare(convert=into)]`, use `std::convert::Into` to convert the value before initialising the field. With this meta, any type that implements an `Into` trait on the field type can be used to initialise the field. 
- `#[declare(convert=strip_option)]`, wrap `V` to `Some(V)` before initialising `Option<V>`, of course `Option<V>` is also accepted.
- `#[declare(convert=custom)]`, implement the filed build by itself, implement the same name method for its declarer to accept the initialize type. Then implement a `set_declare_xxx` method for its host type so that it can be updated with the type you want.  For example:
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

## Use the `default` meta to provide a default value for a field when you declare it.

We can use `#[declare(default)]` to indicate that the field can be initialised by [`std::default::Default`]! if the user does not initialise it.

Let's update our code

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

> Some tips
>
> `#[declare(default)]` also supports accepting an expression as an init value instead of using `Default::Default` directly. There are two identifiers you can use in your expression.
> - `self` is the declarer, 
> - and `ctx` is the build context of the widget. For the build context see [`BuildCtx`]!.

There are two other `meta` that we don't use in `HeroCard`, but need to know about.

- We can use `#[declare(skip)]` to skip the field that we don't want the user to declare. The field type must implement the `Default` trait or provide a default expression through the `default` meta.
- We can use `#[declare(rename=...)]` to rename a field. It's useful if our field name conflicts with the built-in fields. See [all built-in fields](builtin_fields).

 [declare_derive]: ../ribir/widget_derive/Declare.html
 [builtin_fields]: ../ribir/widget_derive/declare_builtin_fields.html
