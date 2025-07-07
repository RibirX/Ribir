use std::cmp::Ordering;

use proc_macro2::*;
use quote::{ToTokens, quote_spanned};
use smallvec::{SmallVec, smallvec};
use syn::{Expr, Macro, Token, fold::Fold, parse_quote_spanned, spanned::Spanned};

use crate::{
  dollar_macro::{self, OriginExpr, StateExpr},
  fn_widget_macro,
  rdl_macro::RdlMacro,
};

pub const KW_DOLLAR: &str = "_dollar_ಠ_ಠ";
pub const KW_RDL: &str = "rdl";
pub const KW_PIPE: &str = "pipe";
pub const KW_DISTINCT_PIPE: &str = "distinct_pipe";
pub const KW_WATCH: &str = "watch";
pub const KW_PART_WRITER: &str = "part_writer";
pub const KW_PART_READER: &str = "part_reader";
pub const KW_PART_WATCHER: &str = "part_watcher";
pub const KW_FN_WIDGET: &str = "fn_widget";

pub use tokens_pre_process::*;

pub mod kw {
  syn::custom_keyword!(_dollar_ಠ_ಠ);
  syn::custom_keyword!(rdl);
}

#[derive(Hash, PartialEq, Eq, Debug, Clone)]
pub struct BuiltinInfo {
  pub(crate) host: Ident,
  pub(crate) get_builtin: Ident,
  pub(crate) run_before_clone: SmallVec<[Ident; 1]>,
}

#[derive(Hash, PartialEq, Eq, Debug, Clone, PartialOrd, Ord)]
pub enum DollarUsedInfo {
  Reader,
  Watcher,
  Writer,
  Clone,
}

#[derive(Debug, Clone)]
pub struct DollarRef {
  pub state_expr: StateExpr,
  pub used: DollarUsedInfo,
}

#[derive(Debug)]
pub struct DollarRefsCtx {
  scopes: SmallVec<[DollarRefsScope; 1]>,
  variable_stacks: Vec<Vec<Ident>>,
}

#[derive(Debug, Default)]
pub struct DollarRefsScope {
  refs: SmallVec<[DollarRef; 1]>,
  /// The index of the head of this scope in the variable stack.
  variable_stack_head: usize,
  /// This scope will exclusively capture the specified variable and will not be
  /// treated as a real scope. If set to `None`, it will capture all dollar
  /// references used.
  // todo: remove it
  only_capture: Option<Ident>,
}

pub struct StackGuard<'a>(&'a mut DollarRefsCtx);

mod tokens_pre_process {
  use std::iter::Peekable;

  use super::*;
  use crate::{error::*, symbol_process::KW_RDL};

  /// Convert `@` and `$` symbols into valid Rust macros (`rdl!` or
  /// `_dollar_ಠ_ಠ!`)
  pub fn symbol_to_macro(input: TokenStream) -> Result<TokenStream> {
    let mut iter = input.into_iter().peekable();
    let mut tokens = Vec::new();

    while let Some(token) = iter.next() {
      match token {
        // Handle @ symbol for widget declarations
        TokenTree::Punct(at)
          if at.as_char() == '@' && !matches!(tokens.last(), Some(TokenTree::Ident(_))) =>
        {
          let at_group = parse_at_group(&mut iter, at.span())?;
          let span = tokens_span(&at_group);

          tokens.push(TokenTree::Ident(Ident::new(KW_RDL, span)));
          tokens.push(not_token(span));

          if let Some(TokenTree::Group(_)) = at_group.last() {
            let rdl_group = TokenStream::from_iter(at_group);
            tokens.push(TokenTree::Group(Group::new(Delimiter::Brace, rdl_group)));
          } else {
            let follow = at_group.last().map(|t| t.span());
            return Err(Error::RdlAtSyntax { at: span, follow });
          }
        }

        // Handle $ symbol for dollar expressions
        TokenTree::Punct(dollar) if dollar.as_char() == '$' => {
          let dollar_group = parse_dollar_group(&mut iter, dollar.span())?;

          tokens.push(TokenTree::Ident(Ident::new(KW_DOLLAR, dollar.span())));
          tokens.push(not_token(dollar.span()));
          let span = dollar_group.span();
          let mut g = Group::new(Delimiter::Parenthesis, dollar_group);
          g.set_span(span);
          tokens.push(TokenTree::Group(g));
        }

        // Recursively process groups
        TokenTree::Group(mut g) => {
          if !in_macro(&tokens) {
            let processed = symbol_to_macro(g.stream())?;
            let mut new_group = Group::new(g.delimiter(), processed);
            new_group.set_span(g.span());
            g = new_group;
          }
          tokens.push(TokenTree::Group(g));
        }

        // Default case: preserve other tokens
        other => tokens.push(other),
      }
    }

    Ok(tokens.into_iter().collect())
  }

