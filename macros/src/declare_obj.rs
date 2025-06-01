use proc_macro2::TokenStream;
use quote::{ToTokens, quote_spanned};
use smallvec::SmallVec;
use syn::{
  Ident, Macro, Path,
  fold::Fold,
  punctuated::Punctuated,
  spanned::Spanned,
  token::{Brace, Comma},
};

use crate::{
  error::Error,
  rdl_macro::{DeclareField, RdlParent, StructLiteral},
  symbol_process::{DollarRefsCtx, DollarRefsScope},
  variable_names::BUILTIN_INFOS,
};

pub struct DeclareObj {
  this: ObjNode,
  children: SmallVec<[Macro; 1]>,
}
enum ObjType {
  Type(Path),
  Expr {
    expr: Ident,
    /// A virtual scope that collects references used in the object's fields.
    ///
    /// This prevents double mutable borrow issues that can occur when
    /// expression object is captured by its built-in fields.
    ///
    /// # Example
    ///
    /// Without collecting references, consider the following example:
    ///
    /// ```ignore
    /// @(parent) {
    ///   on_mounted: move |e| { *$write(parent.visible()) = true }
    /// }
    /// ```
    ///
    /// This expands to:
    ///
    /// ```ignore
    /// parent.on_mounted({
    ///   let visible = parent.visible().clone_writer();
    ///   move |e| { *visible.write() = true }
    /// });
    /// ```
    ///
    /// This would result in two mutable borrows of `parent`:
    /// 1. The first mutable borrow occurs on `parent.on_mounted()`.
    /// 2. The second mutable borrow occurs on `parent.visible()`.
    fields_used: DollarRefsScope,
  },
}
struct ObjNode {
  node_type: ObjType,
  fields: Punctuated<DeclareField, Comma>,
}

impl DeclareObj {
  pub fn from_literal(stl: StructLiteral, refs: &mut DollarRefsCtx) -> Self {
    fn fold_fields(
      fields: Punctuated<DeclareField, Comma>, refs: &mut DollarRefsCtx,
    ) -> Punctuated<DeclareField, Comma> {
      fields
        .into_iter()
        .map(|mut f| {
          f.value = refs.fold_expr(f.value);
          f
        })
        .collect()
    }

    let StructLiteral { parent, mut fields, mut children } = stl;

    let node_type = match parent {
      RdlParent::Type(ty) => {
        fields = fold_fields(fields, refs);
        ObjType::Type(ty)
      }
      RdlParent::Expr(expr) => {
        // Collect all dollar references to this var in its fields.
        refs.new_dollar_scope(Some(expr.clone()));
        fields = fold_fields(fields, refs);
        let fields_used = refs.pop_dollar_scope(false);

        ObjType::Expr { expr, fields_used }
      }
    };
    children = children
      .into_iter()
      .map(|m| refs.fold_macro(m))
      .collect();
    let this = ObjNode { node_type, fields };
    DeclareObj { this, children }
  }
}

