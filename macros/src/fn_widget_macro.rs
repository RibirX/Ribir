use crate::{ok, pipe_macro::BodyExpr, symbol_process::symbol_to_macro};
use proc_macro::TokenStream as TokenStream1;
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{fold::Fold, parse_macro_input, Stmt};

use crate::symbol_process::DollarRefsCtx;

pub struct FnWidgetMacro(Vec<Stmt>);

impl FnWidgetMacro {
  pub(crate) fn gen_code(input: TokenStream, refs_ctx: &mut DollarRefsCtx) -> TokenStream1 {
    let input = ok!(symbol_to_macro(TokenStream1::from(input)));
    let body = parse_macro_input!(input as BodyExpr);
    refs_ctx.new_dollar_scope(true);
    let stmts = body.0.into_iter().map(|s| refs_ctx.fold_stmt(s)).collect();
    let _ = refs_ctx.pop_dollar_scope(true);
    FnWidgetMacro(stmts).to_token_stream().into()
  }
}

impl ToTokens for FnWidgetMacro {
  fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
    let Self(stmts) = self;
    quote! {
      FnWidget::new(move |ctx!(): &BuildCtx| { #(#stmts)* })
    }
    .to_tokens(tokens)
  }
}
