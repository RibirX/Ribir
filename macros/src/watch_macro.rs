use crate::{
  ok,
  pipe_macro::BodyExpr,
  symbol_process::{not_subscribe_anything, symbol_to_macro, DollarRefsCtx, DollarRefsScope},
};
use proc_macro::TokenStream as TokenStream1;
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{fold::Fold, parse_macro_input, spanned::Spanned, Stmt};

pub(crate) struct WatchMacro {
  refs: DollarRefsScope,
  expr: Vec<Stmt>,
}

impl WatchMacro {
  pub fn gen_code(input: TokenStream, refs_ctx: &mut DollarRefsCtx) -> TokenStream1 {
    let span = input.span();
    let input = ok!(symbol_to_macro(TokenStream1::from(input)));
    let expr = parse_macro_input! { input as BodyExpr };
    refs_ctx.new_dollar_scope(true);
    let expr = expr.0.into_iter().map(|s| refs_ctx.fold_stmt(s)).collect();
    let refs = refs_ctx.pop_dollar_scope(true);
    if refs.is_empty() {
      not_subscribe_anything(span).into()
    } else {
      WatchMacro { refs, expr }.to_token_stream().into()
    }
  }
}

impl ToTokens for WatchMacro {
  fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
    let Self { refs, expr, .. } = self;

    let upstream = refs.upstream_tokens();

    if refs.used_ctx() {
      quote! {{
        #refs
        let _ctx_handle_ಠ_ಠ = ctx!().handle();
        #upstream
          .map(move |_| _ctx_handle_ಠ_ಠ.with_ctx(|ctx!(): &BuildCtx<'_>| { #(#expr)* }))
      }}
      .to_tokens(tokens)
    } else {
      quote! {{
        #refs
        #upstream.map(move |_| { #(#expr)* })
      }}
      .to_tokens(tokens)
    }
  }
}
