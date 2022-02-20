//! Widget can be supported in `declare!` across implement [`Declare`]!
//! [`DeclareBuilder`]!
//!
//! We provide a derive macro and recommend you use it, you needn't implement it
//! by your self. If you want to implement by yourself, see the [macro
//! document](declare_derive) to know how it work.
//!
//! # Example
//!
//! Assume we want to implement a people card widget with name, email and
//! telephone. And only the name is required.
//!
//! ```rust
//! #![feature(trivial_bounds)]
//!
//! use ribir::prelude::*;
//!
//! #[derive(Declare, Default)]
//! struct PeopleCard {
//!   name: String,
//!   email: Option<String>,
//!   tel: Option<String>,
//! }
//!
//! impl CombinationWidget for PeopleCard {
//!   fn build(&self, ctx: &mut BuildCtx) -> BoxedWidget{
//!     let text_style = ctx.theme().typography_theme.body1.text.clone();
//!     declare! {
//!       Column {
//!          Text {
//!            text: self.name.clone(),
//!            style: text_style.clone()
//!          }
//!          self.email.clone().map(|email| Text {
//!            text: email.into(),
//!            style: text_style.clone()
//!          })
//!          self.tel.clone().map(|tel| Text {
//!             text: tel.into(),
//!             style: text_style.clone()
//!          })
//!       }
//!     }
//!   }
//! }
//!
//! let p = declare!{
//!   PeopleCard {
//!     name: "Mr Ribir".to_string(),
//!     email: Some("ribir@XXX.com".to_string()),
//!     margin: EdgeInsets::all(8.),
//!     ..<_>::default()
//!   }
//! };
//! ```
//!
//! ## #[declare(...)] attr to rename or provide converter for field.
//!
//! After derive `Declare`, the `PeopleCard` is work fine in `declare!`. But
//! init the email with `email: Some("ribir@XXX.com".to_string())` is too
//! verbose, if we init the `email`, is must be some, so we not want write the
//! `Some(XXX)` wrap, and we also want user can init `email` with both `&str`
//! and `String`. `convert` meta is use to finish this job.
//!
//! ```rust
//! # #![feature(trivial_bounds)]
//! # use ribir::prelude::*;
//!
//! #[derive(Declare, Default)]
//! struct PeopleCard {
//!   #[declare(convert(into))]
//!   name: String,
//!   #[declare(convert(into, some))]
//!   email: Option<String>,
//!   #[declare(convert(into, some))]
//!   tel: Option<String>,
//! }
//! ```
//!
//! The argument `into` of `convert` is means the init value need call
//! `Into::into` before init the field, and `some` means this is a `Option`
//! field and will init the filed with its inner Some-Value without a
//! `Some(XXX)` wrap. The `convert` only support these two arguments now. Now,
//! we can declare a `PeopleCard` in `declare!` more elegant:
//!
//! ```rust ignore
//! let p = declare!{
//!   PeopleCard {
//!     name: "Mr Ribir",
//!     email: "ribir@XXX.com",
//!     ..<_>::default()
//!   }
//! };
//! ```
//!
//! `declare` attr also support `rename` meta, it use to if you want rename the
//! field int the `declare!` marco. `#[declare(rename="XXX")]` means you want
//! the field across `XXX` to init in the `declare!`.
//!
//! [declare_derive]: ../ribir/widget_derive/Declare.html

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
