use proc_macro2::TokenStream;
use quote::{ToTokens, quote_spanned};
use syn::{
  Expr, Ident,
  parse::{Parse, ParseStream},
  spanned::Spanned,
};

use crate::symbol_process::{DollarRef, DollarRefsCtx, DollarUsedInfo};

pub fn gen_code(input: TokenStream, refs_ctx: &mut DollarRefsCtx) -> TokenStream {
  match syn::parse2::<DollarMacro>(input) {
    Ok(dollar) => {
      refs_ctx.add_dollar_ref(dollar.dollar_ref());
      dollar.gen_code(refs_ctx)
    }
    Err(err) => err.to_compile_error(),
  }
}

pub(crate) struct DollarMacro {
  state_op: StateOp,
  state_expr: StateExpr,
}

enum StateOp {
  Read(read),
  Write(write),
  Reader(reader),
  Writer(writer),
  Watcher(watcher),
  Clone(clone),
}

#[derive(Debug, Clone)]
pub(crate) struct StateExpr {
  pub(crate) name: Ident,
  pub(crate) origin_state: Ident,
  pub(crate) origin_expr: OriginExpr,
}

#[derive(Hash, PartialEq, Eq, Debug, Clone)]
pub(crate) enum OriginExpr {
  Var(Ident),
  Expr(Expr),
}

syn::custom_keyword!(read);
syn::custom_keyword!(write);
syn::custom_keyword!(reader);
syn::custom_keyword!(writer);
syn::custom_keyword!(watcher);
syn::custom_keyword!(clone);

impl Parse for DollarMacro {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    let look = input.lookahead1();
    let state_op = if look.peek(read) {
      StateOp::Read(input.parse()?)
    } else if look.peek(write) {
      StateOp::Write(input.parse()?)
    } else if look.peek(reader) {
      StateOp::Reader(input.parse()?)
    } else if look.peek(writer) {
      StateOp::Writer(input.parse()?)
    } else if look.peek(watcher) {
      StateOp::Watcher(input.parse()?)
    } else if look.peek(clone) {
      StateOp::Clone(input.parse()?)
    } else {
      return Err(look.error());
    };

    let content;
    syn::parenthesized!(content in input);

    let fork = content.fork();
    // The origin variable of the state come from.
    let origin_state = fork.parse::<Ident>()?;

    let origin_expr = if fork.is_empty() {
      OriginExpr::Var(content.parse()?)
    } else {
      OriginExpr::Expr(content.parse()?)
    };

    let state_expr = StateExpr::new(origin_state, origin_expr);

    Ok(DollarMacro { state_op, state_expr })
  }
}

impl ToTokens for StateOp {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    match self {
      StateOp::Read(op) => quote_spanned! { op.span() => .#op() }.to_tokens(tokens),
      StateOp::Write(op) => quote_spanned! { op.span() => .#op() }.to_tokens(tokens),
      StateOp::Clone(op) => quote_spanned! { op.span() => .#op() }.to_tokens(tokens),
      StateOp::Reader(op) => quote_spanned! { op.span() => .clone_reader() }.to_tokens(tokens),
      StateOp::Writer(op) => quote_spanned! { op.span() => .clone_writer() }.to_tokens(tokens),
      StateOp::Watcher(op) => quote_spanned! { op.span() => .clone_watcher() }.to_tokens(tokens),
    }
  }
}

impl DollarMacro {
  fn gen_code(&self, ctx: &DollarRefsCtx) -> TokenStream {
    let Self { state_op, state_expr } = self;
    let mut tokens = TokenStream::new();
    if ctx.is_capture_var(&state_expr.origin_state) {
      state_expr.name.to_tokens(&mut tokens);
    } else {
      state_expr.origin_expr.to_tokens(&mut tokens);
    }
    state_op.to_tokens(&mut tokens);
    tokens
  }

  fn dollar_ref(&self) -> DollarRef {
    DollarRef { state_expr: self.state_expr.clone(), used: self.state_op.used_info() }
  }
}

impl StateOp {
  fn used_info(&self) -> DollarUsedInfo {
    match self {
      StateOp::Read(_) | StateOp::Reader(_) => DollarUsedInfo::Reader,
      StateOp::Write(_) | StateOp::Writer(_) => DollarUsedInfo::Writer,
      StateOp::Watcher(_) => DollarUsedInfo::Watcher,
      StateOp::Clone(_) => DollarUsedInfo::Clone,
    }
  }
}

impl ToTokens for OriginExpr {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    match self {
      OriginExpr::Var(var) => var.to_tokens(tokens),
      OriginExpr::Expr(expr) => expr.to_tokens(tokens),
    }
  }
}

impl StateExpr {
  pub(crate) fn new(origin_state: Ident, origin_expr: OriginExpr) -> Self {
    let name = Self::sanitize_identifier(&origin_expr.to_token_stream().to_string());
    let name = Ident::new(&name, origin_expr.span());

    StateExpr { origin_state, origin_expr, name }
  }

  fn sanitize_identifier(input: &str) -> String {
    input
      .to_lowercase()
      .chars()
      .map(|c| if c.is_alphanumeric() || c == '_' { c } else { 'ಠ' })
      .chain(['_', 'ಠ'])
      .collect()
  }
}
