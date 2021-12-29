use crate::proxy_derive::ProxyDeriveInfo;
use crate::proxy_derive::PROXY_PATH;
use crate::util::add_where_bounds;
use proc_macro2::TokenStream;
use quote::quote;

pub fn combination_derive(input: &mut syn::DeriveInput) -> TokenStream {
  ProxyDeriveInfo::new(input, "CombinationWidget", PROXY_PATH)
    .and_then(|stt| stt.none_attr_specified_error())
    .and_then(|stt| stt.too_many_attr_specified_error())
    .map_or_else(
      |err| err.into_compile_error(),
      |info| {
        let mut generics = info
          .attr_fields
          .proxy_bounds_generic(quote! {CombinationWidget});
        add_where_bounds(&mut generics, quote! { Self: AttrsAccess});

        let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
        let path = info.attr_path();
        let ident = info.ident;

        quote! {
          impl #impl_generics CombinationWidget for #ident #ty_generics #where_clause {
            #[inline]
            fn build(&self, ctx: &mut BuildCtx) -> BoxedWidget {
              self.#path.build(ctx)
            }
          }
        }
      },
    )
}
