use proc_macro2::{Span, TokenStream};
use syn::{parse_quote, spanned::Spanned, token::Where, Data, Generics};

pub fn struct_unwrap<'a>(
  data: &'a mut syn::Data,
  derive_trait: &'static str,
) -> syn::Result<&'a mut syn::DataStruct> {
  match data {
    Data::Struct(stt) => Ok(stt),
    Data::Enum(e) => {
      let err_str = format!("`{}` not support for Enum", derive_trait);
      Err(syn::Error::new(e.enum_token.span(), err_str))
    }
    Data::Union(u) => {
      let err_str = format!("`{}` not support for Union", derive_trait);
      Err(syn::Error::new(u.union_token.span(), err_str))
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