  /// Parse tokens following an `@` symbol into a widget declaration group
  fn parse_at_group(
    iter: &mut Peekable<impl Iterator<Item = TokenTree>>, at_span: Span,
  ) -> Result<SmallVec<[TokenTree; 3]>> {
    let mut rdl_group = SmallVec::new();

    match iter.next() {
      Some(TokenTree::Group(g)) => match g.delimiter() {
        // Parenthesized expression widget: `@(expr) { ... }`
        Delimiter::Parenthesis => {
          rdl_group.push(TokenTree::Group(g));
          // Lookahead for brace group
          if iter
            .peek()
            .is_some_and(|t| matches!(t, TokenTree::Group(_)))
          {
            rdl_group.push(iter.next().unwrap());
          }
        }
        // Braced expression widget: `@ { ... }`
        Delimiter::Brace => rdl_group.push(TokenTree::Group(g)),
        // Invalid delimiter
        _ => return Err(Error::RdlAtSyntax { at: at_span, follow: Some(g.span()) }),
      },

      // Named widget declaration: `@WidgetName { ... }`
      mut token => {
        while let Some(t) = token.take() {
          let is_group = matches!(&t, TokenTree::Group(_));
          rdl_group.push(t);

          if is_group {
            break;
          }
          token = iter.next();
        }
      }
    }

    Ok(rdl_group)
  }

  fn parse_dollar_group(
    iter: &mut impl Iterator<Item = TokenTree>, dollar_span: Span,
  ) -> Result<TokenStream> {
    let state_name = match iter.next() {
      Some(TokenTree::Ident(ident)) => ident,
      Some(token) => return Err(Error::DollarSyntax(dollar_span.join(token.span()).unwrap())),
      None => return Err(Error::DollarSyntax(dollar_span)),
    };

    let group = match iter.next() {
      Some(TokenTree::Group(g)) => g,
      Some(token) => return Err(Error::DollarSyntax(dollar_span.join(token.span()).unwrap())),
      None => return Err(Error::DollarSyntax(dollar_span)),
    };

    Ok(TokenStream::from_iter([TokenTree::Ident(state_name), TokenTree::Group(group)]))
  }

  fn tokens_span(tokens: &[TokenTree]) -> Span {
    let start = tokens.first().unwrap().span();
    let end = tokens.last().unwrap().span();
    start.join(end).unwrap_or(start)
  }

  fn in_macro(tokens: &[TokenTree]) -> bool {
    let [.., TokenTree::Ident(_), TokenTree::Punct(p)] = tokens else {
      return false;
    };
    p.as_char() == '!'
  }

  fn not_token(span: Span) -> TokenTree {
    let mut t = Punct::new('!', Spacing::Alone);
    t.set_span(span);
    TokenTree::Punct(t)
  }
}

impl Fold for DollarRefsCtx {
  fn fold_block(&mut self, i: syn::Block) -> syn::Block {
    let mut this = self.push_code_stack();
    syn::fold::fold_block(&mut *this, i)
  }

