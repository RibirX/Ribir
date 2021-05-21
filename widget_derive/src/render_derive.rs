use crate::widget_derive::ProxyDeriveInfo;
use crate::{attr_fields::add_trait_bounds_if, widget_derive::PROXY_PATH};
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{parse_quote, Ident};

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

      let render_name = Ident::new(&format!("{}Render", ident), Span::call_site());

      let proxy_generics = add_trait_bounds_if(
        attr_fields.attr_fields_generics(),
        parse_quote!(RenderWidget),
        |param| attr_fields.is_attr_generic(param),
      );
      let (proxy_impl_generics, proxy_type_generics, proxy_where_clause) =
        proxy_generics.split_for_impl();
      let vis = &input.vis;
      quote! {
        #vis struct #render_name #proxy_impl_generics(
          <#proxy_ty as RenderWidget>::RO
        ) #proxy_where_clause;

        impl #impl_generics RenderWidget for #ident #ty_generics #where_clause {
          type RO = #render_name #proxy_type_generics;

          #[inline]
          fn create_render_object(&self) -> Self::RO {
             #render_name(RenderWidget::create_render_object(&self.#path))
          }

          #[inline]
          fn take_children(&mut self) -> Option<SmallVec<[Box<dyn Widget>; 1]>> {
            RenderWidget::take_children(&mut self.#path)
          }
        }

        impl #proxy_impl_generics RenderObject for #render_name #proxy_type_generics #where_clause{
          type Owner = #ident #ty_generics;

          #[inline]
          fn update(&mut self, owner_widget: &Self::Owner, ctx: &mut UpdateCtx) {
            RenderObject::update(&mut self.0, &owner_widget.#path, ctx)
          }

          #[inline]
          fn perform_layout(&mut self, clamp: BoxClamp, ctx: &mut RenderCtx) -> Size {
            RenderObject:: perform_layout(&mut self.0, clamp, ctx)
          }

          #[inline]
          fn only_sized_by_parent(&self) -> bool { RenderObject:: only_sized_by_parent(&self.0) }

          #[inline]
          fn paint<'a>(&'a self, ctx: &mut PaintingContext<'a>) {
            RenderObject:: paint(&self.0, ctx)
          }

          #[inline]
          fn transform(&self) -> Option<Transform> { RenderObject:: transform(&self.0) }
        }
      }
    }
    Err(err) => err,
  }
}
