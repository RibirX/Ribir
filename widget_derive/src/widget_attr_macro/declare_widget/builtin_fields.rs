use ::builtin::builtin;
use inflector::Inflector;
use lazy_static::lazy_static;
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use smallvec::SmallVec;
use std::{
  collections::{BTreeMap, HashMap},
  str::FromStr,
};
use syn::{parse_quote_spanned, spanned::Spanned};

use crate::{
  error::DeclareError,
  widget_attr_macro::{
    ribir_suffix_variable, widget_def_variable, DeclareCtx, FollowPart, Follows,
  },
};

use super::{widget_gen::WidgetGen, DeclareField};

include!("../../builtin_fields_list.rs");

lazy_static! {
  pub static ref RESERVE_IDENT: HashMap<&'static str, &'static str, ahash::RandomState> = WIDGETS
    .iter()
    .flat_map(|w| w.fields.iter())
    .map(|f| (f.name, f.doc))
    .collect();
  pub static ref FIELD_WIDGET_TYPE: HashMap<&'static str, &'static str, ahash::RandomState> =
    WIDGETS
      .iter()
      .flat_map(|w| w.fields.iter().map(|f| (f.name, w.ty)))
      .collect();
  static ref BUILTIN_WIDGET_SUFFIX: HashMap<&'static str, String, ahash::RandomState> = WIDGETS
    .iter()
    .map(|w| (w.ty, w.ty.to_snake_case()))
    .collect();
}

#[derive(Debug, Default)]
pub struct BuiltinFieldWidgets {
  widgets: HashMap<&'static str, SmallVec<[DeclareField; 1]>, ahash::RandomState>,
}

impl BuiltinFieldWidgets {
  pub fn as_builtin_widget(field_name: &Ident) -> Option<&String> {
    FIELD_WIDGET_TYPE
      .get(field_name.to_string().as_str())
      .and_then(|w| BUILTIN_WIDGET_SUFFIX.get(w))
  }

  pub fn visit_builtin_fields_mut(&mut self, ctx: &mut DeclareCtx) {
    self
      .widgets
      .values_mut()
      .flat_map(|v| v.iter_mut())
      .for_each(|f| ctx.visit_declare_field_mut(f))
  }

  pub fn all_builtin_fields(&self) -> impl Iterator<Item = &DeclareField> {
    self.widgets.values().flat_map(|v| v.iter())
  }

  pub fn collect_wrap_widget_follows<'a>(
    &'a self,
    host: &Ident,
    follows_info: &mut BTreeMap<Ident, Follows<'a>>,
  ) {
    self.widgets.iter().for_each(|(widget_ty, fields)| {
      let follows: Follows = fields
        .iter()
        .filter_map(FollowPart::from_widget_field)
        .collect();

      if !follows.is_empty() {
        let name = ribir_suffix_variable(host, BUILTIN_WIDGET_SUFFIX.get(widget_ty).unwrap());
        follows_info.insert(name, follows);
      }
    });
  }

  pub fn key_follow_check(&self) -> crate::error::Result<()> {
    if let Some(fields) = self.widgets.get("Key") {
      assert_eq!(fields.len(), 1);
      let DeclareField { member, follows, .. } = &fields[0];
      if let Some(follows) = follows {
        return Err(DeclareError::KeyDependsOnOther {
          key: member.span().unwrap(),
          depends_on: follows.iter().map(|fo| fo.widget.span().unwrap()).collect(),
        });
      }
    }

    Ok(())
  }

  pub fn widget_tokens_iter<'a>(
    &'a self,
    host_id: Ident,
    ctx: &'a DeclareCtx,
  ) -> impl Iterator<Item = (Ident, TokenStream)> + 'a {
    self.widgets.iter().map(move |(w_ty, fields)| {
      let suffix = BUILTIN_WIDGET_SUFFIX.get(w_ty).unwrap();
      let name = ribir_suffix_variable(&host_id, suffix);
      let span = fields
        .iter()
        .fold(None, |span: Option<Span>, f| {
          if let Some(span) = span {
            span.join(f.member.span())
          } else {
            Some(f.member.span())
          }
        })
        .unwrap();

      let tt = TokenStream::from_str(w_ty).unwrap();
      let ty: syn::Type = parse_quote_spanned! { span => #tt };

      let gen = WidgetGen { ty: &ty, name, fields: &fields };
      let wrap_name = widget_def_variable(&gen.name);
      let mut def_and_ref_tokens = gen.gen_widget_tokens(ctx);

      // If all fields have if guard and condition are false,  widget can
      // emit
      // todo:  use dynamic widget implement if guard?
      if fields.iter().all(|f| f.if_guard.is_some()) {
        def_and_ref_tokens = quote! {
          #def_and_ref_tokens
          let #wrap_name = (!#wrap_name.is_empty()).then(|| #wrap_name);
        };
      }

      (gen.name, def_and_ref_tokens)
    })
  }

  pub fn compose_tokens(&self, host: &Ident) -> TokenStream {
    let compose_iter = self.widgets.iter().map(|(ty, _)| {
      let suffix = BUILTIN_WIDGET_SUFFIX.get(ty).unwrap();
      let name = ribir_suffix_variable(host, suffix);
      let wrap_def = widget_def_variable(&name);
      let host_def = widget_def_variable(host);
      quote! {let #host_def = #wrap_def.have_child(#host_def);}
    });

    quote! {#(#compose_iter)*}
  }

  pub fn assign_builtin_field(
    &mut self,
    widget_ty: &'static str,
    field: DeclareField,
  ) -> syn::Result<()> {
    assert_eq!(
      FIELD_WIDGET_TYPE.get(field.member.to_string().as_str()),
      Some(&widget_ty)
    );

    let fields = self.widgets.entry(widget_ty).or_default();
    if fields.iter().find(|f| f.member == field.member).is_some() {
      return Err(syn::Error::new(
        field.span(),
        format!("field `{}` specified more than once", stringify!($name)).as_str(),
      ));
    }
    fields.push(field);
    Ok(())
  }
}
