use proc_macro2::{Ident, Span, TokenStream};
use quote::{quote, quote_spanned, ToTokens};
use smallvec::{smallvec, SmallVec};
use syn::{
  fold::Fold,
  parse::{Parse, ParseStream},
  parse_quote, parse_quote_spanned,
  spanned::Spanned,
  token::Dollar,
  Expr, ExprField, ExprMethodCall, Macro, Member,
};

use crate::{
  fn_widget_macro,
  rdl_macro::RdlMacro,
  variable_names::{ribir_suffix_variable, BuiltinMemberType, BUILTIN_INFOS},
};

pub const KW_DOLLAR_STR: &str = "_dollar_ಠ_ಠ";
pub const KW_CTX: &str = "ctx";
pub const KW_RDL: &str = "rdl";
pub const KW_PIPE: &str = "pipe";
pub const KW_WATCH: &str = "watch";
pub const KW_FN_WIDGET: &str = "fn_widget";

pub use tokens_pre_process::*;

pub mod kw {
  syn::custom_keyword!(_dollar_ಠ_ಠ);
  syn::custom_keyword!(rdl);
}

#[derive(Hash, PartialEq, Eq, Debug, Clone)]
pub struct BuiltinInfo {
  pub(crate) host: Ident,
  pub(crate) member: Ident,
}

#[derive(Hash, PartialEq, Eq, Debug, Clone, PartialOrd, Ord)]
pub enum DollarUsedInfo {
  Reader,
  Watcher,
  Writer,
}

#[derive(Hash, PartialEq, Eq, Debug, Clone)]
pub struct DollarRef {
  pub name: Ident,
  pub builtin: Option<BuiltinInfo>,
  pub used: DollarUsedInfo,
}
#[derive(Debug)]
pub struct DollarRefsCtx {
  scopes: SmallVec<[DollarRefsScope; 1]>,
  // the head stack index of the variable stack for every capture level.
  capture_level_heads: SmallVec<[usize; 1]>,
  variable_stacks: Vec<Vec<Ident>>,
}

#[derive(Debug, Default)]
pub struct DollarRefsScope {
  refs: SmallVec<[DollarRef; 1]>,
  used_ctx: bool,
}

pub struct StackGuard<'a>(&'a mut DollarRefsCtx);

mod tokens_pre_process {

  use proc_macro::*;
  use quote::{quote_spanned, ToTokens};
  use syn::token::Paren;

  use super::KW_DOLLAR_STR;
  use crate::symbol_process::KW_RDL;

