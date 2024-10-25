use proc_macro2::TokenStream;
use quote::quote_spanned;
use syn::spanned::Spanned;

use crate::{
  error::result_to_token_stream, symbol_process::DollarRefsCtx, watch_macro::process_watch_body,
};

pub fn gen_code(input: TokenStream, refs_ctx: &mut DollarRefsCtx) -> TokenStream {
  let span = input.span();
  let res = process_watch_body(input, refs_ctx).map(|(upstream, map_handler)| {
    quote_spanned! {span =>
      MapPipe::new(
        ModifiesPipe::new(#upstream.box_it()),
        #map_handler
      )
      // Since the pipe has an initial value, we skip the initial notification.
      .value_chain(|s| s.distinct_until_key_changed(|v: &(_, _)| v.1).skip(1).box_it())
    }
  });
  result_to_token_stream(res)
}
