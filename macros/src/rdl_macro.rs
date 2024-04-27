use std::collections::HashSet;

use proc_macro::TokenStream as TokenStream1;
use proc_macro2::{Span, TokenStream};
use quote::{quote_spanned, ToTokens};
use syn::{
  braced,
  fold::Fold,
  parse::{Parse, ParseBuffer, ParseStream},
  parse_macro_input, parse_quote,
  punctuated::Punctuated,
  spanned::Spanned,
  token::{Brace, Colon, Comma, Dollar, Not},
  Expr, Ident, Macro, Path, Result as SynResult, Stmt,
};

use crate::{
  declare_obj::DeclareObj,
  ok,
  symbol_process::{kw, symbol_to_macro, DollarRefsCtx},
};

pub enum RdlMacro {
  Literal(StructLiteral),
  /// Declare an expression as a object, like `rdl!{ Widget::new(...) }`
  ExprObj {
    span: Span,
    stmts: Vec<Stmt>,
  },
}

/// Declare a object use struct literal, like `rdl!{ Row { ... } }` or
/// `@parent { ... }`
pub struct StructLiteral {
  pub span: Span,
  pub parent: RdlParent,
  pub fields: Punctuated<DeclareField, Comma>,
  /// Declare a child in `rdl!` can use `rdl!` macro or `@` symbol.
  /// `rdl!{ Row { rdl!{ SizedBox {...} } } }`
  /// or
  /// `rdl!{ Row { @ SizedBox{ ... } } }`
  /// and the second case will be instead by
  /// ```ignore
  /// rdl!{ Row { rdl!{ SizedBox {...} } } }
  /// ```
  ///  in preprocessor.
  pub children: Vec<Macro>,
}

pub enum RdlParent {
  /// Declare parent use a type `Row { ... }`
  Type(Path),
  /// Declare parent use a variable prefixed with ` @parent { ... }`
  Var(Ident),
}

/// Declare a field of a widget.
pub struct DeclareField {
  /// field member name.
  pub member: Ident,
  pub value: Expr,
}

impl RdlMacro {
  pub fn gen_code(input: TokenStream, refs: &mut DollarRefsCtx) -> TokenStream1 {
    let input = ok!(symbol_to_macro(TokenStream1::from(input)));

    match parse_macro_input! { input as RdlMacro } {
      RdlMacro::Literal(mut l) => {
        let fields = l.fields.into_iter().map(|mut f: DeclareField| {
          f.value = refs.fold_expr(f.value);
          f
        });
        l.fields = fields.collect();
        l.children = l
          .children
          .into_iter()
          .map(|m| refs.fold_macro(m))
          .collect();

        let obj = ok!(DeclareObj::from_literal(&l));
        if let Err(err) = obj.error_check() {
          err.to_compile_error().into()
        } else {
          obj.to_token_stream().into()
        }
      }
      RdlMacro::ExprObj { span, stmts } => {
        let stmts = stmts.into_iter().map(|s| refs.fold_stmt(s));
        if stmts.len() > 1 {
          quote_spanned! { span => { #(#stmts)* }}.into()
        } else {
          quote_spanned! { span => #(#stmts)* }.into()
        }
      }
    }
  }
}

impl Parse for RdlMacro {
  fn parse(input: ParseStream) -> SynResult<Self> {
    let fork = input.fork();
    if fork.parse::<RdlParent>().is_ok() && fork.peek(Brace) {
      Ok(RdlMacro::Literal(input.parse()?))
    } else {
      Ok(RdlMacro::ExprObj { span: input.span(), stmts: syn::Block::parse_within(input)? })
    }
  }
}

impl Parse for StructLiteral {
  fn parse(input: ParseStream) -> SynResult<Self> {
    let span = input.span();
    let parent = input.parse()?;
    let content;
    let _ = braced!(content in input);
    let mut children = vec![];
    let mut fields = Punctuated::default();
    loop {
      if content.is_empty() {
        break;
      }

      if content.peek(kw::rdl) && content.peek2(Not) {
        children.push(content.parse()?);
      } else if content.peek(Ident) {
        let f: DeclareField = content.parse()?;
        if !children.is_empty() {
          let err_msg = "Field should always declare before children.";
          return Err(syn::Error::new(f.span(), err_msg));
        }
        fields.push(f);
        if !content.is_empty() {
          fields.push_punct(content.parse()?);
        }
      } else {
        return Err(syn::Error::new(content.span(), "expected a field or a child."));
      }
    }

    check_duplicate_field(&fields)?;
    Ok(StructLiteral { span, parent, fields, children })
  }
}

impl Parse for RdlParent {
  fn parse(input: ParseStream) -> SynResult<Self> {
    if input.peek(kw::_dollar_ಠ_ಠ) && input.peek2(Not) {
      let mac: Macro = input.parse()?;

      Ok(RdlParent::Var(mac.parse_body_with(|input: &ParseBuffer| {
        input.parse::<Dollar>()?;
        input.parse()
      })?))
    } else {
      Ok(RdlParent::Type(input.parse()?))
    }
  }
}

impl Parse for DeclareField {
  fn parse(input: ParseStream) -> SynResult<Self> {
    let member: Ident = input.parse()?;
    let colon_tk: Option<Colon> = input.parse()?;
    let value = if colon_tk.is_none() { parse_quote!(#member) } else { input.parse()? };

    Ok(DeclareField { member, value })
  }
}

impl ToTokens for DeclareField {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    let DeclareField { member, value, .. } = self;
    quote_spanned! {value.span()=> .#member(#value)}.to_tokens(tokens);
  }
}

/// Check if a field is declared more than once.
fn check_duplicate_field(fields: &Punctuated<DeclareField, Comma>) -> syn::Result<()> {
  let mut sets = HashSet::<&Ident, ahash::RandomState>::default();
  for f in fields {
    if !sets.insert(&f.member) {
      return Err(syn::Error::new(
        f.member.span(),
        format!("`{}` declare more than once", f.member).as_str(),
      ));
    }
  }
  Ok(())
}