  fn fold_expr_closure(&mut self, i: syn::ExprClosure) -> syn::ExprClosure {
    let mut this = self.push_code_stack();
    syn::fold::fold_expr_closure(&mut *this, i)
  }

  fn fold_item_const(&mut self, i: syn::ItemConst) -> syn::ItemConst {
    self.new_local_var(&i.ident);
    syn::fold::fold_item_const(self, i)
  }

  fn fold_local(&mut self, mut i: syn::Local) -> syn::Local {
    //  we fold right expression first, then fold pattern, because the `=` is a
    // right operator.
    i.init = i.init.map(|init| self.fold_local_init(init));
    i.pat = self.fold_pat(i.pat);
    i
  }

  fn fold_expr_block(&mut self, i: syn::ExprBlock) -> syn::ExprBlock {
    let mut this = self.push_code_stack();
    syn::fold::fold_expr_block(&mut *this, i)
  }

  fn fold_expr_for_loop(&mut self, i: syn::ExprForLoop) -> syn::ExprForLoop {
    let mut this = self.push_code_stack();
    syn::fold::fold_expr_for_loop(&mut *this, i)
  }

  fn fold_expr_loop(&mut self, i: syn::ExprLoop) -> syn::ExprLoop {
    let mut this = self.push_code_stack();
    syn::fold::fold_expr_loop(&mut *this, i)
  }

  fn fold_expr_if(&mut self, i: syn::ExprIf) -> syn::ExprIf {
    let mut this = self.push_code_stack();
    syn::fold::fold_expr_if(&mut *this, i)
  }

  fn fold_arm(&mut self, i: syn::Arm) -> syn::Arm {
    let mut this = self.push_code_stack();
    syn::fold::fold_arm(&mut *this, i)
  }

  fn fold_expr_unsafe(&mut self, i: syn::ExprUnsafe) -> syn::ExprUnsafe {
    let mut this = self.push_code_stack();
    syn::fold::fold_expr_unsafe(&mut *this, i)
  }

  fn fold_expr_while(&mut self, i: syn::ExprWhile) -> syn::ExprWhile {
    let mut this = self.push_code_stack();
    syn::fold::fold_expr_while(&mut *this, i)
  }

  fn fold_pat_ident(&mut self, i: syn::PatIdent) -> syn::PatIdent {
    self.new_local_var(&i.ident);
    syn::fold::fold_pat_ident(self, i)
  }

  fn fold_macro(&mut self, mut mac: Macro) -> Macro {
    if mac.path.is_ident(KW_DOLLAR) {
      mac.tokens = dollar_macro::gen_code(mac.tokens, self);
      mark_macro_expanded(&mut mac);
    } else if mac.path.is_ident(KW_WATCH) {
      mac.tokens = crate::watch_macro::gen_code(mac.tokens, Some(self));
      mark_macro_expanded(&mut mac);
    } else if mac.path.is_ident(KW_PART_WRITER) {
      mac.tokens = crate::part_state::gen_part_writer(mac.tokens, Some(self));
      mark_macro_expanded(&mut mac);
    } else if mac.path.is_ident(KW_PART_READER) {
      mac.tokens = crate::part_state::gen_part_reader(mac.tokens, Some(self));
      mark_macro_expanded(&mut mac);
    } else if mac.path.is_ident(KW_PART_WATCHER) {
      mac.tokens = crate::part_state::gen_part_watcher(mac.tokens, Some(self));
      mark_macro_expanded(&mut mac);
    } else if mac.path.is_ident(KW_PIPE) {
      mac.tokens = crate::pipe_macro::gen_code(mac.tokens, Some(self));
      mark_macro_expanded(&mut mac);
    } else if mac.path.is_ident(KW_DISTINCT_PIPE) {
      mac.tokens = crate::distinct_pipe_macro::gen_code(mac.tokens, Some(self));
      mark_macro_expanded(&mut mac);
    } else if mac.path.is_ident(KW_RDL) {
      mac.tokens = RdlMacro::gen_code(mac.tokens, Some(self));
      mark_macro_expanded(&mut mac);
    } else if mac.path.is_ident(KW_FN_WIDGET) {
      mac.tokens = fn_widget_macro::gen_code(mac.tokens, Some(self));
      mark_macro_expanded(&mut mac);
    } else {
      mac = syn::fold::fold_macro(self, mac);
    }
    mac
  }

