use proc_macro2::Span;
use syn::{spanned::Spanned, Ident};

use super::Child;
const AVOID_CONFLICT_SUFFIX: &str = "ribir_ಠ_ಠ";

pub fn child_variable(c: &Child, idx: usize) -> Ident {
  let span = match c {
    Child::Declare(d) => d.path.span(),
    Child::Expr(e) => e.span(),
  };
  let child = Ident::new("c", span);
  ribir_suffix_variable(&child, &idx.to_string())
}

pub fn ribir_variable(name: &str, span: Span) -> Ident {
  let name = format!("{name}_{AVOID_CONFLICT_SUFFIX}");
  Ident::new(&name, span)
}

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

pub fn widget_def_variable(name: &Ident) -> Ident { ribir_suffix_variable(name, "def") }
