use crate::widget_derive::ProxyDeriveInfo;
use crate::{attr_fields::add_trait_bounds_if, widget_derive::PROXY_PATH};
use proc_macro2::TokenStream;
use quote::quote;
use syn::parse_quote;

pub fn render_derive(input: &syn::DeriveInput) -> TokenStream {
  let info = ProxyDeriveInfo::new(input, "RenderWidget", PROXY_PATH)
    .and_then(|stt| stt.none_proxy_specified_error())
    .and_then(|stt| stt.too_many_proxy_specified_error());

  match info {
    Ok(info) => {
      let path = info.proxy_path();
      let ident = info.ident;
      let attr_fields = info.attr_fields;

      let generics =
        add_trait_bounds_if(info.generics.clone(), parse_quote!(RenderWidget), |param| {
          attr_fields.is_attr_generic(param)
        });
      let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

      let (field, _) = &attr_fields.attr_fields()[0];
      let proxy_ty = &field.ty;

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
