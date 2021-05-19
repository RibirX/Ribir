use crate::attr_fields::add_trait_bounds_if;
use crate::widget_derive::ProxyDeriveInfo;
use proc_macro2::TokenStream;
use quote::quote;
use syn::parse_quote;

pub fn combination_derive(input: &syn::DeriveInput) -> TokenStream {
  let info = ProxyDeriveInfo::new(input, "CombinationWidget")
    .and_then(|stt| stt.none_proxy_specified_error())
    .and_then(|stt| stt.too_many_proxy_specified_error());

  match info {
    Ok(info) => {
      let generics = add_trait_bounds_if(
        info.generics.clone(),
        parse_quote!(CombinationWidget),
        |param| info.attr_fields.is_attr_generic(param),
      );
      let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
      let path = info.proxy_path();
      let ident = info.ident;

      quote! {
        impl #impl_generics CombinationWidget for #ident #ty_generics #where_clause {
          #[inline]
          fn build(&self, ctx: &mut BuildCtx) -> BoxWidget {
            self.#path.build(ctx)
          }
        }
      }
    }
    Err(err) => err,
  }
}
