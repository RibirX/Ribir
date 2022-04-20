use super::kw;
use proc_macro2::Ident;
use syn::{braced, parse::Parse, punctuated::Punctuated, spanned::Spanned, token};

pub struct Track {
  track_token: kw::track,
  brace: token::Brace,
  pub track_externs: Punctuated<NameAlias, token::Comma>,
}

pub struct NameAlias {
  _name: Ident,
  _colon_token: Option<token::Colon>,
  alias: Ident,
}

impl Parse for Track {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    let content;
    let track = Track {
      track_token: input.parse()?,
      brace: braced!(content in input),
      track_externs: content.parse_terminated(NameAlias::parse)?,
    };
    Ok(track)
  }
}

impl Parse for NameAlias {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    let _name: Ident = input.parse()?;
    let _colon_token: Option<token::Colon> = input.parse()?;
    let alias = if _colon_token.is_some() {
      _name.clone()
    } else {
      input.parse()?
    };
    Ok(Self { _name, _colon_token, alias })
  }
}

impl Spanned for Track {
  fn span(&self) -> proc_macro2::Span { self.track_token.span().join(self.brace.span).unwrap() }
}

impl Track {
  pub fn track_names(&self) -> impl Iterator<Item = &Ident> {
    self.track_externs.iter().map(|f| &f.alias)
  }
}
