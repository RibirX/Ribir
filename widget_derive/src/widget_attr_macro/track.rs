use super::{kw, widget_def_variable};
use proc_macro2::Ident;
use quote::{quote_spanned, ToTokens};
use syn::{braced, parse::Parse, punctuated::Punctuated, token};

pub struct Track {
  _track_token: kw::track,
  _brace: token::Brace,
  pub track_externs: Punctuated<NameAlias, token::Comma>,
}

pub struct NameAlias {
  name: Ident,
  _colon_token: Option<token::Colon>,
  alias: Ident,
}

impl Parse for Track {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    let content;
    let track = Track {
      _track_token: input.parse()?,
      _brace: braced!(content in input),
      track_externs: content.parse_terminated(NameAlias::parse)?,
    };
    Ok(track)
  }
}

impl Parse for NameAlias {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    let name: Ident = input.parse()?;
    let _colon_token: Option<token::Colon> = input.parse()?;
    let alias = if _colon_token.is_some() {
      input.parse()?
    } else {
      name.clone()
    };
    Ok(Self { name, _colon_token, alias })
  }
}

impl ToTokens for Track {
  fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
    self
      .track_externs
      .iter()
      .for_each(|NameAlias { name, alias, .. }| {
        let def_name = widget_def_variable(alias);
        tokens.extend(quote_spanned!(alias.span() => let #def_name: Stateful<_> = #name.clone(); ));
      });
  }
}

impl Track {
  pub fn track_names(&self) -> impl Iterator<Item = &Ident> {
    self.track_externs.iter().map(|f| &f.alias)
  }
}
