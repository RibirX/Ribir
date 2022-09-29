#![feature(proc_macro_diagnostic, proc_macro_span)]
extern crate proc_macro;
extern crate proc_macro2;

mod declare_derive;
mod error;
mod widget_attr_macro;

mod lerp_derive;
mod util;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};
use widget_attr_macro::{DeclareCtx, WidgetMacro};
use widget_try_track_macro::TryTrack;
mod widget_try_track_macro;

pub(crate) const WIDGET_MACRO_NAME: &str = "widget";

#[proc_macro_derive(SingleChild)]
pub fn single_marco_derive(input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as DeriveInput);
  let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
  let name = input.ident;
  quote! {
      impl #impl_generics SingleChild for #name #ty_generics #where_clause {}
  }
  .into()
}

#[proc_macro_derive(MultiChild)]
pub fn multi_macro_derive(input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as DeriveInput);
  let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
  let name = input.ident;
  quote! {
      impl #impl_generics MultiChild for #name #ty_generics #where_clause {}
  }
  .into()
}

#[proc_macro_derive(Lerp)]
pub fn lerp_derive(input: TokenStream) -> TokenStream {
  let mut input = parse_macro_input!(input as DeriveInput);
  lerp_derive::lerp_derive(&mut input)
    .unwrap_or_else(|e| e.into_compile_error())
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

#[proc_macro]
pub fn widget(input: TokenStream) -> TokenStream {
  let mut w = parse_macro_input! { input as WidgetMacro };
  let mut ctx = DeclareCtx::default();
  w.gen_tokens(&mut ctx).into()
}

#[proc_macro]
pub fn widget_try_track(input: TokenStream) -> TokenStream {
  let try_track = parse_macro_input! { input as TryTrack };
  quote! { #try_track}.into()
}

#[proc_macro]
pub fn include_svg(input: TokenStream) -> TokenStream {
  let w = parse_macro_input! { input as syn::LitStr };
  let span = proc_macro::Span::call_site();
  let mut file = span.source_file().path();
  file.pop();
  file.push(w.value());
  let encoded_bytes = painter::SvgRender::open(file).and_then(|reader| reader.serialize());
  match encoded_bytes {
    Ok(data) => quote! {
      SvgRender::deserialize(#data).unwrap()
    }
    .into(),
    Err(err) => {
      let err = format!("{err}");
      quote! { compile_error!(#err)}.into()
    }
  }
}
