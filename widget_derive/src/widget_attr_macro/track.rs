use super::{animations::SimpleField, kw};
use proc_macro2::Ident;
use quote::{quote_spanned, ToTokens};
use syn::{braced, parse::Parse, punctuated::Punctuated, spanned::Spanned, token};

pub struct Track {
  _track_token: kw::track,
  _brace: token::Brace,
  pub track_externs: Punctuated<SimpleField, token::Comma>,
}

impl Parse for Track {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    let content;
    let track = Track {
      _track_token: input.parse()?,
      _brace: braced!(content in input),
      track_externs: content.parse_terminated(SimpleField::parse)?,
    };
    Ok(track)
  }
}

impl ToTokens for Track {
  fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
    self.track_externs.iter().for_each(|field| {
      let SimpleField { member, expr, .. } = field;
      tokens.extend(quote_spanned!(field.span() => let #member: Stateful<_> = #expr; ));
    });
  }
}

impl Track {
  pub fn track_names(&self) -> impl Iterator<Item = &Ident> {
    self.track_externs.iter().map(|f| &f.member)
  }
}
