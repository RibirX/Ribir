//! Widget can be supported in `widget!` across implement [`Declare`]!
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
//!   #[widget]
//!   fn build(&self, ctx: &mut BuildCtx) -> BoxedWidget {
//!     widget!{
//!       PeopleCard {
//!         name: "Mr Ribir",
//!         email: "ribir@XXX.com".to_string(),
//!         tel: None,
//!         margin: EdgeInsets::all(8.)
//!       }
//!     }
//!   }
//! }
//! ```
//!
//! Notice we use `&str` to initialize `name` field and must use `String` to
//! initialize `email`, that because its field have different type and ribir
//! `implicit` insert an `into()` for field initialization. So we can use `&str`
//! to initialize `String`, not to `Option<String>`.
//!
//! ## use `strip_option` meta for `Option<T>` fields
//!
//! After derive `Declare`, the `PeopleCard` is work fine in `widget!`. But
//! init the email with `email: "ribir@XXX.com".to_string()` is too verbose, if
//! we try to init the `email`here, that implicitly mean we want a `Some-Value`.
//! We want use the type `T` instead of `Option<T>` in `widget!`, `String`
//! instead of `str` here, `strip_option` it's designed for this case.

//!
//! ```rust
//! # use ribir::prelude::*;
//!
//! #[derive(Declare, Default)]
//! struct PeopleCard {
//!   name: String,
//!   #[declare(strip_option)]  // new!
//!   email: Option<String>,
//!   tel: Option<String>,
//! }
//!
//! # impl CombinationWidget for PeopleCard {
//! #  fn build(&self, ctx: &mut BuildCtx) -> BoxedWidget{
//! #    unreachable!("We don't care how implement `PersonCard` here, but force on how to use it.")
//! #  }
//! # }
//!
//! // Now, we can declare a `PeopleCard` in `widget!` more elegant:
//!
//! struct UsePerson;
//!
//! impl CombinationWidget for UsePerson {
//!   #[widget]
//!   fn build(&self, ctx: &mut BuildCtx) -> BoxedWidget {
//!     widget!{
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
//! ## `default` meta make the field optional in `widget!` macro, and Ribir use
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
//!   name: String,
//!   #[declare(strip_option)]  // new!
//!   email: Option<String>,
//!   #[declare(strip_option, default)]  // new!
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
//!   #[widget]
//!   fn build(&self, ctx: &mut BuildCtx) -> BoxedWidget {
//!     widget!{
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
//! when you want user can init it in `widget!`. Sell [all builtin
//! fields](builtin_fields).
//!
//! [declare_derive]: ../ribir/widget_derive/Declare.html
//! [builtin_fields]: ../ribir/widget_derive/declare_builtin_fields.html

use crate::prelude::BuildCtx;

/// Trait to mark the builder type of widget. `widget!` use it to access the
/// build type of the widget. See the [mod level document](declare) to know how
/// to use it.
pub trait Declare {
  type Builder: DeclareBuilder;
  fn builder() -> Self::Builder;
}

/// widget builder use to construct a widget in  `widget!`. See the [mod level
/// document](declare) to know how to use it.
pub trait DeclareBuilder {
  type Target;
  fn build(self, ctx: &mut BuildCtx) -> Self::Target;
}

pub trait Striped<Marker, V> {
  fn striped(self) -> Option<V>;
}

pub struct OptionInnerMarker;
pub struct OptionMarker;

impl<T: Into<V>, V> Striped<OptionInnerMarker, V> for T {
  #[inline]
  fn striped(self) -> Option<V> { Some(self.into()) }
}

impl<V> Striped<OptionMarker, V> for Option<V> {
  #[inline]
  fn striped(self) -> Option<V> { self }
}

#[cfg(test)]
mod tests {
  use super::*;
  use painter::{Brush, Color};

  #[test]
  fn inner_value_into() {
    assert_eq!(
      painter::Color::RED.striped(),
      Some(Brush::Color(Color::RED))
    )
  }

  #[test]
  fn option_self_can_use_with_stripe() {
    assert_eq!(
      Some(Brush::Color(Color::RED)).striped(),
      Some(Brush::Color(Color::RED))
    )
  }
}