impl ToTokens for DeclareObj {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    let Self { this, children } = self;
    if self.is_one_line_node() && children.is_empty() {
      self.gen_node_tokens(tokens);
    } else {
      Brace::default().surround(tokens, |tokens| {
        self.gen_node_tokens(tokens);

        if !children.is_empty() {
          let mut children = Vec::with_capacity(self.children.len());
          for (i, c) in self.children.iter().enumerate() {
            let child = Ident::new(&format!("_child_{i}_ಠ_ಠ"), c.tokens.span());
            quote_spanned! { c.span() => let #child = #c; }.to_tokens(tokens);
            children.push(child)
          }
          match &this.node_type {
            ObjType::Type(ty) => quote_spanned! { ty.span() => _ಠ_ಠ }.to_tokens(tokens),
            ObjType::Expr { expr, .. } => expr.to_tokens(tokens),
          };
          children.into_iter().for_each(|name| {
            quote_spanned! { name.span() => .with_child(#name) }.to_tokens(tokens)
          });
        }
      })
    }
  }
}

impl DeclareObj {
  pub fn error_check(&self) -> Result<(), Error> {
    if matches!(self.this.node_type, ObjType::Expr { .. }) {
      let invalid_fields = self
        .this
        .fields
        .iter()
        .filter(|f| !BUILTIN_INFOS.contains_key(f.member.to_string().as_str()))
        .collect::<Vec<_>>();
      if !invalid_fields.is_empty() {
        let spans = invalid_fields
          .iter()
          .map(|f| f.member.span())
          .collect();
        return Err(Error::InvalidFieldInVar(spans));
      }
    }

    Ok(())
  }

  fn is_one_line_node(&self) -> bool {
    let this = &self.this;
    match &this.node_type {
      ObjType::Type { .. } => this.fields.is_empty(),
      ObjType::Expr { fields_used, .. } => fields_used.is_empty() && this.fields.is_empty(),
    }
  }

  fn gen_node_tokens(&self, tokens: &mut TokenStream) {
    match &self.this.node_type {
      ObjType::Type(ty) => {
        if self.children.is_empty() && self.this.fields.is_empty() {
          quote_spanned! { ty.span() => #ty::declarer().finish() }.to_tokens(tokens)
        } else {
          let span = ty.span();
          let name = Ident::new("_ಠ_ಠ", span);
          if self.this.fields.is_empty() {
            quote_spanned! { span => let #name = #ty::declarer().finish(); }.to_tokens(tokens);
          } else {
            quote_spanned! { span => let mut #name = #ty::declarer(); }.to_tokens(tokens);
            self.gen_fields_tokens(&name, tokens);
            if self.children.is_empty() {
              quote_spanned! { span => #name.finish() }.to_tokens(tokens);
            } else {
              quote_spanned! { span => let #name = #name.finish(); }.to_tokens(tokens);
            }
          }
        }
      }
      ObjType::Expr { expr, fields_used } => {
        // todo: we can not care if it self reference used in the fields, if
        // we capture builtin state as normal.

        // if has capture self, rename to `_ಠ_ಠ` avoid conflict name.
        let self_ref = fields_used.iter().find(|r| r.builtin.is_none());
        if let Some(mut self_ref) = self_ref.cloned() {
          fields_used
            .iter()
            .filter(|r| r.builtin.is_some())
            .for_each(|r| r.to_tokens(tokens));

          let name = Ident::new("_ಠ_ಠ", expr.span());
          quote_spanned! { expr.span() =>  let mut #name = #expr;}.to_tokens(tokens);
          let orig = std::mem::replace(&mut self_ref.name, name.clone());
          self_ref.capture_state(&orig, tokens);

          self.gen_fields_tokens(&name, tokens);
          // If a child exist, revert the variable name to compose children.
          if !self.children.is_empty() {
            quote_spanned! { expr.span() =>
              #[allow(unused_mut)]
              let mut #expr = #name;
            }
            .to_tokens(tokens);
          } else {
            name.to_tokens(tokens);
          }
        } else {
          fields_used.to_tokens(tokens);
          self.gen_fields_tokens(expr, tokens);
          if self.children.is_empty() {
            expr.to_tokens(tokens);
          }
        }
      }
    };
  }

  fn gen_fields_tokens(&self, var: &Ident, tokens: &mut TokenStream) {
    // If there are multiple fields, we avoid generating a chained call to prevent
    // borrowing twice. For example:
    //
    // ```
    // let mut x = ...;
    // X::declarer().with_a(&mut x.a).with_b(&mut x.b).finish();
    // ```
    //
    // In this scenario, `x` would be borrowed twice, causing a compilation failure
    // as Rust does not handle this.
    for f in self.this.fields.iter() {
      var.to_tokens(tokens);
      f.to_tokens(tokens);
      syn::Token![;](f.value.span()).to_tokens(tokens);
    }
  }
}
