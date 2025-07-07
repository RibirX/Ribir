use proc_macro2::TokenStream;
use quote::{ToTokens, quote_spanned};
use syn::{
  Stmt,
  fold::Fold,
  parse::{Parse, ParseStream},
  spanned::Spanned,
};

use crate::{dollar_macro::OriginExpr, error::*, symbol_process::*};

/// Represents the processed watch body containing upstream dependencies and
/// mapping handler
pub struct WatchBody {
  pub upstream: Upstreams,
  pub map_handler: TokenStream,
}

/// Wrapper for a block of statements that can be parsed from a TokenStream
pub(crate) struct BodyExpr(pub(crate) Vec<Stmt>);

impl Parse for BodyExpr {
  fn parse(input: ParseStream) -> syn::Result<Self> { Ok(Self(syn::Block::parse_within(input)?)) }
}

impl BodyExpr {
  pub fn fold(self, refs_ctx: &mut DollarRefsCtx) -> Self {
    Self(
      self
        .0
        .into_iter()
        .map(|stmt| refs_ctx.fold_stmt(stmt))
        .collect(),
    )
  }
}

/// Public interface for processing watch bodies with optional context
pub fn process_watch_body(
  input: TokenStream, refs_ctx: Option<&mut DollarRefsCtx>,
) -> Result<WatchBody> {
  let span = input.span();

  let input = symbol_to_macro(input)?;
  let body = syn::parse2::<BodyExpr>(input)?;
  let expr;
  let refs = if let Some(ctx) = refs_ctx {
    ctx.new_dollar_scope(None);
    expr = body.fold(ctx).0;
    ctx.pop_dollar_scope(true)
  } else {
    let mut ctx = DollarRefsCtx::top_level();
    expr = body.fold(&mut ctx).0;
    ctx.pop_dollar_scope(true)
  };

  if refs.is_state_empty() {
    Err(Error::WatchNothing(span))
  } else {
    let map_handler = quote_spanned! { span => {
      #refs
      move |_: ModifyInfo| { #(#expr)* }
    }};

    Ok(WatchBody { upstream: Upstreams::new(refs), map_handler })
  }
}

/// Generates final output code from processed watch body
pub fn gen_code(input: TokenStream, refs_ctx: Option<&mut DollarRefsCtx>) -> TokenStream {
  let span = input.span();

  process_watch_body(input, refs_ctx)
    .map(|WatchBody { upstream, map_handler }| {
      quote_spanned! { span => #upstream.map(#map_handler)}
    })
    .unwrap_or_else(|e| e.to_compile_error())
}

//===================================================================
// Upstream Dependencies Handling
//===================================================================

/// Represents all upstream dependencies being watched
pub struct Upstreams(DollarRefsScope);

impl Upstreams {
  /// Creates new Upstream from validated scope (must not be empty)
  fn new(scope: DollarRefsScope) -> Self {
    assert!(!scope.is_state_empty());
    Self(scope)
  }
}

impl ToTokens for Upstreams {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    if self.0.len() == 1 {
      let modify = Modifies(&self.0[0]);
      quote_spanned! { modify.expr().span() =>
          observable::of(ModifyInfo::default()).merge(#modify)
      }
      .to_tokens(tokens);
    } else {
      let modifies_iter = self.0.iter().map(Modifies);
      let first_span = modifies_iter
        .clone()
        .next()
        .expect("should have at least one state")
        .expr()
        .span();

      quote_spanned! { first_span =>
          observable::of(ModifyInfo::default())
              .merge(observable::from_iter([#(#modifies_iter),*]).merge_all(usize::MAX))
      }
      .to_tokens(tokens);
    }
  }
}

/// Helper for generating modify calls on origin expressions
struct Modifies<'a>(&'a DollarRef);

impl<'a> Modifies<'a> {
  /// Gets the underlying origin expression with proper span information   
  fn expr(&self) -> &OriginExpr { &self.0.state_expr.origin_expr }
}

impl<'a> ToTokens for Modifies<'a> {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    let expr = self.expr();
    quote_spanned! { expr.span() => #expr.modifies() }.to_tokens(tokens);
  }
}
