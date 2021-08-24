use proc_macro2::{Span, TokenStream};
use quote::quote_spanned;
use syn::{parse_quote, spanned::Spanned, token::Where, Data, Generics, Ident};

pub fn prefix_ident(prefix: &str, ident: &TokenStream) -> Ident {
  syn::parse_str::<Ident>(&format!("{}{}", prefix, ident)).unwrap()
}

pub fn suffix_ident(suffix: &str, ident: &TokenStream) -> Ident {
  syn::parse_str::<Ident>(&format!("{}{}", ident, suffix)).unwrap()
}

pub fn struct_unwrap<'a>(
  data: &'a mut syn::Data,
  derive_trait: &'static str,
) -> Result<&'a mut syn::DataStruct, TokenStream> {
  match data {
    Data::Struct(stt) => Ok(stt),
    Data::Enum(e) => {
      let err_str = format!("`{}` not support for Enum", derive_trait);
      Err(quote_spanned! {
        e.enum_token.span() => compile_error!(#err_str);
      })
    }
    Data::Union(u) => {
      let err_str = format!("`{}` not support for Union", derive_trait);
      Err(quote_spanned! {
        u.union_token.span() => compile_error!(#err_str);
      })
    }
  }
}

pub fn add_where_bounds(generics: &mut Generics, bounds: TokenStream) -> &mut Generics {
  generics
    .where_clause
    .get_or_insert_with(|| syn::WhereClause {
      where_token: Where(Span::call_site()),
      predicates: <_>::default(),
    })
    .predicates
    .push(parse_quote! {#bounds});
  generics
}
