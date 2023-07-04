use crate::{pipe_macro::fold_expr_as_in_closure, symbol_process::DollarRefs};
use quote::{quote, ToTokens};
use syn::{
  parse::{Parse, ParseStream},
  Stmt,
};

pub(crate) struct WatchMacro {
  refs: DollarRefs,
  expr: Vec<Stmt>,
}

impl Parse for WatchMacro {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    let (refs, stmts) = fold_expr_as_in_closure(input)?;
    Ok(Self { refs, expr: stmts })
  }
}

impl ToTokens for WatchMacro {
  fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
    let Self { refs, expr } = self;

    let upstream = refs.upstream_tokens();

    if refs.used_ctx() {
      quote! {{
        #refs
        let _ctx_handle = ctx!().handle();
        #upstream
          .map(move |_| _ctx_handle.with_ctx(|ctx!(): &BuildCtx<'_>| { #(#expr)* }))
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