  fn fold_expr(&mut self, i: Expr) -> Expr {
    match i {
      Expr::Closure(c) if c.capture.is_some() => {
        self.new_dollar_scope(None);
        let c = self.fold_expr_closure(c);
        let dollar_scope = self.pop_dollar_scope(false);

        if !dollar_scope.is_empty() {
          Expr::Verbatim(quote_spanned!(c.span() => {
            #dollar_scope

            #c
          }))
        } else {
          Expr::Closure(self.fold_expr_closure(c))
        }
      }
      _ => syn::fold::fold_expr(self, i),
    }
  }
}

fn mark_macro_expanded(mac: &mut Macro) {
  mac.path = parse_quote_spanned! { mac.path.span() => ribir_expanded_ಠ_ಠ };
}

impl ToTokens for DollarRefsScope {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    self.refs.iter().for_each(|d| d.to_tokens(tokens))
  }
}

impl ToTokens for DollarRef {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    let Self { state_expr, used } = self;
    let span = state_expr.origin_expr.span();

    Token![let](span).to_tokens(tokens);
    state_expr.name.to_tokens(tokens);
    Token![=](span).to_tokens(tokens);
    state_expr.origin_expr.to_tokens(tokens);
    Token![.](span).to_tokens(tokens);
    let method = match used {
      DollarUsedInfo::Reader => Ident::new("clone_reader", span),
      DollarUsedInfo::Watcher => Ident::new("clone_watcher", span),
      DollarUsedInfo::Writer => Ident::new("clone_writer", span),
      DollarUsedInfo::Clone => Ident::new("clone", span),
    };
    method.to_tokens(tokens);
    syn::token::Paren(span).surround(tokens, |_| {});
    Token![;](span).to_tokens(tokens);
  }
}

impl DollarRef {
  fn is_var_state(&self) -> bool { matches!(self.state_expr.origin_expr, OriginExpr::Var(_)) }
}

impl DollarRefsCtx {
  #[inline]
  pub fn top_level() -> Self { Self::default() }

  /// Begin a new scope to track dollar reference information.
  #[inline]
  pub fn new_dollar_scope(&mut self, only_capture: Option<Ident>) {
    let variable_stack_head = if only_capture.is_some() {
      self.current_dollar_scope().variable_stack_head
    } else {
      self.variable_stacks.push(vec![]);
      self.variable_stacks.len() - 1
    };

    self
      .scopes
      .push(DollarRefsScope { only_capture, refs: <_>::default(), variable_stack_head });
  }

