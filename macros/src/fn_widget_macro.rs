use proc_macro2::TokenStream;
use quote::quote;
use syn::fold::Fold;

use crate::{
  error::result_to_token_stream,
  pipe_macro::BodyExpr,
  symbol_process::{DollarRefsCtx, symbol_to_macro},
};

pub(crate) fn gen_code(input: TokenStream, refs_ctx: &mut DollarRefsCtx) -> TokenStream {
  let res = symbol_to_macro(input)
    .and_then(|input| syn::parse2::<BodyExpr>(input).map_err(Into::into))
    .map(|body| {
      refs_ctx.new_dollar_scope(None);
      let stmts: Vec<_> = body
        .0
        .into_iter()
        .map(|s| refs_ctx.fold_stmt(s))
        .collect();

      refs_ctx.current_dollar_scope_mut().used_ctx = false;
      let _ = refs_ctx.pop_dollar_scope(false);
      quote! {
        move |ctx!(): &mut BuildCtx| -> Widget { #(#stmts)*.into_widget() }
      }
    });

  result_to_token_stream(res)
}
