use crate::attr_fields::{add_trait_bounds_if, AttrFields};
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::{parse_quote, spanned::Spanned, Data, DeriveInput, Generics, Ident};

pub const PROXY_PATH: &'static str = "proxy";

pub fn proxy_derive(
  input: &syn::DeriveInput,
  mut derive_impl: impl FnMut(&Generics, &AttrFields, &Ident, TokenStream) -> TokenStream,
) -> TokenStream {
  let DeriveInput {
    ident,
    generics,
    data,
    ..
  } = input;
  match data {
    Data::Struct(stt) => {
      let attr_fields = AttrFields::new(&stt, &generics, PROXY_PATH);
      let fields = attr_fields.attr_fields();
      match fields.len() {
        0 => {
          quote_spanned! {
            ident.span() => compile_error!("Must specify a `#[proxy] attr to one field.");
          }
        }
        1 => {
          let (f, idx) = &attr_fields.attr_fields()[0];
          let path = f.ident.as_ref().map_or_else(
            || {
              let index = syn::Index::from(*idx);
              quote! {#index}
            },
            |f| quote! {#f},
          );
          derive_impl(generics, &attr_fields, ident, path)
        }
        _ => {
          let too_many = fields.iter().map(|(f, _)| {
            quote_spanned! {
              f.attrs.iter().find(|attr| {
                attr.path.segments.len() == 1 && attr.path.segments[0].ident != PROXY_PATH
              }).span() =>compile_error!("Too may `#[proxy]` specified, only once need.");
            }
          });
          quote! {
            #(#too_many)*
          }
        }
      }
    }
    Data::Enum(_) => {
      unimplemented!("Unimplemented know");
    }
    _ => {
      quote_spanned! {
        ident.span() => compile_error!("`CombinationWidget` can not derived by this type.");
      }
    }
  }
}

pub fn combination_derive(input: &syn::DeriveInput) -> TokenStream {
  proxy_derive(
    input,
    |generics: &Generics, attr_fields: &AttrFields, ident: &Ident, path| {
      let generics =
        add_trait_bounds_if(generics.clone(), parse_quote!(CombinationWidget), |param| {
          attr_fields.is_attr_generic(param)
        });
      let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

      quote! {
        impl #impl_generics CombinationWidget for #ident #ty_generics #where_clause {
          #[inline]
          fn build(&self, ctx: &mut BuildCtx) -> BoxWidget {
            self.#path.build(ctx)
          }
        }
      }
    },
  )
}