  fn rdl_syntax_err<T>(at: Span, follow: Option<Span>) -> Result<T, TokenStream> {
    let err_msg = "Syntax Error: use `@` to declare object, must be: \n 1. `@ XXX { ... }`, \
                   declare a new `XXX` type object;\n 2. `@ $parent { ... }`, declare a variable \
                   as parent;\n 3. `@ { ... } `, declare an object by an expression.";

    let err_tokens = if let Some(follow) = follow {
      let mut err_tokens = quote_spanned! { at.into() => compile_error! };
      Paren::<proc_macro2::Span>(follow.into())
        .surround(&mut err_tokens, |tokens| err_msg.to_tokens(tokens));
      err_tokens
    } else {
      quote_spanned! { at.into() => compile_error!(#err_msg) }
    };
    Err(err_tokens.into())
  }

  fn dollar_err<T>(span: Span) -> Result<T, TokenStream> {
    let err_token = quote_spanned! { span.into() =>
      compile_error!("Syntax error: expected an identifier after `$`")
    };
    Err(err_token.into())
  }

  /// Convert `@` and `$` symbol to a `rdl!` or `_dollar_ಠ_ಠ!` macro, make it
  /// conform to Rust syntax
  pub fn symbol_to_macro(
    input: impl IntoIterator<Item = TokenTree>,
  ) -> Result<TokenStream, TokenStream> {
    let mut iter = input.into_iter();
    let mut tokens = vec![];

    loop {
      match iter.next() {
        Some(TokenTree::Punct(at))
          // maybe rust identify bind syntax, `identify @`
          if at.as_char() == '@' && !matches!(tokens.last(), Some(TokenTree::Ident(_))) =>
        {
          tokens.push(TokenTree::Ident(Ident::new(KW_RDL, at.span())));
          tokens.push(not_token(at.span()));

          let body = match iter.next() {
            // declare a new widget: `@ SizedBox { ... }`
            Some(TokenTree::Ident(name)) => {
              let next = iter.next();
              let Some(TokenTree::Group(body)) = next else {
                return rdl_syntax_err(
                  at.span(),
                  next.map(|n| n.span()).or_else(|| Some(name.span()))
                )
              };
              let tokens = TokenStream::from_iter([TokenTree::Ident(name), TokenTree::Group(body)]);
              Group::new(Delimiter::Brace, tokens)
            }
            // declare a variable widget as parent,  `@ $var { ... }`
            Some(TokenTree::Punct(dollar)) if dollar.as_char() == '$' => {
              if let Some(TokenTree::Ident(var)) = iter.next() {
                let next = iter.next();
                let Some(TokenTree::Group(body))  =  next else {
                  return rdl_syntax_err(
                    at.span(),
                    next.map(|n| n.span()).or_else(|| Some(var.span()))
                  )
                };
                let tokens = TokenStream::from_iter([
                  TokenTree::Punct(dollar),
                  TokenTree::Ident(var),
                  TokenTree::Group(body),
                ]);
                Group::new(Delimiter::Brace, tokens)
              } else {
                return dollar_err(dollar.span());
              }
            }
            // declare a expression widget  `@ { ... }`
            Some(TokenTree::Group(g)) => g,
            n => {
              return rdl_syntax_err(at.span(), n.map(|n| n.span()));
            }
          };
          tokens.push(TokenTree::Group(body));
        }
        Some(TokenTree::Punct(p)) if p.as_char() == '$' => {
          match iter.next() {
            Some(TokenTree::Ident(name)) => {
              tokens.push(TokenTree::Ident(Ident::new(KW_DOLLAR_STR, p.span())));
              tokens.push(not_token(p.span()));
              let span = name.span();
              let mut g = Group::new(
                Delimiter::Parenthesis,
                [TokenTree::Punct(p), TokenTree::Ident(name)].into_iter().collect()
              );
              g.set_span(span);
              tokens.push(TokenTree::Group(g));
            }
            Some(t) => return dollar_err(t.span()),
            None => return dollar_err(p.span()),
          };
        }
        Some(TokenTree::Group(mut g)) => {
          // not process symbol in macro, because it's maybe as part of the macro syntax.
          if !in_macro(&tokens) {
            let mut n = Group::new(g.delimiter(), symbol_to_macro(g.stream())?);
            n.set_span(g.span());
            g = n;
          }

          tokens.push(TokenTree::Group(g));
        }
        Some(t) => tokens.push(t),
        None => break,
      };
    }
    Ok(tokens.into_iter().collect())
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

  fn fold_expr_field(&mut self, mut i: ExprField) -> ExprField {
    let ExprField { base, member, .. } = &mut i;

    if let Member::Named(member) = member {
      let dollar = BUILTIN_INFOS
        .get(member.to_string().as_str())
        .filter(|info| info.mem_ty == BuiltinMemberType::Field)
        .and_then(|info| self.replace_builtin_ident(&mut *base, info.var_name));
      if dollar.is_some() {
        return i;
      }
    }

    syn::fold::fold_expr_field(self, i)
  }

  fn fold_expr_method_call(&mut self, mut i: ExprMethodCall) -> ExprMethodCall {
    // fold builtin method on state
    let dollar = BUILTIN_INFOS
      .get(i.method.to_string().as_str())
      .filter(|info| info.mem_ty == BuiltinMemberType::Method)
      .and_then(|info| self.replace_builtin_ident(&mut i.receiver, info.var_name));
    if dollar.is_some() {
      return i;
    }

    // fold if write on state.
    let write_mac = is_state_write_method(&i).then(|| {
      let Expr::Macro(m) = &mut *i.receiver else {
        return None;
      };
      parse_dollar_macro(&m.mac).map(|d| (d.name, &mut m.mac))
    });
    if let Some(Some((name, mac))) = write_mac {
      mac.tokens = expand_write_method(name.to_token_stream());
      mark_macro_expanded(mac);
      let dollar_ref = DollarRef { name, builtin: None, used: DollarUsedInfo::Writer };
      self.add_dollar_ref(dollar_ref);
      return i;
    }

    syn::fold::fold_expr_method_call(self, i)
  }

  fn fold_macro(&mut self, mut mac: Macro) -> Macro {
    if let Some(DollarMacro { name, .. }) = parse_dollar_macro(&mac) {
      mac.tokens = expand_read(name.to_token_stream());
      mark_macro_expanded(&mut mac);
      let dollar_ref = DollarRef { name, builtin: None, used: DollarUsedInfo::Reader };
      self.add_dollar_ref(dollar_ref)
    } else if mac.path.is_ident(KW_WATCH) {
      mac.tokens = crate::watch_macro::gen_code(mac.tokens, self).into();
      mark_macro_expanded(&mut mac);
    } else if mac.path.is_ident(KW_PIPE) {
      self.mark_used_ctx();
      mac.tokens = crate::pipe_macro::gen_code(mac.tokens, self).into();
      mark_macro_expanded(&mut mac);
    } else if mac.path.is_ident(KW_RDL) {
      self.mark_used_ctx();
      mac.tokens = RdlMacro::gen_code(mac.tokens, self).into();
      mark_macro_expanded(&mut mac);
    } else if mac.path.is_ident(KW_FN_WIDGET) {
      mac.tokens = fn_widget_macro::gen_code(mac.tokens, self).into();
      mark_macro_expanded(&mut mac);
    } else if mac.path.is_ident(KW_CTX) {
      self.mark_used_ctx();
    } else {
      mac = syn::fold::fold_macro(self, mac);
    }
    mac
  }

  fn fold_expr(&mut self, i: Expr) -> Expr {
    match i {
      Expr::Closure(c) if c.capture.is_some() => {
        self.new_dollar_scope(true);
        let mut c = self.fold_expr_closure(c);
        let dollar_scope = self.pop_dollar_scope(true, false);

        if dollar_scope.used_ctx() || !dollar_scope.is_empty() {
          if dollar_scope.used_ctx() {
            let body = &mut *c.body;
            let body_with_ctx = quote_spanned! { body.span() =>
              _ctx_handle.with_ctx(|ctx!()| #body).expect("ctx is not available")
            };

            if matches!(c.output, syn::ReturnType::Default) {
              *body = parse_quote! { #body_with_ctx };
            } else {
              *body = parse_quote_spanned! { body.span() => { #body_with_ctx }};
            }
          }

          let handle = dollar_scope
            .used_ctx()
            .then(|| quote_spanned! { c.span() => let _ctx_handle = ctx!().handle(); });

          Expr::Verbatim(quote_spanned!(c.span() => {
            #dollar_scope
            #handle
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
    for DollarRef { name, builtin, used } in &self.refs {
      if let Some(BuiltinInfo { host, member }) = builtin {
        quote_spanned! { name.span() => let #name = #host.#member() }
      } else {
        quote_spanned! { name.span() => let #name = #name }
      }
      .to_tokens(tokens);
      match used {
        DollarUsedInfo::Reader => quote_spanned! { name.span() => .clone_reader() },
        DollarUsedInfo::Watcher => {
          quote_spanned! { name.span() => .clone_watcher() }
        }
        DollarUsedInfo::Writer => quote_spanned! { name.span() => .clone_writer() },
      }
      .to_tokens(tokens);
      syn::token::Semi(name.span()).to_tokens(tokens);
    }
  }
}

impl DollarRefsCtx {
  #[inline]
  pub fn top_level() -> Self { Self::default() }

  #[inline]
  pub fn new_dollar_scope(&mut self, capture_scope: bool) {
    if capture_scope {
      self
        .capture_level_heads
        .push(self.variable_stacks.len());
      // new scope level, should start a new variables scope, otherwise the local
      // variables will record in its parent level.
      self.variable_stacks.push(vec![]);
    }
    self.scopes.push(<_>::default());
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
  /// - **capture_scope**: if the scope is a capture scope, should pop the
  ///   variable stack.
  /// - **watch_scope**: if the scope is a watch scope, should mark all reader
  ///   as watcher.
  #[inline]
  pub fn pop_dollar_scope(&mut self, capture_scope: bool, watch_scope: bool) -> DollarRefsScope {
    if capture_scope {
      self.variable_stacks.pop();
      self.capture_level_heads.pop();
    }
    let mut scope = self.scopes.pop().unwrap();

    // sort and remove duplicate
    scope.refs.sort_by(|a, b| {
      a.builtin
        .is_none()
        .cmp(&b.builtin.is_none())
        .then_with(|| a.name.cmp(&b.name))
        .then_with(|| b.used.cmp(&a.used))
    });
    scope.refs.dedup_by(|a, b| a.name == b.name);

    if !self.scopes.is_empty() {
      self.current_dollar_scope_mut().used_ctx |= scope.used_ctx();

      for r in scope.refs.iter_mut() {
        if !self.is_local_var(r.host()) && self.scopes.len() > 1 {
          let mut c_r = r.clone();
          if watch_scope && c_r.used == DollarUsedInfo::Reader {
            c_r.used = DollarUsedInfo::Watcher;
          }
          self.current_dollar_scope_mut().refs.push(c_r);
          // if ref variable is not a local variable of parent capture level, should
          // remove its builtin info as a normal variable, because parent will capture the
          // builtin object individually.
          if capture_scope {
            r.builtin.take();
          }
        }
      }
    }
    scope
  }

  pub fn push_code_stack(&mut self) -> StackGuard<'_> {
    self.variable_stacks.push(vec![]);
    StackGuard(self)
  }

  pub fn builtin_host_tokens(&self, dollar_ref: &DollarRef) -> TokenStream {
    let DollarRef { name, builtin, .. } = dollar_ref;
    let BuiltinInfo { host, member } = builtin.as_ref().unwrap();

    // if used in embedded closure, we directly use the builtin variable, the
    // variable that capture by the closure is already a separate builtin variable.

    if !self.is_local_var(host) && self.capture_level_heads.len() > 1 {
      name.to_token_stream()
    } else {
      quote_spanned! { host.span() => #host.#member() }
    }
  }

  fn mark_used_ctx(&mut self) { self.current_dollar_scope_mut().used_ctx = true; }

  fn replace_builtin_ident(
    &mut self, caller: &mut Expr, builtin_member: &str,
  ) -> Option<&DollarRef> {
    let mut used = DollarUsedInfo::Reader;
    let e = if let Expr::MethodCall(m) = caller {
      if is_state_write_method(m) {
        used = DollarUsedInfo::Writer;
        &mut *m.receiver
      } else {
        caller
      }
    } else {
      caller
    };

    let Expr::Macro(m) = e else { return None };
    let DollarMacro { name: host, .. } = parse_dollar_macro(&m.mac)?;

    // When a builtin widget captured by a `move |_| {...}` closure, we need split
    // the builtin widget from the `FatObj` so we only capture the builtin part that
    // we used.
    let name = ribir_suffix_variable(&host, builtin_member);
    let get_builtin_method = Ident::new(&format!("get_{builtin_member}_widget"), host.span());
    let builtin = Some(BuiltinInfo { host, member: get_builtin_method });
    let dollar_ref = DollarRef { name, builtin, used };

    let state = self.builtin_host_tokens(&dollar_ref);
    m.mac.tokens = if dollar_ref.used == DollarUsedInfo::Writer {
      expand_write_method(state)
    } else {
      expand_read(state)
    };
    mark_macro_expanded(&mut m.mac);

    self.add_dollar_ref(dollar_ref);
    self.current_dollar_scope().last()
  }

  fn new_local_var(&mut self, name: &Ident) {
    self
      .variable_stacks
      .last_mut()
      .unwrap()
      .push(name.clone())
  }

  fn add_dollar_ref(&mut self, dollar_ref: DollarRef) {
    // local variable is not a outside reference.
    if !self.is_local_var(dollar_ref.host()) {
      let scope = self
        .scopes
        .last_mut()
        .expect("no dollar refs scope");
      scope.refs.push(dollar_ref);
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

  fn is_local_var(&self, name: &Ident) -> bool {
    let stack_idx = self
      .capture_level_heads
      .last()
      .copied()
      .unwrap_or(0);
    self.variable_stacks[stack_idx..]
      .iter()
      .any(|stack| stack.contains(name))
  }
}

impl DollarRefsScope {
  pub fn used_ctx(&self) -> bool { self.used_ctx }

  pub fn upstream_tokens(&self) -> TokenStream {
    match self.len() {
      0 => quote! {},
      1 => self.refs[0].upstream_tokens(),
      _ => {
        let upstream = self.iter().map(DollarRef::upstream_tokens);
        quote_spanned! { self.refs[0].name.span() =>
          observable::from_iter([#(#upstream),*]).merge_all(usize::MAX)
        }
      }
    }
  }
}

impl DollarRef {
  pub fn host(&self) -> &Ident {
    self
      .builtin
      .as_ref()
      .map_or_else(|| &self.name, |b| &b.host)
  }

  pub fn upstream_tokens(&self) -> TokenStream {
    let DollarRef { name, builtin, .. } = self;
    if let Some(BuiltinInfo { host, member }) = builtin {
      quote_spanned! { name.span() => #host.#member().modifies() }
    } else {
      quote_spanned! { name.span() => #name.modifies() }
    }
  }
}

fn parse_dollar_macro(mac: &Macro) -> Option<DollarMacro> {
  if mac.path.is_ident(KW_DOLLAR_STR) {
    Some(mac.parse_body::<DollarMacro>().unwrap())
  } else {
    None
  }
}

impl std::ops::Deref for DollarRefsScope {
  type Target = [DollarRef];
  fn deref(&self) -> &Self::Target { &self.refs }
}

struct DollarMacro {
  _dollar: Dollar,
  name: Ident,
}

impl Parse for DollarMacro {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    let _dollar = input.parse()?;
    let name = if input.peek(syn::token::SelfValue) {
      let name = input.parse::<syn::token::SelfValue>()?;
      Ident::new("self", name.span())
    } else {
      input.parse::<Ident>()?
    };

    Ok(Self { _dollar, name })
  }
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
  fn default() -> Self {
    Self { scopes: smallvec![], capture_level_heads: smallvec![], variable_stacks: vec![vec![]] }
  }
}

pub fn not_subscribe_anything(span: Span) -> TokenStream {
  quote_spanned!(span =>
    compile_error!("expression not subscribe anything, it must contain at least one $")
  )
}

fn is_state_write_method(m: &ExprMethodCall) -> bool {
  m.method == "write" || m.method == "silent" || m.method == "shallow"
}

fn expand_write_method(host: TokenStream) -> TokenStream { host }

fn expand_read(name: TokenStream) -> TokenStream { quote_spanned!(name.span() => #name.read()) }
