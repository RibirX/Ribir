use crate::widget_derive::ProxyDeriveInfo;
use crate::widget_derive::PROXY_PATH;
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::parse_quote;
use syn::token::Where;

pub fn render_derive(input: &mut syn::DeriveInput) -> TokenStream {
  let info = ProxyDeriveInfo::new(input, "RenderWidget", PROXY_PATH)
    .and_then(|stt| stt.none_attr_specified_error())
    .and_then(|stt| stt.too_many_attr_specified_error());

  match info {
    Ok(info) => {
      let path = info.attr_path();
      let ident = info.ident;
      let attr_fields = info.attr_fields;

      let (field, _) = &attr_fields.attr_fields()[0];
      let proxy_ty = &field.ty;
      let mut generics = info.generics.clone();
      generics
        .where_clause
        .get_or_insert_with(|| syn::WhereClause {
          where_token: Where(Span::call_site()),
          predicates: <_>::default(),
        })
        .predicates
        .push(parse_quote! {#proxy_ty: RenderWidget});

      let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

      quote! {
        impl #impl_generics CloneStates for #ident #ty_generics #where_clause {
          type States = <#proxy_ty as CloneStates>::States;

          #[inline]
          fn clone_states(&self) -> Self::States { self.#path.clone_states() }
        }

        impl #impl_generics RenderWidget for #ident #ty_generics #where_clause {
          type RO = <#proxy_ty as RenderWidget>::RO;

          #[inline]
          fn create_render_object(&self) -> Self::RO {
            RenderWidget::create_render_object(&self.#path)
          }

          #[inline]
          fn take_children(&mut self) -> Option<SmallVec<[Box<dyn Widget>; 1]>> {
            RenderWidget::take_children(&mut self.#path)
           }
        }
      }
    }
    Err(err) => err,
  }
}
