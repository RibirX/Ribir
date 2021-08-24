#![feature(proc_macro_diagnostic)]
//! A derive implementation for `CombinationWidget` and `RenderWidget`. Can use
//! `proxy` attr to specify where to derive form.
//!
//! ## proxy attr.
//!
//! `#[proxy]` attr tell the widget trait where to derive from. `Widget` can
//! emit it to give a default implementation, but `CombinationWidget` or
//! `RenderWidget` must specify one and only one `proxy` attr.

//! Derive from field `b` which is a `Text`. Because `Text` is a render widget,
//!
//! ```
//! use ribir::prelude::*;
//! ##[derive(RenderWidget)]
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
//! ##[derive(RenderWidget, CombinationWidget)]
//! struct ProxyWidget<W>(#[proxy] W);
//! ```
extern crate proc_macro;
extern crate proc_macro2;

mod attr_fields;
mod combination_derive;
mod declare_derive;
mod declare_func_derive;
mod error;
mod multi_derive;
mod proxy_derive;
mod render_derive;
mod single_derive;
mod state_derive;
mod state_partial_eq_derive;
mod util;
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

#[proc_macro_derive(Declare, attributes(rename))]
pub fn declare_trait_macro_derive(input: TokenStream) -> TokenStream {
  let mut input = parse_macro_input!(input as DeriveInput);
  declare_derive::declare_derive(&mut input)
    .unwrap_or_else(|e| e)
    .into()
}

#[proc_macro]
pub fn declare(input: TokenStream) -> TokenStream {
  declare_func_derive::declare_func_macro(input).into()
}
