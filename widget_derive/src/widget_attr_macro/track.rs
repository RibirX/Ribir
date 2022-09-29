use super::kw;
use proc_macro2::Ident;
use quote::{quote_spanned, ToTokens};
use syn::{
  braced,
  parse::Parse,
  punctuated::Punctuated,
  spanned::Spanned,
  token::{self, Comma},
  Expr,
};

#[derive(Debug)]
pub struct SimpleField {
  pub(crate) member: Ident,
  pub(crate) colon_token: Option<token::Colon>,
  pub(crate) expr: Expr,
}

pub struct Track {
  _track_token: kw::track,
  _brace: token::Brace,
  pub track_externs: Vec<SimpleField>,
}

impl Parse for Track {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    let content;

    let track = Track {
      _track_token: input.parse()?,
      _brace: braced!(content in input),
      track_externs: {
        let fields: Punctuated<SimpleField, Comma> =
          content.parse_terminated(SimpleField::parse)?;
        fields.into_iter().collect()
      },
    };
    Ok(track)
  }
}

impl ToTokens for Track {
  fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
    self
      .track_externs
      .iter()
      .filter(|f| f.colon_token.is_some())
      .for_each(|field| {
        let SimpleField { member, expr, .. } = field;
        tokens.extend(quote_spanned!(field.span() => let #member: Stateful<_> = #expr; ));
      });
  }
}

impl Track {
  pub fn has_def_names(&self) -> bool { self.track_externs.iter().any(|f| f.colon_token.is_some()) }

  pub fn track_names(&self) -> impl Iterator<Item = &Ident> {
    self.track_externs.iter().map(|f| &f.member)
  }
}
