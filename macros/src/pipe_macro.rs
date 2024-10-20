use proc_macro2::TokenStream;
use quote::quote_spanned;
use syn::{
  Stmt,
  parse::{Parse, ParseStream},
  spanned::Spanned,
};

use crate::{
  error::result_to_token_stream, symbol_process::DollarRefsCtx, watch_macro::process_watch_body,
};

pub(crate) struct BodyExpr(pub(crate) Vec<Stmt>);

pub fn gen_code(input: TokenStream, refs_ctx: &mut DollarRefsCtx) -> TokenStream {
  let span = input.span();
  let res = process_watch_body(input, refs_ctx).map(|(upstream, map_handler)| {
    quote_spanned! {span =>
    MapPipe::new(
      // Since the pipe has an initial value, we skip the initial notification.
      ModifiesPipe::new(#upstream.skip(1).box_it()),
      #map_handler
    )}
  });
  result_to_token_stream(res)
}

impl Parse for BodyExpr {
  fn parse(input: ParseStream) -> syn::Result<Self> { Ok(Self(syn::Block::parse_within(input)?)) }
}
