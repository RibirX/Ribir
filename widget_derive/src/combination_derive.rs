use crate::widget_derive::ProxyDeriveInfo;
use crate::widget_derive::PROXY_PATH;
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::parse_quote;
use syn::token::Where;

pub fn combination_derive(input: &mut syn::DeriveInput) -> TokenStream {
  let info = ProxyDeriveInfo::new(input, "CombinationWidget", PROXY_PATH)
    .and_then(|stt| stt.none_attr_specified_error())
    .and_then(|stt| stt.too_many_attr_specified_error());

  match info {
    Ok(info) => {
      let mut generics = info.generics.clone();

      let (field, _) = &info.attr_fields.attr_fields()[0];
      let proxy_ty = &field.ty;

      generics
        .where_clause
        .get_or_insert_with(|| syn::WhereClause {
          where_token: Where(Span::call_site()),
          predicates: <_>::default(),
        })
        .predicates
        .push(parse_quote! {#proxy_ty: CombinationWidget});

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
    }
    Err(err) => err,
  }
}
