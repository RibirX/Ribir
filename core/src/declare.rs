//! Widget can be supported in `declare!` across implement [`Declare`]!
//! [`DeclareBuilder`]!
//!
//! We provide a derive macro and recommend you use it, you needn't implement it
//! by your self. If you want to implement by yourself, see the [macro
//! document](declare_derive) to know how it work.
//!
//!
//! Assume we want to implement a people card widget with name, email and
//! telephone. And only the name is required.
//!
//! ```rust
//! 
//! use ribir::prelude::*;
//!
//! #[derive(Declare)]
//! struct PeopleCard {
//!   name: String,
//!   email: Option<String>,
//!   tel: Option<String>,
//! }
//!
//! impl CombinationWidget for PeopleCard {
//!   fn build(&self, ctx: &mut BuildCtx) -> BoxedWidget{
//!     unreachable!("We don't care how implement `PersonCard` here, but force on how to use it.")
//!   }
//! }
//!
//! // Person widget can be declared in other widget now, and builtin field like
//! // `margin` can be used as its field.
//!
//! struct UsePerson;
//!
//! impl CombinationWidget for UsePerson {
//!   fn build(&self, ctx: &mut BuildCtx) -> BoxedWidget {
//!     declare!{
//!       PeopleCard {
//!         name: "Mr Ribir".to_string(),
//!         email: Some("ribir@XXX.com".to_string()),
//!         tel: None,
//!         margin: EdgeInsets::all(8.)
//!       }
//!     }
//!   }
//! }
//! ```
//!
//! ## `setter(...)` meta settings for the field setters, this tell Ribir how
//! usr can init field by other types in `declare` macro.
//!
//! - `into`: tell Ribir accept any type that satisfy the `std::convert::Into`
//!   trait bounds and can convert to the field type  to init the field.
//! - `strip_option`: for Option<T> fields only, this makes user can use the
//!   inner type `T` to init the field.
//! - use `into` and `strip_option` both: for Option<T> fields only, accept the
//!   init value that can convert to the inner type `T`
//!
//! After derive `Declare`, the `PeopleCard` is work fine in `declare!`. But
//! init the email with `email: Some("ribir@XXX.com".to_string())` is too
//! verbose, if we init the `email`, is must be a `Some-Value`, so we not want
//! write the `Some(XXX)` wrap, and we also want user can init `email` with both
//! `&str` and `String`. `setter` is what we want.
//!
//! ```rust
//! # use ribir::prelude::*;
//!
//! #[derive(Declare, Default)]
//! struct PeopleCard {
//!   #[declare(setter(into))]                // new!
//!   name: String,
//!   #[declare(setter(into, strip_option))]  // new!
//!   email: Option<String>,
//!   #[declare(setter(into))]                // new!
//!   tel: Option<String>,
//! }
//!
//! # impl CombinationWidget for PeopleCard {
//! #  fn build(&self, ctx: &mut BuildCtx) -> BoxedWidget{
//! #    unreachable!("We don't care how implement `PersonCard` here, but force on how to use it.")
//! #  }
//! # }
//!
//! // Now, we can declare a `PeopleCard` in `declare!` more elegant:
//!
//! struct UsePerson;
//!
//! impl CombinationWidget for UsePerson {
//!   fn build(&self, ctx: &mut BuildCtx) -> BoxedWidget {
//!     declare!{
//!       PeopleCard {
//!         name: "Mr Ribir",
//!         email: "ribir@XXX.com",
//!         tel: None,
//!         margin: EdgeInsets::all(8.)
//!       }
//!     }
//!   }
//! }
//! ```
//!
//! ## `default` meta make the field optional in `declare!` macro, and Ribir use
//! its 'default' value to init it. The 'default' value can be:
//!
//! - `default` without init expression, the field type must implemented
//!   [`std::default::Default`]!, and the default value of the field type use to
//!   init the field.
//!
//! - `default="expr"`: `expr` use to init the field if its init value is
//!   omitted by user, Note expression must be inclosed in quotes. Use `\` to
//!   escaped if you contain a string in your expression, or raw string. Three
//!   are two identify you can use in your expression to access context, `self`
//!   is the Builder type of the widget, and `ctx` is build context of the
//!   widget you can use, about build context see [`BuildCtx`]!. `ctx` is useful
//!   when you want provide a default value for your 'style' fields from theme.
//!
//! We update the `PeopleCard`,
//! ```rust
//! # use ribir::prelude::*;
//!
//! #[derive(Declare, Default)]
//! struct PeopleCard {
//!   #[declare(setter(into))]
//!   name: String,
//!   #[declare(setter(into, strip_option), default)]  // new!
//!   email: Option<String>,
//!   #[declare(setter(into, strip_option), default)]  // new!
//!   tel: Option<String>,
//! }
//!
//! # impl CombinationWidget for PeopleCard {
//! #  fn build(&self, ctx: &mut BuildCtx) -> BoxedWidget{
//! #    unreachable!("We force on how to use and don't care how to implement `PeopleCard` here")
//! #  }
//! # }
//!
//! // we can omit `tel: None` now.
//!
//! struct UsePerson;
//!
//! impl CombinationWidget for UsePerson {
//!   fn build(&self, ctx: &mut BuildCtx) -> BoxedWidget {
//!     declare!{
//!       PeopleCard {
//!         name: "Mr Ribir",
//!         email: "ribir@XXX.com",
//!         margin: EdgeInsets::all(8.)
//!       }
//!     }
//!   }
//! }
//! ```
//!
//! ## `rename="xxx"` meta, use an other field name to init the field.
//!
//! Field with attribute `#[declare(rename="xxx")]` tell Ribir that `xxx` will
//! as the field name when init the widget in `declare`. If your field name
//! conflict with the builtin field names, you must use `rename` to avoid it,
//! when you want user can init it in `declare!`. Sell [all builtin
//! fields](builtin_fields).
//!
//! [declare_derive]: ../ribir/widget_derive/Declare.html
//! [builtin_fields]: ../ribir/widget_derive/declare_builtin_fields.html

use crate::prelude::BuildCtx;

/// Trait to mark the builder type of widget. `declare!` use it to access the
/// build type of the widget. See the [mod level document](declare) to know how
/// to use it.
pub trait Declare {
  type Builder: DeclareBuilder;
  fn builder() -> Self::Builder;
}

/// widget builder use to construct a widget in  `declare!`. See the [mod level
/// document](declare) to know how to use it.
pub trait DeclareBuilder {
  type Target;
  fn build(self, ctx: &mut BuildCtx) -> Self::Target;
}
