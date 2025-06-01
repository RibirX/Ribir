use std::collections::HashSet;

use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote, quote_spanned};
use smallvec::SmallVec;
use syn::{
  Expr, Ident, Macro, Path, Result as SynResult, Stmt, braced,
  fold::Fold,
  parse::{Parse, ParseBuffer, ParseStream},
  parse_quote,
  punctuated::Punctuated,
  spanned::Spanned,
  token::{Brace, Colon, Comma, Dollar, Not},
};

use crate::{
  declare_obj::DeclareObj,
  error::result_to_token_stream,
  symbol_process::{DollarRefsCtx, kw, symbol_to_macro},
  util::declare_init_method,
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
  pub children: SmallVec<[Macro; 1]>,
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
  pub fn gen_code(input: TokenStream, ctx: Option<&mut DollarRefsCtx>) -> TokenStream {
    let res = symbol_to_macro(input).and_then(|input| {
      let rdl = syn::parse2::<RdlMacro>(input)?;
      if let Some(ctx) = ctx {
        RdlMacro::gen_rdl(rdl, ctx)
      } else {
        let mut ctx = DollarRefsCtx::top_level();
        let mut tokens = RdlMacro::gen_rdl(rdl, &mut ctx)?;
        let refs = ctx.pop_dollar_scope(false);
        if !refs.is_empty() {
          tokens = quote! {{ #refs; #tokens }};
        }
        Ok(tokens)
      }
    });
    result_to_token_stream(res)
  }

  fn gen_rdl(self, refs: &mut DollarRefsCtx) -> crate::error::Result<TokenStream> {
    let tokens = match self {
      RdlMacro::Literal(stl) => {
        let obj = DeclareObj::from_literal(stl, refs);
        obj.error_check()?;
        obj.to_token_stream()
      }
      RdlMacro::ExprObj { span, stmts } => {
        let stmts = stmts.into_iter().map(|s| refs.fold_stmt(s));
        if stmts.len() > 1 {
          quote_spanned! { span => { #(#stmts)* }}
        } else {
          quote_spanned! { span => #(#stmts)* }
        }
      }
    };
    Ok(tokens)
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
    let mut children = SmallVec::default();
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
    let init_method = declare_init_method(member);
    quote_spanned! {value.span()=> .#init_method(#value)}.to_tokens(tokens);
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