  /// Pop the last dollar scope, and removes duplicate elements in it and make
  /// the builtin widget first. Keep the builtin reference before the host
  /// because if a obj both reference builtin widget and its host, the host
  /// reference may shadow the original.
  ///
  /// For example, this generate code not work:
  ///
  /// ```ignore
  /// let a = a.clone_reader();
  /// // the `a` is shadowed by the before `a` variable.
  /// let a_margin = a.get_margin_widget(ctx!());
  /// ```
  ///
  /// must generate `a_margin` first:
  ///
  /// ```ignore
  /// let a_margin = a.get_margin_widget(ctx!());
  /// let a = a.clone_reader();
  /// ```
  ///
  /// - **watch_scope**: A watch scope that should designate all readers as
  ///   watchers.
  pub fn pop_dollar_scope(&mut self, watch_scope: bool) -> DollarRefsScope {
    let mut scope = self.scopes.pop().expect("Unmatched scope");
    if scope.only_capture.is_none() {
      self.variable_stacks.pop();
    }

    // To maintain the order, ensure that the builtin widget precedes its host.
    // Otherwise, the host might only clone a reader that cannot create a
    // builtin widget.
    scope.refs.sort_by_key(|r| r.is_var_state());

    if !self.scopes.is_empty() {
      for r in scope.refs.iter_mut() {
        if self.is_capture_var(&r.state_expr.origin_state) {
          let mut c_r = r.clone();
          if watch_scope && c_r.used == DollarUsedInfo::Reader {
            c_r.used = DollarUsedInfo::Watcher;
          }
          self.add_dollar_ref(c_r);

          // If a expression state is captured by the parent scope and treated as a
          // regular variable, the child scope does not need to capture it from
          // the host variable.
          r.state_expr.origin_expr = OriginExpr::Var(r.state_expr.name.clone());
        }
      }
    }

    if let Some(c) = scope.only_capture.as_ref() {
      scope
        .refs
        .drain_filter(|r| &r.state_expr.origin_state != c);
    }
    scope
  }

  pub fn push_code_stack(&mut self) -> StackGuard<'_> {
    self.variable_stacks.push(vec![]);
    StackGuard(self)
  }

  fn new_local_var(&mut self, name: &Ident) {
    self
      .variable_stacks
      .last_mut()
      .unwrap()
      .push(name.clone())
  }

  pub fn add_dollar_ref(&mut self, dollar_ref: DollarRef) {
    // local variable is not a outside reference.
    if self.is_capture_var(&dollar_ref.state_expr.origin_state) {
      let scope = self.current_dollar_scope_mut();
      let r = scope
        .refs
        .iter_mut()
        .find(|v| v.state_expr.origin_expr == dollar_ref.state_expr.origin_expr);

      if let Some(r) = r {
        if r.used.cmp(&dollar_ref.used) == Ordering::Less {
          r.used = dollar_ref.used
        }
      } else {
        scope.refs.push(dollar_ref);
      }
    }
  }

  pub fn current_dollar_scope(&self) -> &DollarRefsScope {
    self.scopes.last().expect("no dollar refs scope")
  }

  pub fn current_dollar_scope_mut(&mut self) -> &mut DollarRefsScope {
    self
      .scopes
      .last_mut()
      .expect("no dollar refs scope")
  }

  fn is_only_capture(&self, name: &Ident) -> bool {
    self.current_dollar_scope().only_capture.as_ref() == Some(name)
  }

  pub(crate) fn is_capture_var(&self, name: &Ident) -> bool {
    self.is_only_capture(name) || !self.is_local_var(name)
  }

  fn is_local_var(&self, name: &Ident) -> bool {
    let head = self.current_dollar_scope().variable_stack_head;
    self.variable_stacks[head..]
      .iter()
      .any(|stack| stack.contains(name))
  }
}

impl DollarRefsScope {
  pub(crate) fn is_state_empty(&self) -> bool { self.state_refs().next().is_none() }

  fn state_refs(&self) -> impl Iterator<Item = &DollarRef> {
    self
      .refs
      .iter()
      .filter(|r| r.used != DollarUsedInfo::Clone)
  }
}

impl std::ops::Deref for DollarRefsScope {
  type Target = [DollarRef];
  fn deref(&self) -> &Self::Target { &self.refs }
}

impl<'a> std::ops::Deref for StackGuard<'a> {
  type Target = DollarRefsCtx;
  fn deref(&self) -> &Self::Target { self.0 }
}

impl<'a> std::ops::DerefMut for StackGuard<'a> {
  fn deref_mut(&mut self) -> &mut Self::Target { self.0 }
}

impl<'a> Drop for StackGuard<'a> {
  fn drop(&mut self) { self.0.variable_stacks.pop(); }
}

impl Default for DollarRefsCtx {
  fn default() -> Self { Self { scopes: smallvec![<_>::default()], variable_stacks: vec![vec![]] } }
}
