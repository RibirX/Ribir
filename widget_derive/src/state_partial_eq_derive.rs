use crate::attr_fields::add_trait_bounds_if;
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{parse_quote, Data, Fields, Index};

pub fn derive_state_partial_eq(input: &syn::DeriveInput) -> proc_macro2::TokenStream {
  let generics = add_trait_bounds_if(
    input.generics.clone(),
    parse_quote!(std::ops::PartialEq),
    |_| true,
  );

  let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
  let name = &input.ident;
  let method_impl = state_partial_eq_method_impl(&input.data, name);
  quote! {
    impl #impl_generics StatePartialEq<Self> for #name #ty_generics #where_clause {
      fn eq(&self, other: &Self) -> bool {
        #method_impl
      }
    }
  }
}

fn state_partial_eq_method_impl(data: &Data, name: &syn::Ident) -> TokenStream {
  fn fields_eq_impl(fields: &Fields, lhs: TokenStream, rhs: TokenStream) -> TokenStream {
    match fields {
      Fields::Named(ref fields) => {
        let recurse = fields.named.iter().map(|f| {
          let name = &f.ident;
          quote_spanned! {f.span() => StatePartialEq::eq(&#lhs.#name, &#rhs.#name) }
        });
        quote! { true #(&& #recurse)* }
      }
      Fields::Unnamed(ref fields) => {
        let recurse = fields.unnamed.iter().enumerate().map(|(i, f)| {
          let index = Index::from(i);
          quote_spanned! { f.span()=> StatePartialEq::eq(&#lhs.#index, &#rhs.#index) }
        });
        quote! { true #(&& #recurse)* }
      }
      Fields::Unit => {
        quote!(true)
      }
    }
  }
  match *data {
    Data::Struct(ref data) => fields_eq_impl(&data.fields, quote! { self }, quote! {other}),
    Data::Enum(ref data_enum) => {
      let recurse = data_enum.variants.iter().map(|v| {
        let arm = &v.ident;
        if v.fields.is_empty() {
          quote! { #name::#arm => matches!(other, #name::#arm),}
        } else {
          let fields_eq = fields_eq_impl(&v.fields, quote! { lhs }, quote! { rhs });
          quote! {
            lhs @ #name::#arm => {
              if let rhs @ #name::#arm = other {
                #fields_eq
              } else {
                false
              }
            },
          }
        }
      });

      quote! {
        match other {
          #(#recurse)*
        }
      }
    }
    Data::Union(_) => unimplemented!(),
  }
}
