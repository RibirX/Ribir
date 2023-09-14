use ::ribir_builtin::builtin;
use inflector::Inflector;
use lazy_static::lazy_static;
use std::collections::HashMap;
use syn::Ident;

include!("./builtin_fields_list.rs");

lazy_static! {
  pub static ref RESERVE_IDENT: HashMap<&'static str, &'static str, ahash::RandomState> = WIDGETS
    .iter()
    .flat_map(|w| w.fields.iter())
    .map(|f| (f.name, f.doc))
    .collect();
  pub static ref WIDGET_OF_BUILTIN_METHOD: HashMap<&'static str, &'static str, ahash::RandomState> =
    WIDGETS
      .iter()
      .flat_map(|w| w.methods.iter().map(|m| (m.name, w.ty)))
      .collect();
  pub static ref WIDGET_OF_BUILTIN_FIELD: HashMap<&'static str, &'static str, ahash::RandomState> =
    WIDGETS
      .iter()
      .flat_map(|w| w.fields.iter().map(|f| (f.name, w.ty)))
      .collect();
  pub static ref BUILTIN_WIDGET_SUFFIX: HashMap<&'static str, String, ahash::RandomState> = WIDGETS
    .iter()
    .map(|w| (w.ty, w.ty.to_snake_case()))
    .collect();
}

pub(crate) const AVOID_CONFLICT_SUFFIX: &str = "ಠ_ಠ";

pub fn ribir_suffix_variable(from: &Ident, suffix: &str) -> Ident {
  let name_str = from.to_string();
  let prefix_size = if name_str.ends_with(AVOID_CONFLICT_SUFFIX) {
    name_str.len() - AVOID_CONFLICT_SUFFIX.len() - 1
  } else {
    name_str.len()
  };
  let prefix = &name_str[..prefix_size];
  let name = format!("{prefix}_{suffix}_{AVOID_CONFLICT_SUFFIX}");
  Ident::new(&name, from.span())
}
