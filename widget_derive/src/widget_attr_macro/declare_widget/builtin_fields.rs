use ::builtin::builtin;
use inflector::Inflector;
use lazy_static::lazy_static;
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote_spanned;
use smallvec::SmallVec;
use std::collections::{BTreeMap, HashMap};
use syn::spanned::Spanned;

use crate::{
  error::DeclareError,
  widget_attr_macro::{ribir_suffix_variable, DeclareCtx, NameUsed},
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
    follows_info: &mut BTreeMap<Ident, NameUsed<'a>>,
  ) {
    self.widgets.iter().for_each(|(name, info)| {
      let follows: NameUsed = info.0.iter().filter_map(|f| f.used_part()).collect();
      if !follows.is_empty() {
        let name = ribir_suffix_variable(host, BUILTIN_WIDGET_SUFFIX.get(name).unwrap());
        follows_info.insert(name, follows);
      }
    });
  }

  pub fn key_follow_check(&self) -> crate::error::Result<()> {
    if let Some((_, info)) = self.widgets.iter().find(|(name, _)| "KeyWidget" == **name) {
      assert_eq!(info.0.len(), 1);
      let DeclareField { member, used_name_info, .. } = &info.0[0];
      if let Some(follows) = used_name_info.directly_used_widgets() {
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
    // builtin widgets compose in special order.
    WIDGETS
      .iter()
      .filter_map(|builtin| self.widgets.get_key_value(builtin.ty))
      .map(move |(ty_name, info)| {
        let suffix = BUILTIN_WIDGET_SUFFIX.get(ty_name).unwrap();
        let name = ribir_suffix_variable(host_id, suffix);

        let span = info.span();
        let ty = Ident::new(ty_name, span).into();

        let fields = &info.0;
        let gen = WidgetGen::new(&ty, &name, fields.iter());
        let tokens = gen.gen_widget_tokens(ctx);

        (name, tokens)
      })
  }

  pub fn builtin_widget_names<'a>(
    &'a self,
    host_name: &'a Ident,
  ) -> impl Iterator<Item = Ident> + '_ {
    self.widgets.keys().map(|w| {
      let suffix = BUILTIN_WIDGET_SUFFIX.get(w).unwrap();
      ribir_suffix_variable(host_name, suffix)
    })
  }

  /// return builtin fields composed tokens, and the upstream tokens if the
  /// finally widget as a expr widget.
  pub fn compose_tokens(&self, name: &Ident, is_expr_host: bool, tokens: &mut TokenStream) {
    WIDGETS
      .iter()
      .filter_map(|builtin| self.widgets.get_key_value(builtin.ty))
      .fold(is_expr_host, |is_expr_widget, (builtin_ty, info)| {
        let suffix = BUILTIN_WIDGET_SUFFIX.get(builtin_ty).unwrap();
        let builtin_name = ribir_suffix_variable(name, suffix);
        let span = info.span();
        if is_expr_widget {
          tokens.extend(quote_spanned! { span =>
             let #name = SingleChildWidget::from_expr_child(#builtin_name, #name);
          });
        } else {
          tokens.extend(quote_spanned! { span =>
            let #name = SingleChildWidget::new(#builtin_name, #name);
          });
        }
        false
      });
  }

  pub fn is_builtin_field(host: &syn::Path, field: &DeclareField) -> Option<&'static str> {
    FIELD_WIDGET_TYPE
      .get(field.member.to_string().as_str())
      .filter(|ty| !host.is_ident(ty))
      .cloned()
  }

  pub fn fill_as_builtin_field(
    &mut self,
    widget_ty: &'static str,
    field: DeclareField,
  ) -> syn::Result<()> {
    assert_eq!(
      FIELD_WIDGET_TYPE.get(field.member.to_string().as_str()),
      Some(&widget_ty)
    );

    let info = self.widgets.entry(widget_ty).or_default();

    if info.0.iter().any(|f| f.member == field.member) {
      return Err(syn::Error::new(
        field.span(),
        format!("field `{}` specified more than once", stringify!($name)).as_str(),
      ));
    }
    info.0.push(field);
    Ok(())
  }

  pub fn is_empty(&self) -> bool { self.widgets.is_empty() }
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
