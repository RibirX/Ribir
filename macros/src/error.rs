use proc_macro2::TokenStream;
use quote::{quote_spanned, ToTokens};

use crate::DeclareField;

pub enum Error<'a> {
  InvalidFieldInVar(Box<[&'a DeclareField]>),
}

impl<'a> Error<'a> {
  pub fn to_compile_error(&self) -> TokenStream {
    match self {
      Self::InvalidFieldInVar(fields) => {
        let mut tokens = TokenStream::new();
        for f in fields.iter() {
          quote_spanned! { f.member.span() =>
            compile_error!("Only allow to declare builtin fields in a variable parent.");
          }
          .to_tokens(&mut tokens);
        }
        tokens
      }
    }
  }
}
