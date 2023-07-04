use proc_macro2::TokenStream;
use quote::{quote_spanned, ToTokens};
use std::collections::HashSet;
use syn::{
  braced,
  fold::Fold,
  parse::{Parse, ParseStream},
  parse_quote,
  punctuated::Punctuated,
  spanned::Spanned,
  token::{At, Bang, Brace, Colon, Comma},
  Expr, Ident, Macro, Path, Result as SynResult,
};

use crate::{
  declare_obj::DeclareObj,
  symbol_process::{kw, DollarRefs},
};

pub enum RdlBody {
  Literal(StructLiteral),
  /// Declare an expression as a object, like `rdl! { Widget::new(...) }`
  ExprObj(TokenStream),
}

/// Declare a object use struct literal, like `rdl! { Row { ... } }` or
/// `@parent { ... }`
pub struct StructLiteral {
  pub parent: RdlParent,
  pub brace: Brace,
  pub fields: Punctuated<DeclareField, Comma>,
  /// Declare a child in `rdl!` can use `rdl!` macro or `@` symbol.
  /// `rdl! { Row { rdl! { SizedBox {...} } } }`
  /// or
  /// `rdl! { Row { @ SizedBox{ ... } } }`
  /// but will be all processed as `rdl! { ... }`
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
  pub colon_tk: Option<Colon>,
  pub value: FieldValue,
}

pub struct FieldValue {
  refs: DollarRefs,
  /// field value
  value: Expr,
}

impl Parse for RdlBody {
  fn parse(input: ParseStream) -> SynResult<Self> {
    let fork = input.fork();
    if fork.parse::<RdlParent>().is_ok() && fork.peek(Brace) {
      Ok(RdlBody::Literal(input.parse()?))
    } else {
      let tokens = input.step(move |c| {
        let mut cursor = *c;

        let mut tts = vec![];
        while let Some((t, c)) = cursor.token_tree() {
          tts.push(t);
          cursor = c;
        }

        Ok((tts.into_iter().collect(), cursor))
      })?;
      Ok(RdlBody::ExprObj(tokens))
    }
  }
}

impl Parse for StructLiteral {
  fn parse(input: ParseStream) -> SynResult<Self> {
    let parent = input.parse()?;
    let content;
    let brace = braced!(content in input);
    let mut children = vec![];
    let mut fields = Punctuated::default();
    loop {
      if content.is_empty() {
        break;
      }

      if content.peek(At) || content.peek(kw::rdl) && content.peek2(Bang) {
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
        return Err(syn::Error::new(
          content.span(),
          "expected a field or a child.",
        ));
      }
    }

    check_duplicate_field(&fields)?;
    Ok(StructLiteral { parent, brace, fields, children })
  }
}

impl Parse for RdlParent {
  fn parse(input: ParseStream) -> SynResult<Self> {
    if input.peek(kw::_dollar_ಠ_ಠ) && input.peek2(Bang) {
      let mac: Macro = input.parse()?;
      Ok(RdlParent::Var(mac.parse_body()?))
    } else {
      Ok(RdlParent::Type(input.parse()?))
    }
  }
}

impl Parse for DeclareField {
  fn parse(input: ParseStream) -> SynResult<Self> {
    let member: Ident = input.parse()?;
    let colon_tk: Option<_> = input.parse()?;
    let value = if colon_tk.is_none() {
      parse_quote!(#member)
    } else {
      input.parse()?
    };

    Ok(DeclareField { member, colon_tk, value })
  }
}

impl Parse for FieldValue {
  fn parse(input: ParseStream) -> SynResult<Self> {
    let mut refs = DollarRefs::default();
    let value = refs.fold_expr(input.parse()?);
    refs.dedup();
    Ok(FieldValue { refs, value })
  }
}

impl ToTokens for RdlBody {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    match self {
      RdlBody::Literal(l) => match DeclareObj::from_literal(l) {
        Ok(declare) => declare.to_tokens(tokens),
        Err(err) => err.to_tokens(tokens),
      },
      RdlBody::ExprObj(e) => Brace(e.span()).surround(tokens, |tokens| e.to_tokens(tokens)),
    }
  }
}

impl ToTokens for DeclareField {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    let DeclareField { member, value, .. } = self;
    quote_spanned! {value.span()=> .#member(#value)}.to_tokens(tokens);
  }
}

impl ToTokens for FieldValue {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    let Self { refs, value } = self;
    if !refs.is_empty() {
      Brace(value.span()).surround(tokens, |tokens| {
        refs.to_tokens(tokens);
        value.to_tokens(tokens);
      });
    } else {
      value.to_tokens(tokens);
    }
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
