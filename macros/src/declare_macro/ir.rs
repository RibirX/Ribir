use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, Fields, Ident, Result, spanned::Spanned};

use super::{
  DECLARE_ATTR,
  parser::{DeclareAttr, DefaultMeta, EventMeta},
};
use crate::util::{declare_init_method, doc_attr};

pub struct Declarer<'a> {
  pub name: Ident,
  pub fields: Vec<DeclareField<'a>>,
  pub original: &'a syn::ItemStruct,
  pub validate: Option<Ident>,
  pub simple: bool,
  pub stateless: bool,
  pub eager: bool,
}

impl<'a> Declarer<'a> {
  pub fn new(item_stt: &'a mut syn::ItemStruct) -> Result<Self> {
    let host = &item_stt.ident;
    let name = Ident::new(&format!("{host}Declarer"), host.span());
    let mut validate = None;
    let mut simple = false;
    let mut stateless = false;
    let mut eager = false;
    item_stt.attrs.retain(|attr| {
      if attr.path().is_ident(DECLARE_ATTR)
        && let Ok(attr) = attr.parse_args::<DeclareAttr>()
      {
        if let Some(v) = attr.validate {
          validate = Some(
            v.method_name
              .unwrap_or_else(|| Ident::new("declare_validate", v.validate_kw.span())),
          );
        }
        if attr.simple.is_some() {
          simple = true;
        }
        if attr.stateless.is_some() {
          stateless = true;
        }
        if attr.eager.is_some() {
          eager = true;
        }
        return false;
      }
      true
    });

    let (original, item_stt) = unsafe {
      let ptr = item_stt as *mut syn::ItemStruct;
      (&*ptr, &mut *ptr)
    };
    let fields = match &mut item_stt.fields {
      Fields::Named(named) => collect_fields(named.named.iter_mut())?,
      Fields::Unnamed(unnamed) => collect_fields(unnamed.unnamed.iter_mut())?,
      Fields::Unit => vec![],
    };

    if fields.is_empty() {
      stateless = true;
    }

    Ok(Declarer { name, fields, original, validate, simple, stateless, eager })
  }

  /// Returns true if this mode needs PhantomData marker (only lazy needs it)
  pub fn needs_marker(&self) -> bool { !self.eager }

  pub fn all_members(&self) -> impl Iterator<Item = &Ident> {
    self.fields.iter().map(|f| f.member())
  }

  pub fn no_skip_fields(&self) -> impl Iterator<Item = &DeclareField<'_>> {
    self.fields.iter().filter(|f| f.is_not_skip())
  }

  pub fn host(&self) -> &Ident { &self.original.ident }
}

pub struct DeclareField<'a> {
  pub(crate) attr: Option<DeclareAttr>,
  pub(crate) field: &'a syn::Field,
}

impl<'a> DeclareField<'a> {
  pub fn member(&self) -> &Ident { self.field.ident.as_ref().unwrap() }

  pub(crate) fn attr(&self) -> Option<&DeclareAttr> { self.attr.as_ref() }

  pub fn is_not_skip(&self) -> bool { self.attr().is_none_or(|a| a.skip.is_none()) }

  pub fn is_strict(&self) -> bool { self.attr().is_some_and(|a| a.strict.is_some()) }

  pub fn set_method_name(&self) -> Ident {
    let name = self.field.ident.as_ref().unwrap();
    declare_init_method(name)
  }

  pub fn need_set_method(&self) -> bool {
    self
      .attr()
      .is_none_or(|a| a.custom.is_none() && a.skip.is_none())
  }

  pub fn doc_attr(&self) -> Option<&Attribute> { doc_attr(self.field) }

  fn setter_meta(&self) -> Option<&super::parser::SetterMeta> {
    self.attr().and_then(|a| a.setter.as_ref())
  }

  pub fn setter_name(&self) -> Option<&Ident> { self.setter_meta().map(|m| &m.method_name) }

  pub fn setter_ty(&self) -> Option<&syn::Type> { self.setter_meta().and_then(|m| m.ty.as_ref()) }

  /// Returns event metadata if this field has `event = on_xxx(EventType)`
  /// attribute
  pub fn event_meta(&self) -> Option<&EventMeta> { self.attr().and_then(|a| a.event.as_ref()) }

  pub fn default_value(&self) -> Option<TokenStream> {
    let attr = self.attr()?;
    match attr.default.as_ref() {
      Some(DefaultMeta { value: Some(v), .. }) => Some(quote! { RFrom::r_from(#v) }),
      Some(_) => Some(quote! { <_>::default() }),
      None if attr.skip.is_some() => Some(quote! { <_>::default() }),
      None => None,
    }
  }
}

fn collect_fields<'a>(
  fields: impl Iterator<Item = &'a mut syn::Field>,
) -> Result<Vec<DeclareField<'a>>> {
  fields
    .enumerate()
    .map(|(idx, f)| {
      if f.ident.is_none() {
        f.ident = Some(Ident::new(&format!("v_{idx}"), f.span()))
      }
      Ok(DeclareField { attr: take_build_attr(f)?, field: f })
    })
    .collect()
}

fn take_build_attr(field: &mut syn::Field) -> Result<Option<DeclareAttr>> {
  let idx = field
    .attrs
    .iter()
    .position(|attr| matches!(&attr.meta, syn::Meta::List(l) if l.path.is_ident(DECLARE_ATTR)));

  match idx {
    Some(idx) => {
      let attr = field.attrs.remove(idx);
      Ok(Some(attr.parse_args()?))
    }
    None => Ok(None),
  }
}
