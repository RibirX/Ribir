use crate::symbol_process::{CaptureRef, DollarRefs};
use quote::{quote, ToTokens};
use syn::{
  fold::Fold,
  parse::{Parse, ParseStream},
  Stmt,
};

pub(crate) struct PipeExpr {
  refs: DollarRefs,
  expr: Vec<Stmt>,
}

impl Parse for PipeExpr {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    let mut refs = DollarRefs::default();

    let stmts = syn::Block::parse_within(input)?;
    let stmts = stmts.into_iter().map(|s| refs.fold_stmt(s)).collect();
    if refs.is_empty() {
      let err = syn::Error::new(
        input.span(),
        "`pipe!` expression not subscribe anything, it must contain at least one $",
      );
      Err(err)
    } else {
      refs.dedup();
      Ok(Self { refs, expr: stmts })
    }
  }
}

impl ToTokens for PipeExpr {
  fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
    let Self { refs, expr } = self;

    let captures = refs.capture_state_tokens();
    let upstream = refs.upstream_tokens();
    let capture_refs = refs.iter().map(CaptureRef);

    if refs.used_ctx() {
      quote! {{
        #captures
        let upstream = #upstream;
        let mut expr_value = move |ctx!(): &BuildCtx<'_>| {
          #(#capture_refs)*
          #(#expr)*
        };
        let _ctx_handle = ctx!().handle();

        Pipe::new(
          expr_value(ctx!()),
          upstream
            .filter_map(move |_| _ctx_handle.with_ctx(&mut expr_value))
            .box_it()
        )
      }}
      .to_tokens(tokens)
    } else {
      quote! {{
        #captures
        let upstream = #upstream;
        let mut expr_value = move || {
          #(#capture_refs)*
          #(#expr)*
        };
        Pipe::new(
          expr_value(),
          upstream.map(move |_| expr_value()).box_it()
        )
      }}
      .to_tokens(tokens)
    }
  }
}
