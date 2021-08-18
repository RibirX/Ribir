use crate::proxy_derive::ProxyDeriveInfo;
use crate::proxy_derive::PROXY_PATH;
use proc_macro2::TokenStream;
use quote::quote;

pub fn render_derive(input: &mut syn::DeriveInput) -> TokenStream {
  let info = ProxyDeriveInfo::new(input, "RenderWidget", PROXY_PATH)
    .and_then(|stt| stt.none_attr_specified_error())
    .and_then(|stt| stt.too_many_attr_specified_error());

  match info {
    Ok(info) => {
      let path = info.attr_path();
      let generics = info.attr_fields.proxy_bounds_generic(quote! {RenderWidget});
      let ident = info.ident;
      let attr_fields = info.attr_fields;
      let (field, _) = &attr_fields.attr_fields()[0];
      let proxy_ty = &field.ty;

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
          fn get_attrs(&self) -> Option<&Attributes> {
            RenderWidget::get_attrs(&self.#path)
           }
        }

        //Todo: should derive AttachAttr ?
      }
    }
    Err(err) => err,
  }
}
