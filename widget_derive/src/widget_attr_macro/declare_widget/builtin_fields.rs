use ::builtin::builtin;
use inflector::Inflector;
use lazy_static::lazy_static;
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote_spanned;
use smallvec::SmallVec;
use std::collections::{BTreeMap, HashMap};

use crate::{
  error::DeclareError,
  widget_attr_macro::{ribir_suffix_variable, DeclareCtx, IdType, ObjectUsed},
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

pub fn is_listener(ty_name: &str) -> bool { ty_name.ends_with("Listener") }

#[derive(Debug, Default)]
pub struct BuiltinFieldWidgets {
  widgets: HashMap<&'static str, BuiltinWidgetInfo, ahash::RandomState>,
}

#[derive(Debug, Default)]
struct BuiltinWidgetInfo(SmallVec<[DeclareField; 1]>);

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
      .flat_map(|info| info.0.iter_mut())
      .for_each(|f| ctx.visit_declare_field_mut(f))
  }

  pub fn all_builtin_fields(&self) -> impl Iterator<Item = &DeclareField> {
    self.widgets.values().flat_map(|info| info.0.iter())
  }

  pub fn collect_builtin_widget_follows<'a>(
    &'a self,
    host: &Ident,
    follows_info: &mut BTreeMap<Ident, ObjectUsed<'a>>,
  ) {
    self.widgets.iter().for_each(|(name, info)| {
      let follows: ObjectUsed = info.0.iter().filter_map(|f| f.used_part()).collect();
      if !follows.is_empty() {
        let name = ribir_suffix_variable(host, BUILTIN_WIDGET_SUFFIX.get(name).unwrap());
        follows_info.insert(name, follows);
      }
    });
  }

  // todo: key should not as a builtin widget again.
  pub fn key_follow_check(&self) -> crate::error::Result<()> {
    if let Some((_, info)) = self.widgets.iter().find(|(name, _)| "KeyWidget" == **name) {
      assert_eq!(info.0.len(), 1);
      let DeclareField { member, expr, .. } = &info.0[0];
      if let Some(follows) = expr.used_name_info.directly_used_widgets() {
        return Err(DeclareError::KeyDependsOnOther {
          key: member.span().unwrap(),
          depends_on: follows.map(|w| w.span().unwrap()).collect(),
        });
      }
    }

    Ok(())
  }

  pub fn widget_tokens_iter<'a>(
    &'a self,
    host_id: &'a Ident,
    ctx: &'a DeclareCtx,
  ) -> impl Iterator<Item = (Ident, TokenStream)> + '_ {
    WIDGETS
      .iter()
      .filter_map(|builtin| self.get_builtin_widget(host_id, ctx, builtin))
      .filter(|(_, ty_name, info)| {
        // we provide a default implementation for a builtin widget if it not declared,
        // but used by others except it's a listener. We can't give a default handler
        // for it, its implementation should depend on who used it.
        !is_listener(ty_name) || info.is_some()
      })
      .map(|(var_name, ty_name, info)| {
        let tokens = if let Some(info) = info {
          let ty = Ident::new(ty_name, info.span()).into();
          let gen = WidgetGen::new(&ty, &var_name, info.0.iter(), false);
          gen.gen_widget_tokens(ctx)
        } else {
          let ty = Ident::new(ty_name, host_id.span()).into();
          let gen = WidgetGen::new(&ty, &var_name, [].into_iter(), false);
          gen.gen_widget_tokens(ctx)
        };
        (var_name, tokens)
      })
  }
  pub fn collect_names(&self, host: &Ident, ctx: &mut DeclareCtx) -> Result<(), DeclareError> {
    for builtin in WIDGETS.iter() {
      let ty_name = builtin.ty;
      if self.widgets.get(ty_name).is_some() {
        let suffix = BUILTIN_WIDGET_SUFFIX.get(ty_name).unwrap();
        let var_name = ribir_suffix_variable(host, suffix);
        ctx.add_named_obj(var_name, IdType::DECLARE)?;
      }
    }
    Ok(())
  }

  pub fn add_user_perspective_pairs(&self, host: &Ident, ctx: &mut DeclareCtx) {
    for builtin in WIDGETS.iter() {
      if let Some((builtin_name, _, _)) = self.get_builtin_widget(host, ctx, builtin) {
        ctx.add_user_perspective_pair(builtin_name, host.clone())
      }
    }
  }

  /// return builtin fields composed tokens, and the upstream tokens if the
  /// finally widget as a expr widget.
  pub fn compose_tokens(&self, name: &Ident, ctx: &DeclareCtx, tokens: &mut TokenStream) {
    WIDGETS
      .iter()
      .filter_map(|builtin| self.get_builtin_widget(name, ctx, builtin))
      .for_each(|(var_name, _, info)| {
        let span = info.map_or_else(|| name.span(), |info| info.span());
        tokens.extend(quote_spanned! { span =>
          let #name: SingleChildWidget<_, _> = #var_name.have_child(#name);
        });
      });
  }

  pub fn is_builtin_field(host: &syn::Path, field: &DeclareField) -> Option<&'static str> {
    FIELD_WIDGET_TYPE
      .get(field.member.to_string().as_str())
      .filter(|ty| !host.is_ident(ty))
      .cloned()
  }

  pub fn fill_as_builtin_field(&mut self, widget_ty: &'static str, field: DeclareField) {
    assert_eq!(
      FIELD_WIDGET_TYPE.get(field.member.to_string().as_str()),
      Some(&widget_ty)
    );

    let info = self.widgets.entry(widget_ty).or_default();
    info.0.push(field);
  }

  fn get_builtin_widget<'a>(
    &'a self,
    host_id: &'a Ident,
    ctx: &'a DeclareCtx,
    builtin: &'a BuiltinWidget,
  ) -> Option<(Ident, &str, Option<&BuiltinWidgetInfo>)> {
    let ty_name = builtin.ty;
    let suffix = BUILTIN_WIDGET_SUFFIX.get(ty_name).unwrap();
    let var_name = ribir_suffix_variable(host_id, suffix);
    if let Some(info) = self.widgets.get(ty_name) {
      Some((var_name, ty_name, Some(info)))
    } else if ctx.is_used(&var_name) || ctx.animate_listener_triggers.contains(&var_name) {
      Some((var_name, ty_name, None))
    } else {
      None
    }
  }
}

impl BuiltinWidgetInfo {
  fn span(&self) -> Span {
    self
      .0
      .iter()
      .fold(None, |span: Option<Span>, f| {
        if let Some(span) = span {
          span.join(f.member.span())
        } else {
          Some(f.member.span())
        }
      })
      .unwrap()
  }
}
