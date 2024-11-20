use proc_macro2::TokenStream;
use quote::quote;

use crate::{
  error::result_to_token_stream,
  symbol_process::{DollarRefsCtx, symbol_to_macro},
  watch_macro::BodyExpr,
};

pub(crate) fn gen_code(input: TokenStream, refs_ctx: &mut DollarRefsCtx) -> TokenStream {
  let res = symbol_to_macro(input).and_then(|input| {
    let body = syn::parse2::<BodyExpr>(input)?;
    refs_ctx.new_dollar_scope(None);
    let stmts = body.fold(refs_ctx).0;
    let _ = refs_ctx.pop_dollar_scope(false);
    Ok(quote! {
      move || -> Widget { #(#stmts)*.into_widget() }
    })
  });

  result_to_token_stream(res)
}
