use crate::attr_fields::{add_trait_bounds_if, AttrFields};
use crate::combination_derive::proxy_derive;
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{parse_quote, Generics, Ident};

pub fn render_derive(input: &syn::DeriveInput) -> TokenStream {
  proxy_derive(
    input,
    |generics: &Generics, attr_fields: &AttrFields, ident: &Ident| {
      let generics = add_trait_bounds_if(generics.clone(), parse_quote!(RenderWidget), |param| {
        attr_fields.is_attr_generic(param)
      });
      let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

      let field = &attr_fields.attr_fields()[0];
      let proxy_ty = &field.ty;
      let path = &field.ident;
      let render_name = Ident::new(&format!("{}Render", ident), Span::call_site());

      let proxy_generics = add_trait_bounds_if(
        attr_fields.state_generics(),
        parse_quote!(RenderWidget),
        |param| attr_fields.is_attr_generic(param),
      );
      let (proxy_impl_generics, proxy_ty_generics, proxy_where_clause) =
        proxy_generics.split_for_impl();

      // todo!("support tuple, should hold index in state fields");
      quote! {
        #[derive(Debug)]
        struct #render_name #proxy_impl_generics(<#proxy_ty as RenderWidget>::RO) #proxy_where_clause;

        impl #impl_generics RenderWidget for #ident #ty_generics #where_clause {
          type RO = #render_name #ty_generics;

          #[inline]
          fn create_render_object(&self) -> Self::RO {
             #render_name(self.#path.create_render_object())
          }

          #[inline]
          fn take_children(&mut self) -> Option<SmallVec<[BoxWidget; 1]>> {
            self.#path.take_children()
          }
        }

        impl #impl_generics RenderObject for #render_name #ty_generics #where_clause{
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
          fn paint<'a>(&'a self, ctx: &mut PaintingContext<'a>) { RenderObject:: paint(&self.0, ctx) }

          #[inline]
          fn transform(&self) -> Option<Transform> { RenderObject:: transform(&self.0) }
        }
      }
    },
  )
}
