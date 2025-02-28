use proc_macro2::TokenStream;
use quote::quote;

use crate::{
  error::result_to_token_stream,
  symbol_process::{DollarRefsCtx, symbol_to_macro},
  watch_macro::BodyExpr,
};

pub(crate) fn gen_code(input: TokenStream, ctx: Option<&mut DollarRefsCtx>) -> TokenStream {
  let res = symbol_to_macro(input).and_then(|input| {
    let body = syn::parse2::<BodyExpr>(input)?;
    let (stmts, refs) = if let Some(ctx) = ctx {
      ctx.new_dollar_scope(None);
      let stmts = body.fold(ctx).0;
      let refs = ctx.pop_dollar_scope(false);
      (stmts, refs)
    } else {
      let mut ctx = DollarRefsCtx::top_level();
      let stmts = body.fold(&mut ctx).0;
      let mut refs = ctx.pop_dollar_scope(false);
      refs.keep_only_builtin_refs();

      (stmts, refs)
    };
    if !refs.is_empty() {
      Ok(quote! {{
        #refs
        move || -> Widget { #(#stmts)*.into_widget() }
      }})
    } else {
      Ok(quote! { move || -> Widget { #(#stmts)*.into_widget() }})
    }
  });

  result_to_token_stream(res)
}
