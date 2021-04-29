extern crate proc_macro;
extern crate proc_macro2;

mod attr_fields;
mod combination_derive;
mod render_derive;

mod state;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(Widget, attributes(state))]
pub fn widget_macro_derive(input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as DeriveInput);
  let state_impl = state::state_gen(&input);

  let name = input.ident;
  let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

  let expanded = quote! {
      // The generated `Widget` impl.
      impl #impl_generics Widget for #name #ty_generics #where_clause {
        #[inline]
        fn attrs_ref(&self) -> Option<&AttrsRef>{ None }

        #[inline]
        fn attrs_mut(&self) -> Option<&AttrsMut> { None }
      }

      impl #impl_generics AttachAttr for #name #ty_generics #where_clause {
        type HostWidget = Self;
      }

      #state_impl
  };

  expanded.into()
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
