use crate::proxy_derive::ProxyDeriveInfo;
use crate::proxy_derive::PROXY_PATH;
use proc_macro2::TokenStream;
use quote::quote;

pub fn combination_derive(input: &mut syn::DeriveInput) -> TokenStream {
  let info = ProxyDeriveInfo::new(input, "CombinationWidget", PROXY_PATH)
    .and_then(|stt| stt.none_attr_specified_error())
    .and_then(|stt| stt.too_many_attr_specified_error());

  match info {
    Ok(info) => {
      let generics = info
        .attr_fields
        .proxy_bounds_generic(quote! {CombinationWidget});

      let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
      let path = info.attr_path();
      let ident = info.ident;

      quote! {
        impl #impl_generics CombinationWidget for #ident #ty_generics #where_clause {
          #[inline]
          fn build(&self, ctx: &mut BuildCtx) -> BoxedWidget {
            self.#path.build(ctx)
          }

          #[inline]
          fn get_attrs(&self) -> Option<&Attributes> {
            CombinationWidget::get_attrs(&self.#path)
           }
        }
      }
    }
    Err(err) => err,
  }
}
