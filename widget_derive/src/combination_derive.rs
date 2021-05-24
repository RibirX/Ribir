use crate::widget_derive::ProxyDeriveInfo;
use crate::{attr_fields::add_trait_bounds_if, widget_derive::PROXY_PATH};
use proc_macro2::TokenStream;
use quote::quote;
use syn::parse_quote;

pub fn combination_derive(input: &mut syn::DeriveInput) -> TokenStream {
  let info = ProxyDeriveInfo::new(input, "CombinationWidget", PROXY_PATH)
    .and_then(|stt| stt.none_attr_specified_error())
    .and_then(|stt| stt.too_many_attr_specified_error());

  match info {
    Ok(info) => {
      let generics = add_trait_bounds_if(
        info.generics.clone(),
        parse_quote!(CombinationWidget),
        |param| info.attr_fields.is_attr_generic(param),
      );
      let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
      let path = info.attr_path();
      let ident = info.ident;

      quote! {
        impl #impl_generics CombinationWidget for #ident #ty_generics #where_clause {
          #[inline]
          fn build(&self, ctx: &mut BuildCtx) -> Box<dyn Widget> {
            self.#path.build(ctx)
          }
        }
      }
    }
    Err(err) => err,
  }
}
