use crate::proxy_derive::ProxyDeriveInfo;
use crate::proxy_derive::PROXY_PATH;
use proc_macro2::TokenStream;
use quote::quote;

pub fn single_derive(input: &mut syn::DeriveInput) -> TokenStream {
  let info = ProxyDeriveInfo::new(input, "SingleChildWidget", PROXY_PATH)
    .and_then(|stt| stt.too_many_attr_specified_error());

  match info {
    Ok(info) => {
      let generics = info
        .attr_fields
        .proxy_bounds_generic(quote! {SingleChildWidget});

      let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
      let name = info.ident;
      quote! {
          impl #impl_generics SingleChildWidget for #name #ty_generics #where_clause {}
      }
    }
    Err(err) => err.into_compile_error(),
  }
}
