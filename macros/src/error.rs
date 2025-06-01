use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote_spanned};

pub enum Error {
  InvalidFieldInVar(Box<[Span]>),
  WatchNothing(Span),
  RdlAtSyntax { at: Span, follow: Option<Span> },
  IdentNotFollowDollar(Span),
  Syn(syn::Error),
}

impl Error {
  pub fn to_compile_error(&self) -> TokenStream {
    match self {
      Error::InvalidFieldInVar(fields) => Self::invalid_field_in_var_error(fields),
      Error::WatchNothing(span) => Self::watch_nothing_error(*span),
      &Error::RdlAtSyntax { at, follow } => Self::rdl_at_syntax_error(at, follow),
      Error::IdentNotFollowDollar(span) => Self::ident_not_follow_dollar_error(*span),
      Error::Syn(err) => err.to_compile_error(),
    }
  }

  fn invalid_field_in_var_error(fields: &[Span]) -> TokenStream {
    let mut tokens = TokenStream::new();
    let error_msg = "Only built-in fields are allowed in variable parent declarations.";

    for span in fields {
      quote_spanned! { *span => compile_error!(#error_msg); }.to_tokens(&mut tokens);
    }
    tokens
  }

  fn watch_nothing_error(span: Span) -> TokenStream {
    let error_msg =
      "Expression does not subscribe to anything. It must contain at least one '$' symbol.";
    quote_spanned! { span => compile_error!(#error_msg) }
  }

  fn rdl_at_syntax_error(at: Span, follow: Option<Span>) -> TokenStream {
    let span = follow.and_then(|f| at.join(f)).unwrap_or(at);
    let error_msg = "Syntax error: Invalid use of '@'. Valid forms are:\n1. `@TypeName { ... }` - \
                     Declare a new object of type `TypeName`\n2. `@(parent_expr) { ... }` - \
                     Declare with an expression as parent\n3. `@ { ... }` - Declare an object \
                     using an expression";

    quote_spanned! { span => compile_error!(#error_msg) }
  }

  fn ident_not_follow_dollar_error(span: Span) -> TokenStream {
    let error_msg = "Syntax error: Expected identifier after '$'";
    quote_spanned! { span => compile_error!(#error_msg) }
  }
}

pub type Result<T> = std::result::Result<T, Error>;

pub fn result_to_token_stream<T: ToTokens>(res: Result<T>) -> TokenStream {
  match res {
    Ok(value) => value.to_token_stream(),
    Err(err) => err.to_compile_error(),
  }
}

impl From<syn::Error> for Error {
  fn from(value: syn::Error) -> Self { Error::Syn(value) }
}
