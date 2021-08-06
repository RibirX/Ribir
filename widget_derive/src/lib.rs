//! A custom derive implementation for `Widget`, `CombinationWidget` and
//! `RenderWidget`. Can use `proxy` attr to specify where to derive form.
//!
//! # Example
//! A default widget implementation for W.

//! ```
//! use ribir::prelude::*;
//！##[derive(Widget)]
//! struct W;
//! ```
//! ## proxy attr.
//!
//! `#[proxy]` attr tell the widget trait where to derive from. `Widget` can
//! emit it to give a default implementation, but `CombinationWidget` or
//! `RenderWidget` must specify one and only one `proxy` attr.

//! Derive from field `b` which is a `Text`. Because `Text` is a render widget,
//!
//! ```
//! use ribir::prelude::*;
//! ##[derive(Widget, RenderWidget)]
//! struct W {
//!  ##[proxy]
//!  b: widget::Text
//! }
//! ```
//！
//! Derive from a generic type, and derive `RenderWidget` if it's a render
//! widget, derive `CombinationWidget` if it's a combination widget.
//! ```
//! use ribir::prelude::*;
//! ##[derive(Widget, RenderWidget, CombinationWidget)]
//! struct ProxyWidget<W>(#[proxy] W);
//! ```
extern crate proc_macro;
extern crate proc_macro2;

mod attach_attr_derive;
mod attr_fields;
mod combination_derive;
mod multi_derive;
mod render_derive;
mod single_derive;
mod state_derive;
mod state_partial_eq_derive;
mod proxy_derive;

use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(CombinationWidget, attributes(proxy))]
pub fn combination_macro_derive(input: TokenStream) -> TokenStream {
  let mut input = parse_macro_input!(input as DeriveInput);
  combination_derive::combination_derive(&mut input).into()
}

#[proc_macro_derive(RenderWidget, attributes(proxy))]
pub fn render_macro_derive(input: TokenStream) -> TokenStream {
  let mut input = parse_macro_input!(input as DeriveInput);
  render_derive::render_derive(&mut input).into()
}

#[proc_macro_derive(SingleChildWidget, attributes(proxy))]
pub fn single_marco_derive(input: TokenStream) -> TokenStream {
  let mut input = parse_macro_input!(input as DeriveInput);
  single_derive::single_derive(&mut input).into()
}

#[proc_macro_derive(MultiChildWidget, attributes(proxy))]
pub fn multi_macro_derive(input: TokenStream) -> TokenStream {
  let mut input = parse_macro_input!(input as DeriveInput);
  multi_derive::multi_derive(&mut input).into()
}

// Todo Should give a default implement in attr mod.  Depends on https://github.com/rust-lang/rust/pull/85499 fixed.
#[proc_macro_derive(AttachAttr)]
pub fn attach_attr_macro_derive(input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as DeriveInput);
  attach_attr_derive::attach_attr_derive(&input).into()
}

#[proc_macro_derive(StatePartialEq)]
pub fn state_partial_eq_macro_derive(input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as DeriveInput);
  state_partial_eq_derive::derive_state_partial_eq(&input).into()
}

#[proc_macro_attribute]
pub fn stateful(attrs: TokenStream, input: TokenStream) -> TokenStream {
  let attrs = parse_macro_input!(attrs as syn::AttributeArgs);
  let mut input = parse_macro_input!(input as DeriveInput);
  state_derive::stateful_derive(&mut input, attrs)
    .unwrap_or_else(|e| e)
    .into()
}
