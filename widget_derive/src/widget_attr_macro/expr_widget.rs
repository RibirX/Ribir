use proc_macro2::TokenStream;
use quote::{quote_spanned, ToTokens};
use syn::{
  parse::{discouraged::Speculative, Parse, ParseStream},
  spanned::Spanned,
  visit_mut::VisitMut,
};

use super::{
  capture_widget, declare_widget::macro_wrap_declare_keyword, kw, widget_state_ref, DeclareCtx,
  FollowOn,
};

#[derive(Debug)]
pub struct ExprWidget {
  expr_widget_token: kw::ExprWidget,
  expr: syn::ExprBlock,
  pub follows: Option<Vec<FollowOn>>,
}

impl Parse for ExprWidget {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    let _expr_child = input.parse()?;
    let wrap_fork = input.fork();
    let expr = if let Some(tokens) =
      wrap_fork.step(|step_cursor| Ok(macro_wrap_declare_keyword(*step_cursor)))?
    {
      input.advance_to(&wrap_fork);
      syn::parse2(tokens.into_iter().collect())?
    } else {
      input.parse()?
    };

    Ok(ExprWidget {
      expr_widget_token: _expr_child,
      expr,
      follows: None,
    })
  }
}

impl ToTokens for ExprWidget {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    if let Some(follows) = self.follows.as_ref() {
      let refs = follows.iter().map(|f| widget_state_ref(&f.widget));
      let captures = follows.iter().map(|f| capture_widget(&f.widget));
      let expr = &self.expr;
      tokens.extend(quote_spanned! { self.expr_widget_token.span() => {
        #(#captures)*
        #(#refs)*
        #expr
      }});
    } else {
      self.expr.to_tokens(tokens)
    }
  }
}

impl DeclareCtx {
  pub fn visit_expr_widget_mut(&mut self, expr_widget: &mut ExprWidget) {
    self.visit_expr_block_mut(&mut expr_widget.expr)
  }
}
