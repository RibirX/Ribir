//! A custom derive implemention for `Widget` and `CombinationWidget`
//! `RenderWidget`. Can use `proxy` attr to specify where to derive form.
//!
//! ## `proxy` attr.
//! To derive `CombinationWidget` or `RenderWidget` must specify one and only
//! `proxy` attr, `Widget` can emit the it to give a default implementation.
//！
//! A default widget implementation for W.
//！```
//！#[derive(Widget)]
//! struct W;
//! ```
//！
//! Derive from field b, like a checkbox. Because `Checkbox` is a
//! render widget, also derive the `RenderWidget`
//!
//! ```
//！#[derive(Widget, RenderWidget)]
//! struct W {
//!  #[proxy]
//!  b: Checkbox
//! }
//! ```
//！
//! Derive from a generic type, and derive `RenderWidget` if it's a render
//! widget, derive `CombinationWidget`.
//！```
//! #[derive(Widget, RenderWidget, CombinationWidget)]
//! struct ProxyWidget<W>(#[proxy] W);
//!
//! Use meta `ref` and `ref_mut` to give a method name to specify how to borrow
//! the reference from the field.
//!
//! #[derive(Widget, RenderWidget)]
//! struct W {
//!  #[proxy(ref=borrow, ref_mut=borrow_mut)]
//!  b: Checkbox
//! }

extern crate proc_macro;
extern crate proc_macro2;

mod attr_fields;
mod combination_derive;
mod render_derive;
mod widget_derive;

mod state;
use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(Widget, attributes(proxy))]
pub fn widget_macro_derive(input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as DeriveInput);

  widget_derive::widget_derive(&input).into()
}

#[proc_macro_derive(CombinationWidget, attributes(proxy))]
pub fn combination_macro_derive(input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as DeriveInput);

  combination_derive::combination_derive(&input).into()
}

#[proc_macro_derive(RenderWidget, attributes(proxy))]
pub fn render_macro_derive(input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as DeriveInput);

  render_derive::render_derive(&input).into()
}

#[proc_macro_derive(Stateful, attributes(state))]
pub fn stateful_derive(input: TokenStream) -> TokenStream { unimplemented!() }
