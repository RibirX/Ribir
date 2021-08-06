use proc_macro2::TokenStream;
use quote::quote;

pub fn attach_attr_derive(input: &syn::DeriveInput) -> TokenStream {
  let (g_impl, g_ty, g_where) = input.generics.split_for_impl();
  let name = &input.ident;
  quote! {
       impl #g_impl AttachAttr for #name #g_ty #g_where {
        type W = Self;

        fn into_attr_widget(self) -> AttrWidget<Self::W> {
          AttrWidget {widget: self, attrs: Default::default()}
        }
      }
  }
}
