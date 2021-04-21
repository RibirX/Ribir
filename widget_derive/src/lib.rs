extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(Widget)]
pub fn hello_macro_derive(input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as DeriveInput);
  let name = input.ident;
  let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

  let expanded = quote! {
      // The generated `Widget` impl.
      impl #impl_generics Widget for #name #ty_generics #where_clause {
      }

      // Todo: this can auto implement like below
      // ```ignore
      //  impl<T: Widget> AttributeAttach for T {
      //    default type HostWidget = Self;
      //  }
      // ```
      // But specialization not finished, and can not infer the associated type.
      impl #impl_generics AttributeAttach for #name #ty_generics #where_clause {
        type HostWidget = Self;
      }

  };

  expanded.into()
}
