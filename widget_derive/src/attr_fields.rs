use proc_macro2::TokenStream;
use quote::quote;
use syn::{punctuated::Punctuated, token::Comma, DataStruct, Field, Fields, Generics, Path};

/// Pick fields from struct by specify inner attr.
pub struct AttrFields<'a> {
  generics: &'a Generics,
  attr_fields: Vec<(Field, usize)>,
  pub is_tuple: bool,
}

impl<'a> AttrFields<'a> {
  pub fn new(from: &'a mut DataStruct, generics: &'a Generics, attr_name: &'static str) -> Self {
    Self {
      attr_fields: Self::pick_attr_fields(from, attr_name),
      generics,
      is_tuple: matches!(from.fields, Fields::Unnamed(_)),
    }
  }

  fn pick_attr_fields(stt: &mut DataStruct, attr_name: &'static str) -> Vec<(Field, usize)> {
    let pick_state_fields = |fds: &mut Punctuated<Field, Comma>| -> Vec<(Field, usize)> {
      fds
        .iter_mut()
        .enumerate()
        .filter_map(|(idx, f)| {
          let len = f.attrs.len();
          f.attrs.retain(|attr| !pure_ident(&attr.path, attr_name));
          if f.attrs.len() != len {
            Some((f.clone(), idx))
          } else {
            None
          }
        })
        .collect()
    };

    match &mut stt.fields {
      Fields::Unit => vec![],
      Fields::Unnamed(fds) => pick_state_fields(&mut fds.unnamed),
      Fields::Named(fds) => pick_state_fields(&mut fds.named),
    }
  }

  pub fn proxy_bounds_generic(&self, trait_token: TokenStream) -> Generics {
    let mut generics = self.generics.clone();

    if !self.attr_fields.is_empty() {
      let (field, _) = &self.attr_fields[0];
      let proxy_ty = &field.ty;
      crate::util::add_where_bounds(&mut generics, quote! {#proxy_ty: #trait_token});
    }

    generics
  }

  pub fn attr_fields(&self) -> &[(Field, usize)] { &self.attr_fields }
}

pub fn pure_ident(path: &Path, attr_name: &'static str) -> bool {
  path.segments.len() == 1 && path.segments[0].ident == attr_name
}
