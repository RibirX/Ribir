use proc_macro::TokenStream as TokenStream1;
use proc_macro2::TokenStream;
use quote::quote_spanned;
use syn::{fold::Fold, parse_macro_input, spanned::Spanned};

use crate::{
  ok,
  pipe_macro::BodyExpr,
  symbol_process::{not_subscribe_anything, symbol_to_macro, DollarRefsCtx},
};

pub fn gen_code(input: TokenStream, refs_ctx: &mut DollarRefsCtx) -> TokenStream1 {
  let span = input.span();
  let input = ok!(symbol_to_macro(TokenStream1::from(input)));
  let expr = parse_macro_input! { input as BodyExpr };
  refs_ctx.new_dollar_scope(true);
  let expr = expr
    .0
    .into_iter()
    .map(|s| refs_ctx.fold_stmt(s))
    .collect::<Vec<_>>();
  let refs = refs_ctx.pop_dollar_scope(true, true);
  if refs.is_empty() {
    not_subscribe_anything(span).into()
  } else {
    let upstream = refs.upstream_tokens();

    if refs.used_ctx() {
      quote_spanned! { span =>
        #upstream
        .map({
            #refs
            let _ctx_handle_ಠ_ಠ = ctx!().handle();
            move |_| _ctx_handle_ಠ_ಠ.with_ctx(|ctx!(): &BuildCtx<'_>| { #(#expr)* })
          })
      }
    } else {
      quote_spanned! { span =>
        #upstream.map({
          #refs
          move |_| { #(#expr)* }
        })
      }
    }
    .into()
  }
}
