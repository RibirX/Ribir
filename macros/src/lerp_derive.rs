use proc_macro2::TokenStream;
use quote::quote;
use syn::spanned::Spanned;

use crate::util::data_struct_unwrap;

pub(crate) fn lerp_derive(input: &mut syn::DeriveInput) -> syn::Result<TokenStream> {
  let syn::DeriveInput { ident: name, generics, data, .. } = input;
  let (g_impl, g_ty, g_where) = generics.split_for_impl();
  let stt = data_struct_unwrap(data, "Lerp")?;

  let tokens = match &stt.fields {
    syn::Fields::Named(n) => {
      let fields = n.named.iter().map(|f| &f.ident);
      quote! {
        impl #g_impl Lerp for #name #g_ty #g_impl #g_where {
          fn lerp(&self, to: &Self, factor: f32) -> Self {
            #name {
              #(#fields: self.#fields.lerp(&to.#fields, factor)),*
            }
          }
        }
      }
    }
    syn::Fields::Unnamed(u) => {
      let indexes = u
        .unnamed
        .iter()
        .enumerate()
        .map(|(i, f)| syn::Index { index: i as u32, span: f.span() });
      quote! {
        impl #g_impl Lerp for #name #g_ty #g_impl #g_where {
          fn lerp(&self, to: &Self, factor: f32) -> Self {
            #name( #(self.#indexes.lerp(&to.#indexes, factor)) ,*)
          }
        }
      }
    }
    syn::Fields::Unit => quote! {
      impl #g_impl Lerp for #name #g_ty #g_impl #g_where {
        fn lerp(&self, to: &Self, factor: f32) -> Self {
           #name
        }
      }
    },
  };

  Ok(tokens)
}
