use syn::{Data, spanned::Spanned};

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

pub fn doc_attr(field: &syn::Field) -> Option<&syn::Attribute> {
  field
    .attrs
    .iter()
    .find(|attr| matches!(&attr.meta, syn::Meta::NameValue(nv) if nv.path.is_ident("doc")))
}

pub fn declare_init_method(member: &syn::Ident) -> syn::Ident {
  if member.to_string().starts_with("on_") {
    member.clone()
  } else {
    syn::Ident::new(&format!("with_{}", member), member.span())
  }
}
