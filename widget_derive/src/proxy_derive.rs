use crate::attr_fields::AttrFields;
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::{spanned::Spanned, Data, DeriveInput, Generics, Ident};
pub const PROXY_PATH: &str = "proxy";

pub struct ProxyDeriveInfo<'a> {
  pub derive_trait: &'static str,
  pub attr_fields: AttrFields<'a>,
  pub ident: &'a Ident,
  pub generics: &'a Generics,
  pub attr_name: &'static str,
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

impl<'a> ProxyDeriveInfo<'a> {
  pub fn new(
    input: &'a mut syn::DeriveInput,
    derive_trait: &'static str,
    attr_name: &'static str,
  ) -> Result<Self, TokenStream> {
    let DeriveInput { ident, data, generics, .. } = input;

    let stt = struct_unwrap(data, derive_trait)?;
    let attr_fields = AttrFields::new(stt, generics, attr_name);
    Ok(Self {
      derive_trait,
      attr_fields,
      ident,
      generics,
      attr_name,
    })
  }

  pub fn attr_path(&self) -> TokenStream {
    let (f, idx) = &self.attr_fields.attr_fields()[0];
    let path = f.ident.as_ref().map_or_else(
      || {
        let index = syn::Index::from(*idx);
        quote! {#index}
      },
      |f| quote! {#f},
    );
    path
  }

  pub fn too_many_attr_specified_error(self) -> Result<Self, TokenStream> {
    if self.attr_fields.attr_fields().len() > 1 {
      let err_str = format!(
        "Too many `#[{}]` attr specified, need only one",
        self.attr_name,
      );
      Err(quote_spanned! {
       self.ident.span() => compile_error!(#err_str);
      })
    } else {
      Ok(self)
    }
  }

  pub fn none_attr_specified_error(self) -> Result<Self, TokenStream> {
    if self.attr_fields.attr_fields().is_empty() {
      let err_str = format!(
        "There is not `#[{}]` attr specified, required by {}",
        self.attr_name, self.derive_trait
      );
      Err(quote_spanned! {
       self. ident.span() => compile_error!(#err_str);
      })
    } else {
      Ok(self)
    }
  }
}
