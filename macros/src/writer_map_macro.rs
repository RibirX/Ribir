use proc_macro::{TokenStream as TokenStream1, TokenTree};
use proc_macro2::{Ident, Span, TokenStream};
use quote::{quote, quote_spanned, ToTokens};
use syn::{
  fold::Fold,
  parse::{Parse, ParseStream},
  parse_macro_input, parse_quote,
  spanned::Spanned,
  Expr, ExprMacro, Result,
};

use crate::{
  ok,
  symbol_process::{symbol_to_macro, DollarRefsCtx},
};

pub fn gen_map_path_writer(input: TokenStream, refs_ctx: &mut DollarRefsCtx) -> TokenStream1 {
  gen_path_partial_writer(input, "map_writer", refs_ctx)
}

pub fn gen_split_path_writer(input: TokenStream, refs_ctx: &mut DollarRefsCtx) -> TokenStream1 {
  gen_path_partial_writer(input, "split_writer", refs_ctx)
}

fn gen_path_partial_writer(
  input: TokenStream, method_name: &'static str, refs_ctx: &mut DollarRefsCtx,
) -> TokenStream1 {
  fn first_dollar_err(span: Span) -> TokenStream1 {
    quote_spanned! { span =>
      compile_error!("expected `$` as the first token, and only one `$` is allowed")
    }
    .into()
  }

  let mut input = TokenStream1::from(input).into_iter();
  let first = input.next();
  let is_first_dollar = first
    .as_ref()
    .map_or(false, |f| matches!(f, TokenTree::Punct(p) if p.as_char() == '$'));
  if !is_first_dollar {
    first_dollar_err(
      first
        .as_ref()
        .map_or(Span::call_site(), |t| t.span().into()),
    );
  }

  let input = ok!(symbol_to_macro(first.into_iter().chain(input)));

  let expr = parse_macro_input! { input as Expr };
  // Although `split_writer!` and `map_writer!` are not a capture scope, but we
  // start a new capture scope to ensure found the dollar in the macro. We will
  // not use the result of the `$var`, so it ok.
  refs_ctx.new_dollar_scope(true);
  let expr = refs_ctx.fold_expr(expr);
  let refs = refs_ctx.pop_dollar_scope(true);

  if refs.len() != 1 {
    quote_spanned! { expr.span() =>
      compile_error!("expected `$` as the first token, and only one `$` is allowed")
    }
    .into()
  } else {
    let dollar_ref = &refs[0];
    let host = if dollar_ref.builtin.is_some() {
      refs_ctx.builtin_host_tokens(dollar_ref)
    } else {
      dollar_ref.name.to_token_stream()
    };

    let path: RouterPath = parse_quote!(#expr);
    RouterMacro { host, path: path.0, method_name }
      .to_token_stream()
      .into()
  }
}

struct RouterPath(TokenStream);

impl Parse for RouterPath {
  fn parse(input: ParseStream) -> Result<Self> {
    input.parse::<ExprMacro>()?;
    Ok(Self(input.parse()?))
  }
}

struct RouterMacro {
  host: TokenStream,
  path: TokenStream,
  method_name: &'static str,
}

impl ToTokens for RouterMacro {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    let Self { host, path, method_name } = self;
    let method = Ident::new(method_name, Span::call_site());

    quote!(
      #host.#method(
        move |origin: &_| &origin #path,
        move |origin: &mut _| &mut origin #path
      )
    )
    .to_tokens(tokens)
  }
}
