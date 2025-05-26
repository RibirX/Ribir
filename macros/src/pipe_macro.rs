use proc_macro2::TokenStream;
use quote::quote_spanned;
use syn::spanned::Spanned;

use crate::{
  error::result_to_token_stream, symbol_process::DollarRefsCtx, watch_macro::process_watch_body,
};

pub fn gen_code(input: TokenStream, refs_ctx: Option<&mut DollarRefsCtx>) -> TokenStream {
  let span = input.span();
  let res = process_watch_body(input, refs_ctx).map(|(upstream, map_handler)| {
    quote_spanned! {span => Pipe::new(#upstream.box_it(), #map_handler) }
  });
  result_to_token_stream(res)
}
