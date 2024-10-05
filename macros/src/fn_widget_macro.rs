use proc_macro::TokenStream as TokenStream1;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{fold::Fold, parse_macro_input};

use crate::{
  ok,
  pipe_macro::BodyExpr,
  symbol_process::{DollarRefsCtx, symbol_to_macro},
};

pub(crate) fn gen_code(input: TokenStream, refs_ctx: &mut DollarRefsCtx) -> TokenStream1 {
  let input = ok!(symbol_to_macro(TokenStream1::from(input)));
  let body = parse_macro_input!(input as BodyExpr);
  refs_ctx.new_dollar_scope(true);
  let stmts: Vec<_> = body
    .0
    .into_iter()
    .map(|s| refs_ctx.fold_stmt(s))
    .collect();

  refs_ctx.current_dollar_scope_mut().used_ctx = false;
  let _ = refs_ctx.pop_dollar_scope(true, false);
  quote! {
    move |ctx!(): &mut BuildCtx| -> Widget { #(#stmts)*.into_widget() }
  }
  .into()
}
