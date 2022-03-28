#![feature(proc_macro_diagnostic)]
extern crate proc_macro;
extern crate proc_macro2;

mod declare_derive;
mod declare_func_derive;
mod error;

mod util;

use declare_func_derive::DeclareCtx;
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{quote, quote_spanned};
use syn::{
  parse_macro_input, punctuated::Punctuated, spanned::Spanned, token::Comma, visit_mut::VisitMut,
  DeriveInput, FnArg, Ident,
};
pub(crate) const WIDGET_MACRO_NAME: &str = "widget";

#[proc_macro_derive(SingleChildWidget, attributes(proxy))]
pub fn single_marco_derive(input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as DeriveInput);
  let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
  let name = input.ident;
  quote! {
      impl #impl_generics SingleChildWidget for #name #ty_generics #where_clause {}
  }
  .into()
}

#[proc_macro_derive(MultiChildWidget, attributes(proxy))]
pub fn multi_macro_derive(input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as DeriveInput);
  let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
  let name = input.ident;
  quote! {
      impl #impl_generics MultiChildWidget for #name #ty_generics #where_clause {}
  }
  .into()
}

/// Macro to implement the `Declare` trait. To know how to use it see the
/// [`declare` mod document](declare)
///
/// This macro implement a `XXXBuilder` struct with same field type for `XXX`
/// widget, then
///
/// - implement `Declare` for `XXX`  mark `XXXBuilder` as its builder type.
/// - implement `DeclareBuilder` for `XXXBuilder` which build `XXX` and used by
///   `declare！` to build the `XXX` widget.
/// - for every field of `XXXBuilder`
///   - implement an associate method `into_xxx`   use to convert a value to the
///     `xxx` field type, which effect by the   `convert` meta. `widget!` will
///     use it to convert the field value
///   - implement method with same name of the field and use to init the field.
///
///  [declare]: ../ribir/declare/index.html
#[proc_macro_derive(Declare, attributes(declare))]
pub fn declare_trait_macro_derive(input: TokenStream) -> TokenStream {
  let mut input = parse_macro_input!(input as DeriveInput);
  declare_derive::declare_derive(&mut input)
    .unwrap_or_else(|e| e.into_compile_error())
    .into()
}

#[doc = include_str!("../../docs/declare_macro.md")]
#[doc = include_str!("../../docs/declare_builtin_fields.md")]
#[proc_macro_attribute]
pub fn widget(_attr: TokenStream, input: TokenStream) -> TokenStream {
  let mut compose_fn = parse_macro_input! { input as syn::ImplItemMethod };
  let inputs = &compose_fn.sig.inputs;

  fn this_and_ctx_args(inputs: &Punctuated<FnArg, Comma>) -> Result<(Ident, Ident), TokenStream> {
    if inputs.len() != 2 {
      let compile_err = format!(
        "`#[{WIDGET_MACRO_NAME}]` expected 2 parameters, found {}",
        inputs.len()
      );
      return Err(
        quote_spanned! {
          inputs.span() =>compile_error!(stringify!(#compile_err));
        }
        .into(),
      );
    }

    fn unknown_arguments_err(span: Span) -> Result<(Ident, Ident), TokenStream> {
      Err(quote_spanned! { span => compile_err!("unknown arguments name") }.into())
    }

    let self_name = match inputs.first().unwrap() {
      FnArg::Receiver(r) => syn::Ident::new("self", r.self_token.span),
      FnArg::Typed(t) => match *t.pat {
        syn::Pat::Ident(ref name) => name.ident.clone(),
        _ => return unknown_arguments_err(t.span()),
      },
    };

    let ctx_name = match inputs.last().unwrap() {
      FnArg::Receiver(r) => return unknown_arguments_err(r.span()),
      FnArg::Typed(t) => match *t.pat {
        syn::Pat::Ident(ref name) => name.ident.clone(),
        _ => return unknown_arguments_err(t.span()),
      },
    };

    Ok((self_name, ctx_name))
  }

  let (self_name, ctx_name) = match this_and_ctx_args(inputs) {
    Ok(names) => names,
    Err(err) => return err,
  };

  let mut ctx = DeclareCtx::new(self_name, ctx_name);

  ctx.stack_push().visit_impl_item_method_mut(&mut compose_fn);
  quote! { #compose_fn }.into()
}
