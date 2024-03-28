use proc_macro::TokenStream as TokenStream1;
use proc_macro2::TokenStream;
use quote::quote_spanned;
use syn::{
  fold::Fold,
  parse::{Parse, ParseStream},
  parse_macro_input,
  spanned::Spanned,
  Stmt,
};

use crate::{
  ok,
  symbol_process::{not_subscribe_anything, symbol_to_macro, DollarRefsCtx},
};

pub(crate) struct BodyExpr(pub(crate) Vec<Stmt>);

pub fn gen_code(input: TokenStream, refs_ctx: &mut DollarRefsCtx) -> TokenStream1 {
  let span = input.span();
  let input = ok!(symbol_to_macro(TokenStream1::from(input)));
  let expr = parse_macro_input! { input as BodyExpr };

  refs_ctx.new_dollar_scope(true);
  let expr = expr
    .0
    .into_iter()
    .map(|s| refs_ctx.fold_stmt(s))
    .collect::<Vec<Stmt>>();
  let refs = refs_ctx.pop_dollar_scope(true, true);
  if refs.is_empty() {
    not_subscribe_anything(span)
  } else {
    let upstream = refs.upstream_tokens();

    if refs.used_ctx() {
      quote_spanned! {span =>
        MapPipe::new(
          ModifiesPipe::new(#upstream.filter(|s| s.contains(ModifyScope::FRAMEWORK)).box_it()),
          {
            #refs
            let _ctx_handle_ಠ_ಠ = ctx!().handle();
            move |_: ModifyScope| {
              _ctx_handle_ಠ_ಠ
                .with_ctx(|ctx!(): &BuildCtx<'_>| { #(#expr)* })
                .expect("ctx is not available")
            }
          }
        )
      }
    } else {
      quote_spanned! {span =>
        MapPipe::new(
          ModifiesPipe::new(#upstream.box_it()),
          {
            #refs
            move |_: ModifyScope| { #(#expr)* }
          }
        )
      }
    }
  }
  .into()
}

impl Parse for BodyExpr {
  fn parse(input: ParseStream) -> syn::Result<Self> { Ok(Self(syn::Block::parse_within(input)?)) }
}
