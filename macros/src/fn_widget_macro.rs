use quote::{quote, ToTokens};
use syn::{
  fold::Fold,
  parse::{Parse, ParseStream},
  Stmt,
};

use crate::symbol_process::DollarRefs;

pub struct FnWidgetMacro {
  stmts: Vec<Stmt>,
}

impl Parse for FnWidgetMacro {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    let stmts = syn::Block::parse_within(input)?;
    let mut refs = DollarRefs::default();
    let stmts = stmts.into_iter().map(|s| refs.fold_stmt(s)).collect();
    Ok(Self { stmts })
  }
}

impl ToTokens for FnWidgetMacro {
  fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
    let Self { stmts } = self;
    quote! {
      FnWidget::new(move |ctx: &BuildCtx| {
        set_build_ctx!(ctx);
        #[allow(unused_mut)]
        { #(#stmts)* }
      })
    }
    .to_tokens(tokens)
  }
}
