use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote_spanned};

use crate::DeclareField;

pub enum Error<'a> {
  InvalidFieldInVar(Box<[&'a DeclareField]>),
  WatchNothing(Span),
  RdlAtSyntax { at: Span, follow: Option<Span> },
  IdentNotFollowDollar(Span),
}

impl<'a> Error<'a> {
  pub fn to_compile_error(&self) -> TokenStream {
    match self {
      Error::InvalidFieldInVar(fields) => {
        let mut tokens = TokenStream::new();
        for f in fields.iter() {
          quote_spanned! { f.member.span() =>
            compile_error!("Only allow to declare builtin fields in a variable parent.");
          }
          .to_tokens(&mut tokens);
        }
        tokens
      }
      Error::WatchNothing(span) => quote_spanned! { *span =>
        compile_error!("expression not subscribe anything, it must contain at least one $")
      },
      &Error::RdlAtSyntax { at, follow } => {
        let span = follow.and_then(|f| at.join(f)).unwrap_or(at);
        quote_spanned! { span => compile_error!("Syntax Error: use `@` to declare object, must be: \n 1. `@ XXX { ... }`, \
        declare a new `XXX` type object;\n 2. `@ $parent { ... }`, declare a \
        variable as parent;\n 3. `@ { ... } `, declare an object by an expression.") }
      }
      Error::IdentNotFollowDollar(span) => {
        quote_spanned! { *span => compile_error!("Syntax error: expected an identifier after `$`"); }
      }
    }
  }
}

pub type Result<'a, T> = std::result::Result<T, Error<'a>>;

pub fn result_to_token_stream<T: ToTokens>(res: Result<T>) -> TokenStream {
  match res {
    Ok(value) => value.to_token_stream(),
    Err(err) => err.to_compile_error(),
  }
}
