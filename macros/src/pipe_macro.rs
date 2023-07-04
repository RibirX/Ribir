use crate::symbol_process::DollarRefs;
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
    let (refs, stmts) = fold_expr_as_in_closure(input)?;
    Ok(Self { refs, expr: stmts })
  }
}

pub fn fold_expr_as_in_closure(input: ParseStream) -> syn::Result<(DollarRefs, Vec<Stmt>)> {
  let mut refs = DollarRefs::default();
  refs.in_capture += 1;
  let stmts = syn::Block::parse_within(input)?;
  let stmts = stmts.into_iter().map(|s| refs.fold_stmt(s)).collect();
  refs.in_capture -= 1;
  if refs.is_empty() {
    let err = syn::Error::new(
      input.span(),
      "expression not subscribe anything, it must contain at least one $",
    );
    Err(err)
  } else {
    refs.dedup();
    Ok((refs, stmts))
  }
}

impl ToTokens for PipeExpr {
  fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
    let Self { refs, expr } = self;

    let upstream = refs.upstream_tokens();

    if refs.used_ctx() {
      quote! {{
        #refs
        let upstream = #upstream;
        let mut expr_value = move |ctx!(): &BuildCtx<'_>| { #(#expr)* };
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
        #refs
        let upstream = #upstream;
        let mut expr_value = move || { #(#expr)* };
        Pipe::new(
          expr_value(),
          upstream.map(move |_| expr_value()).box_it()
        )
      }}
      .to_tokens(tokens)
    }
  }
}
