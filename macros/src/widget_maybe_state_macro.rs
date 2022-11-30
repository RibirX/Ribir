use std::ops::BitAnd;

use proc_macro2::TokenStream;
use quote::{quote, quote_spanned, ToTokens};
use syn::{
  braced,
  parse::Parse,
  punctuated::Punctuated,
  token::{Brace, Comma, Paren},
};

use crate::widget_macro::StateField;

syn::custom_keyword!(maybe_states);
pub struct MaybeStates {
  _try_track_token: maybe_states,
  _brace: Brace,
  targets: Punctuated<StateField, Comma>,
  rest_tokens: TokenStream,
}

impl Parse for MaybeStates {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    let content;
    Ok(MaybeStates {
      _try_track_token: input.parse()?,
      _brace: braced!(content in input),
      targets: Punctuated::parse_separated_nonempty(&content)?,
      rest_tokens: input.parse()?,
    })
  }
}

impl ToTokens for MaybeStates {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    let mut init = quote! {};
    self.targets.iter().for_each(|f| {
      if f.colon_token.is_some() {
        let StateField { member, expr, .. } = f;
        init.extend(quote! { let #member = #expr; });
      }
    });
    if init.is_empty() {
      self.gen_widget_macro(tokens);
    } else {
      Brace::default().surround(tokens, |tokens| {
        init.to_tokens(tokens);
        self.gen_widget_macro(tokens);
      })
    }
  }
}

impl MaybeStates {
  fn gen_widget_macro(&self, tokens: &mut TokenStream) {
    let Self { targets, rest_tokens, .. } = self;
    if targets.len() == 1 {
      let name = &targets[0].member;
      tokens.extend(quote! {
        match #name {
          StateWidget::Stateful(#name) => widget!{
            states { #name }
            #rest_tokens
          },
          StateWidget::Stateless(#name) => widget!{ #rest_tokens }
        }
      });
    } else {
      let names = self.targets.iter().map(|f| &f.member);
      tokens.extend(quote! { match (#(#names),*) });
      Brace::default().surround(tokens, |tokens| self.tuple_match_arms(tokens));
    }
  }

  fn tuple_match_arms(&self, tokens: &mut TokenStream) {
    let Self { targets, rest_tokens, .. } = self;
    let count = targets.len();
    let arms = 2usize.pow(count as u32);
    for a in 0..arms {
      let mut stateful_names = vec![];
      Paren::default().surround(tokens, |tokens| {
        for b in 0..count {
          let name = &targets[b].member;
          if a.bitand(1 << b) > 0 {
            stateful_names.push(name);
            tokens.extend(quote_spanned! { name.span() =>  StateWidget::Stateful(#name),});
          } else {
            tokens.extend(quote_spanned! { name.span() => StateWidget::Stateless(#name), });
          }
        }
      });

      if stateful_names.is_empty() {
        tokens.extend(quote! { => widget! {
          #rest_tokens
        },});
      } else {
        tokens.extend(quote! { => widget! {
          states { #(#stateful_names),* }
          #rest_tokens
        },});
      }
    }
  }
}
