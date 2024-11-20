use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::{
  Stmt,
  fold::Fold,
  parse::{Parse, ParseStream},
  spanned::Spanned,
};

use crate::{
  error::{Error, Result, result_to_token_stream},
  symbol_process::{DollarRefsCtx, symbol_to_macro},
};

pub fn gen_code(input: TokenStream, refs_ctx: Option<&mut DollarRefsCtx>) -> TokenStream {
  let span = input.span();
  let res = process_watch_body(input, refs_ctx)
    .map(|(upstream, map_handler)| quote_spanned! { span => #upstream.map(#map_handler) });
  result_to_token_stream(res)
}

pub fn process_watch_body(
  input: TokenStream, refs_ctx: Option<&mut DollarRefsCtx>,
) -> Result<(TokenStream, TokenStream)> {
  let span = input.span();
  let input = symbol_to_macro(input)?;
  let mut body = syn::parse2::<BodyExpr>(input)?;

  let refs = if let Some(refs_ctx) = refs_ctx {
    refs_ctx.new_dollar_scope(None);
    body = body.fold(refs_ctx);
    refs_ctx.pop_dollar_scope(true)
  } else {
    let mut refs_ctx = DollarRefsCtx::top_level();
    body = body.fold(&mut refs_ctx);
    refs_ctx.pop_dollar_scope(true)
  };

  let expr = body.0;
  let map_handler = quote! { move |_: ModifyScope| { #(#expr)* } };
  if refs.is_empty() {
    Err(Error::WatchNothing(span))
  } else {
    let upstream = refs.upstream_tokens();
    Ok((upstream, quote! {{ #refs  #map_handler }}))
  }
}

pub(crate) struct BodyExpr(pub(crate) Vec<Stmt>);

impl Parse for BodyExpr {
  fn parse(input: ParseStream) -> syn::Result<Self> { Ok(Self(syn::Block::parse_within(input)?)) }
}

impl BodyExpr {
  pub fn fold(self, refs_ctx: &mut DollarRefsCtx) -> Self {
    Self(
      self
        .0
        .into_iter()
        .map(|s| refs_ctx.fold_stmt(s))
        .collect::<Vec<_>>(),
    )
  }
}
