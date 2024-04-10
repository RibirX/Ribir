use syn::{spanned::Spanned, Data};

pub fn data_struct_unwrap<'a>(
  data: &'a mut syn::Data, derive_trait: &'static str,
) -> syn::Result<&'a mut syn::DataStruct> {
  match data {
    Data::Struct(stt) => Ok(stt),
    Data::Enum(e) => {
      let err_str = format!("`{derive_trait}` not support for Enum");
      Err(syn::Error::new(e.enum_token.span(), err_str))
    }
    Data::Union(u) => {
      let err_str = format!("`{derive_trait}` not support for Union");
      Err(syn::Error::new(u.union_token.span(), err_str))
    }
  }
}
