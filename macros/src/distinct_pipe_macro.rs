use proc_macro2::TokenStream;
use quote::quote;

use crate::{pipe_macro, symbol_process::DollarRefsCtx};

pub fn gen_code(input: TokenStream, refs_ctx: &mut DollarRefsCtx) -> proc_macro::TokenStream {
  let mut tokens = pipe_macro::gen_code(input, refs_ctx);
  tokens.extend(quote! {
    .value_chain(|s| s.distinct_until_changed().box_it())
  });
  tokens.into()
}
