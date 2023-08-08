use crate::symbol_process::{not_subscribe_anything, DollarRefsCtx, DollarRefsScope};
use crate::{ok, symbol_process::symbol_to_macro};
use proc_macro::TokenStream as TokenStream1;
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::fold::Fold;
use syn::{
  parse::{Parse, ParseStream},
  parse_macro_input,
  spanned::Spanned,
  Stmt,
};

pub(crate) struct BodyExpr(pub(crate) Vec<Stmt>);
pub(crate) struct PipeMacro {
  refs: DollarRefsScope,
  expr: Vec<Stmt>,
}

impl PipeMacro {
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
      PipeMacro { refs, expr }.to_token_stream().into()
    }
  }
}

impl Parse for BodyExpr {
  fn parse(input: ParseStream) -> syn::Result<Self> { Ok(Self(syn::Block::parse_within(input)?)) }
}

impl ToTokens for PipeMacro {
  fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
    let Self { refs, expr } = self;

    let upstream = refs.upstream_tokens();

    if refs.used_ctx() {
      quote! {{
        #refs
        let upstream = #upstream;
        let _ctx_handle_ಠ_ಠ = ctx!().handle();
        let mut expr_value = move |ctx!(): &BuildCtx<'_>| { #(#expr)* };
        Pipe::new(
          expr_value(ctx!()),
          upstream
            .filter_map(move |scope| _ctx_handle_ಠ_ಠ
              .with_ctx(&mut expr_value)
              .map(|v| (scope, v))
            )
            .box_it()
        )
      }}
      .to_tokens(tokens)
    } else {
      quote! {{
        #refs
        let upstream = #upstream;
        let mut expr_value = move || { #(#expr)* };
        Pipe::new(
          expr_value(),
          upstream.map(move |scope| (scope, expr_value())).box_it()
        )
      }}
      .to_tokens(tokens)
    }
  }
}
