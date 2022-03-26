//! # Declare Macro tokens Syntax
//! ```ascii
//! 
//! DeclareMacroTokens: DeclareWidget DataFlow?
//!
//! DeclareWidget:
//!   PathInExpression {
//!     DeclareFields?
//!     (DeclareChild ,?)*
//!   }
//!
//! DeclareFields: DeclareField (, DeclareField)* ,?
//!
//! DeclareField: (#[skip_nc])? Ident ((if Expr : Expr) | ( : Expr))?
//!
//! DeclareChild: (DeclareWidget | Expr ,?)*
//!
//! Expr: `rust syntax`
//!
//! DataFlow: dataflows DataFlowBody
//!
//! DataFlowBody: [ DataFlowExprs ]
//!   | ( DataFlowExprs )
//!   | { DataFlowExprs }
//!
//! DataFlowExprs: DataFlowExpr (; DataFlowExpr)* ;?
//!
//! DataFlowExpr: Expr ～> Expr  | Expr < Expr
//! ```
use super::*;
use syn::{
  parse::{Parse, ParseStream},
  token::{self, Brace},
  Ident,
};

const CTX_DEFAULT_NAME: &str = "ctx";

impl Parse for DeclareMacro {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    let ctx = if !input.peek2(token::Brace) {
      let ctx = input.parse()?;
      input.parse::<token::Comma>()?;
      ctx
    } else {
      Ident::new(CTX_DEFAULT_NAME, Span::call_site())
    };

    let widget = input.parse()?;

    let mut dataflows = None;
    let mut animations = None;
    loop {
      if input.is_empty() {
        break;
      }
      let lookahead = input.lookahead1();
      if lookahead.peek(kw::dataflows) && input.peek2(token::Brace) {
        dataflows = Some(input.parse()?);
      } else if lookahead.peek(kw::animations) && input.peek2(token::Brace) {
        animations = Some(input.parse()?);
      } else {
        Err(lookahead.error())?
      }
    }

    Ok(Self {
      ctx_name: ctx,
      widget,
      dataflows,
      animations,
    })
  }
}

impl Parse for Child {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    if input.peek(Ident) && input.peek2(Brace) {
      Ok(Child::Declare(input.parse()?))
    } else {
      Ok(Child::Expr(input.parse()?))
    }
  }
}

impl Parse for IfGuard {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    Ok(IfGuard {
      if_token: input.parse()?,
      cond: input.parse()?,
      fat_arrow_token: input.parse()?,
    })
  }
}
