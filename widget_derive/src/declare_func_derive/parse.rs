//! # Declare Macro tokens Syntax
//! ```ignore
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
//! DataFlow: data_flow ! DataFlowBody
//!
//! DataFlowBody: [ DataFlowExprs ]
//!   | ( DataFlowExprs )
//!   | { DataFlowExprs }
//!
//! DataFlowExprs: DataFlowExpr (; DataFlowExpr)* ;？
//!
//! DataFlowExpr: Expr ～> Expr  | Expr < Expr
//! ```

use super::*;
use syn::{
  bracketed,
  parse::{Parse, ParseBuffer, ParseStream, Parser},
  punctuated::Punctuated,
  spanned::Spanned,
  token::{self, Brace, Comma},
  Ident, Macro, Token,
};

impl Parse for DeclareMacro {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    fn parse_data_flows_tokens(input: ParseStream) -> syn::Result<Punctuated<DataFlow, Token![;]>> {
      Ok(Punctuated::parse_terminated(input)?)
    }
    fn parse_data_flows(input: ParseStream) -> syn::Result<Punctuated<DataFlow, Token![;]>> {
      if input.is_empty() {
        return Ok(<_>::default());
      }
      let lookahead = input.lookahead1();
      if lookahead.peek(kw::data_flow) && input.peek2(Token![!]) {
        let mac: Macro = input.parse()?;
        Ok(parse_data_flows_tokens.parse2(mac.tokens)?)
      } else {
        Err(lookahead.error())
      }
    }

    let res = Self {
      widget: input.parse()?,
      data_flows: parse_data_flows(input)?,
    };

    Ok(res)
  }
}

impl Parse for DeclareWidget {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    fn peek2_none(input: ParseBuffer) -> bool { input.parse::<Ident>().is_ok() && input.is_empty() }

    fn is_field(input: ParseStream) -> bool {
      input.peek(Ident)
        && (input.peek2(Token![if])
          || input.peek2(Token![:])
          || input.peek2(Token![,])
          || peek2_none(input.fork()))
        || input.fork().parse::<SkipNcAttr>().is_ok()
    }

    fn parse_fields(input: ParseStream) -> syn::Result<Punctuated<DeclareField, Token!(,)>> {
      let mut punctuated = Punctuated::new();
      while is_field(&input) {
        punctuated.push(input.parse()?);
        if input.is_empty() {
          break;
        }
        punctuated.push_punct(input.parse()?);
      }
      Ok(punctuated)
    }

    let content;
    let mut widget = DeclareWidget {
      path: input.parse()?,
      brace_token: syn::braced!(content in input),
      named: None,
      fields: <_>::default(),
      sugar_fields: <_>::default(),
      rest: None,
      children: vec![],
    };

    let fields = parse_fields(&content)?;

    fields
      .into_pairs()
      .try_for_each::<_, syn::Result<()>>(|pair| {
        let (f, comma) = pair.into_tuple();

        let member = &f.member;
        if syn::parse2::<kw::id>(quote! { #member}).is_ok() {
          let name = Id::from_declare_field(f)?;
          let _: Option<DeclareField> = assign_uninit_field!(widget.named, name)?;
        } else if let Some(f) = widget.sugar_fields.assign_field(f)? {
          widget.fields.push(f);
          if let Some(comma) = comma {
            widget.fields.push_punct(comma);
          }
        }
        Ok(())
      })?;

    if content.peek(Token![..]) {
      widget.rest = Some(content.parse()?);
      if !content.is_empty() {
        let _: Comma = content.parse()?;
      }
    }

    loop {
      // Expr child should not a `Type` or `Path`, if it's a `Ident`（`Path`), it's
      // ambiguous  with `DeclareChild`, and perfer as `DeclareField`.
      match content.fork().parse() {
        Err(_) => break,
        Ok(Child::Expr(Expr::Path(_))) => break,
        Ok(Child::Expr(Expr::Type(_))) => break,
        _ => {}
      }

      widget.children.push(content.parse()?);
      // Comma follow Child is option.
      let _: Option<Comma> = content.parse()?;
    }

    // syntax error hint.
    if !content.is_empty() {
      if is_field(&content) {
        let f: DeclareField = content.parse()?;
        if !widget.children.is_empty() {
          return Err(syn::Error::new(
            f.span(),
            "Field should alwyas declare before children.",
          ));
        }
        if let Some(RestExpr(dot, expr)) = widget.rest {
          return Err(syn::Error::new(
            dot.span().join(expr.span()).unwrap(),
            "The base struct must always be the last field.",
          ));
        }
      }
      if content.peek(Token![..]) && !widget.children.is_empty() && widget.rest.is_none() {
        let rest: Token![..] = content.parse()?;
        return Err(syn::Error::new(
          rest.span(),
          "The base struct must always be the last field but declare before children.",
        ));
      }
    }

    Ok(widget)
  }
}

impl Parse for SkipNcAttr {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    let content;
    Ok(Self {
      pound_token: input.parse()?,
      bracket_token: bracketed!(content in input),
      skip_nc_meta: content.parse()?,
    })
  }
}

impl Parse for DataFlow {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    Ok(Self {
      skip_nc: try_parse_skip_nc(input)?,
      from: DataFlowExpr {
        expr: input.parse()?,
        follows: None,
      },
      _arrow_token: input.parse()?,
      to: DataFlowExpr {
        expr: input.parse()?,
        follows: None,
      },
    })
  }
}

impl Parse for RestExpr {
  fn parse(input: ParseStream) -> syn::Result<Self> { Ok(Self(input.parse()?, input.parse()?)) }
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

impl Parse for DeclareField {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    let skip_nc = try_parse_skip_nc(input)?;
    let member: Ident = input.parse()?;
    let if_guard = if input.peek(Token![if]) {
      Some(input.parse()?)
    } else {
      None
    };
    let colon_token: Option<_> = if if_guard.is_some() {
      Some(input.parse()?)
    } else {
      input.parse()?
    };

    let expr = if colon_token.is_some() {
      input.parse()?
    } else {
      Expr::Path(syn::ExprPath {
        attrs: Vec::new(),
        qself: None,
        path: Path::from(member.clone()),
      })
    };

    Ok(DeclareField {
      skip_nc,
      member,
      if_guard,
      colon_token,
      expr,
      follows: None,
    })
  }
}

fn try_parse_skip_nc(input: ParseStream) -> syn::Result<Option<SkipNcAttr>> {
  if input.peek(token::Pound) {
    Ok(Some(input.parse()?))
  } else {
    Ok(None)
  }
}

#[cfg(test)]
mod tests {
  use ribir::{prelude::*, test::widget_and_its_children_box_rect};

  #[test]
  fn if_guard_work() {
    let w = declare! {
      SizedBox {
        size if true => : Size::new(100., 100.),
        margin if false =>: EdgeInsets::all(1.),
        cursor if true =>: CursorIcon::Hand
      }
    };

    assert_eq!(w.get_cursor(), Some(CursorIcon::Hand));
    let (rect, _) = widget_and_its_children_box_rect(w, Size::new(500., 500.));
    assert_eq!(rect.size, Size::new(100., 100.));
  }
}
