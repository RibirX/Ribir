use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::{fold::Fold, spanned::Spanned};

use crate::{
  error::{Error, Result, result_to_token_stream},
  pipe_macro::BodyExpr,
  symbol_process::{DollarRefsCtx, symbol_to_macro},
};

pub fn gen_code(input: TokenStream, refs_ctx: &mut DollarRefsCtx) -> TokenStream {
  let span = input.span();
  let res = process_watch_body(input, refs_ctx)
    .map(|(upstream, map_handler)| quote_spanned! { span => #upstream.map(#map_handler) });
  result_to_token_stream(res)
}

pub fn process_watch_body(
  input: TokenStream, refs_ctx: &mut DollarRefsCtx,
) -> Result<(TokenStream, TokenStream)> {
  let span = input.span();
  let input = symbol_to_macro(input)?;
  let body = syn::parse2::<BodyExpr>(input)?;

  refs_ctx.new_dollar_scope(None);
  let expr = body
    .0
    .into_iter()
    .map(|s| refs_ctx.fold_stmt(s))
    .collect::<Vec<_>>();
  let refs = refs_ctx.pop_dollar_scope(true);
  let map_handler = quote! { move |_: ModifyScope| { #(#expr)* } };
  if refs.is_empty() {
    Err(Error::WatchNothing(span))
  } else {
    let upstream = refs.upstream_tokens();
    Ok((upstream, quote! {{ #refs  #map_handler }}))
  }
}
