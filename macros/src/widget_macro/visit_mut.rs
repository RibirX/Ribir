use crate::{
  declare_derive::declare_field_name,
  error::DeclareError,
  widget_macro::{desugar::WatchField, guard_ident, guard_vec_ident},
  LET_WATCH_MACRO_NAME, MOVE_TO_WIDGET_MACRO_NAME, PROP_MACRO_NAME, WATCH_MACRO_NAME,
  WIDGET_MACRO_NAME,
};

use super::{
  builtin_var_name, capture_widget,
  code_gen::{gen_move_to_widget_macro, gen_prop_macro},
  desugar::{
    ComposeItem, DeclareObj, FieldValue, FinallyBlock, FinallyStmt, InitStmts, NamedObj, WidgetNode,
  },
  gen_widget_macro, ribir_suffix_variable, Desugared, ScopeUsedInfo, TrackExpr, UsedType,
  WIDGET_OF_BUILTIN_FIELD, WIDGET_OF_BUILTIN_METHOD,
};

use proc_macro::Span;
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned, ToTokens};
use std::{
  collections::{HashMap, HashSet},
  hash::Hash,
};
use syn::{
  parse_macro_input, parse_quote, parse_quote_spanned,
  spanned::Spanned,
  token::{Brace, Semi},
  visit_mut,
  visit_mut::VisitMut,
  Expr, ExprClosure, ExprMethodCall, ExprPath, Ident, ItemMacro, Member, Path, Stmt,
};

bitflags::bitflags! {
  pub struct IdType: u16 {
    /// Declared by `id: name`,
    const DECLARE = 0x001;
    /// name provide in `states { ... }`
    const USER_SPECIFY = 0x010;
      /// name pass by outside `widget!` macro.
    const FROM_ANCESTOR = 0x100;
  }
}

pub struct VisitCtx {
  /// All declared object.
  pub declare_objs: HashMap<Ident, Path, ahash::RandomState>,
  pub states: HashSet<Ident, ahash::RandomState>,
  pub current_used_info: ScopeUsedInfo,
  /// name object has be used and its source name.
  pub used_objs: HashMap<Ident, UsedInfo, ahash::RandomState>,
  pub analyze_stack: Vec<Vec<LocalVariable>>,
  pub has_guards_data: bool,
  pub visit_error_occur: bool,
}

#[derive(Debug, Clone)]
pub struct LocalVariable {
  name: Ident,
  alias_of_name: Option<Ident>,
}

impl LocalVariable {
  pub fn local(name: Ident) -> Self { Self { name, alias_of_name: None } }
}

#[derive(Debug, Clone)]
pub struct UsedInfo {
  pub builtin: Option<BuiltinUsed>,
  pub spans: Vec<Span>,
}

#[derive(Debug, Clone)]
pub struct BuiltinUsed {
  pub src_name: Ident,
  pub builtin_ty: &'static str,
}

impl Default for VisitCtx {
  fn default() -> Self {
    Self {
      declare_objs: <_>::default(),
      states: <_>::default(),
      current_used_info: Default::default(),
      used_objs: Default::default(),
      analyze_stack: vec![vec![]],
      has_guards_data: false,
      visit_error_occur: false,
    }
  }
}

impl VisitMut for VisitCtx {
  fn visit_expr_mut(&mut self, expr: &mut Expr) {
    match expr {
      Expr::Macro(m) => {
        let mac = &m.mac;
        if mac.path.is_ident(WIDGET_MACRO_NAME) {
          *expr = Expr::Verbatim(gen_widget_macro(mac.tokens.clone().into(), Some(self)).into());
        } else if mac.path.is_ident(WATCH_MACRO_NAME) {
          *expr = Expr::Verbatim(gen_watch_macro(mac.tokens.clone().into(), self).into());
        } else if mac.path.is_ident(PROP_MACRO_NAME) {
          *expr = Expr::Verbatim(gen_prop_macro(mac.tokens.clone().into(), self).into());
        } else if mac.path.is_ident(MOVE_TO_WIDGET_MACRO_NAME) {
          *expr = Expr::Verbatim(gen_move_to_widget_macro(&mac.tokens, self));
        } else if mac.path.is_ident(LET_WATCH_MACRO_NAME) {
          let mut tokens = quote! {};
          DeclareError::LetWatchWrongPlace(mac.span().unwrap()).into_compile_error(&mut tokens);
          *expr = Expr::Verbatim(tokens);
          self.visit_error_occur = true;
        } else {
          visit_mut::visit_expr_macro_mut(self, m);
        }
      }
      Expr::Path(p) => {
        visit_mut::visit_expr_path_mut(self, p);
        if let Some(name) = p.path.get_ident() {
          if let Some(name) = self.find_named_obj(name).cloned() {
            self.add_used_widget(name, None, UsedType::USED)
          }
        }
      }
      Expr::Closure(c) => {
        let mut new_closure = None;
        let is_capture = c.capture.is_some();
        let mut ctx = self.stack_push();
        ctx.new_scope_visit(
          |ctx| {
            visit_mut::visit_expr_closure_mut(ctx, c);
            new_closure = closure_surround_refs(&ctx.current_used_info, c);
          },
          |scope| {
            scope.iter_mut().for_each(|(_, info)| {
              if is_capture {
                info.used_type = UsedType::SCOPE_CAPTURE
              } else {
                info.used_type |= UsedType::SCOPE_CAPTURE;
                info.used_type.remove(UsedType::SUBSCRIBE);
              }
            });
          },
        );
        if let Some(new) = new_closure {
          *expr = parse_quote!(#new);
        }
      }
      _ => {
        visit_mut::visit_expr_mut(self, expr);
      }
    }
  }

  fn visit_stmt_mut(&mut self, stmt: &mut Stmt) {
    match stmt {
      Stmt::Item(syn::Item::Macro(ItemMacro { ident: None, mac, .. })) => {
        if mac.path.is_ident(WIDGET_MACRO_NAME) {
          let expr: TokenStream = gen_widget_macro(mac.tokens.clone().into(), Some(self)).into();
          *stmt = Stmt::Expr(Expr::Verbatim(expr));
        } else if mac.path.is_ident(WATCH_MACRO_NAME) {
          let t = gen_watch_macro(mac.tokens.clone().into(), self);
          *stmt = Stmt::Expr(Expr::Verbatim(t.into()).into());
        } else if mac.path.is_ident(PROP_MACRO_NAME) {
          *stmt = Stmt::Expr(Expr::Verbatim(
            gen_prop_macro(mac.tokens.clone().into(), self).into(),
          ));
        } else if mac.path.is_ident(MOVE_TO_WIDGET_MACRO_NAME) {
          *stmt = Stmt::Expr(Expr::Verbatim(gen_move_to_widget_macro(&mac.tokens, self)));
        } else if mac.path.is_ident(LET_WATCH_MACRO_NAME) {
          let mut tokens = quote! {};
          DeclareError::LetWatchWrongPlace(mac.span().unwrap()).into_compile_error(&mut tokens);
          self.visit_error_occur = true;
          *stmt = Stmt::Expr(Expr::Verbatim(tokens));
        }
      }
      Stmt::Expr(expr) => {
        if let Some(new_stmt) = self.let_watch_desugar(expr, None) {
          *stmt = new_stmt;
        }
      }
      Stmt::Semi(expr, semi) => {
        if let Some(new_stmt) = self.let_watch_desugar(expr, Some(*semi)) {
          *stmt = new_stmt;
        }
      }
      _ => {}
    }
    visit_mut::visit_stmt_mut(self, stmt);
  }

  fn visit_expr_field_mut(&mut self, f_expr: &mut syn::ExprField) {
    if let Member::Named(member) = &f_expr.member {
      if let Some(builtin_ty) = WIDGET_OF_BUILTIN_FIELD.get(member.to_string().as_str()) {
        let span = f_expr.span();
        if self.visit_builtin_in_expr(&mut f_expr.base, span, builtin_ty) {
          return;
        }
      }
    }

    visit_mut::visit_expr_field_mut(self, f_expr);
  }

  fn visit_expr_method_call_mut(&mut self, i: &mut ExprMethodCall) {
    if let Some(builtin_ty) = WIDGET_OF_BUILTIN_METHOD.get(i.method.to_string().as_str()) {
      let span = i.span();
      if self.visit_builtin_in_expr(&mut i.receiver, span, builtin_ty) {
        return;
      }
    }

    visit_mut::visit_expr_method_call_mut(self, i);
  }

  fn visit_expr_assign_mut(&mut self, assign: &mut syn::ExprAssign) {
    self.recursive_visit_assign_mut(&mut *assign.left, &mut assign.right);
  }

  fn visit_block_mut(&mut self, i: &mut syn::Block) {
    let mut ctx = self.stack_push();
    visit_mut::visit_block_mut(&mut *ctx, i);
  }

  fn visit_item_const_mut(&mut self, i: &mut syn::ItemConst) {
    visit_mut::visit_item_const_mut(self, i);

    self.analyze_stack.last_mut().unwrap().push(LocalVariable {
      name: i.ident.clone(),
      alias_of_name: None,
    });
  }

  fn visit_local_mut(&mut self, local: &mut syn::Local) {
    self.recursive_visit_local_mut(
      &mut local.pat,
      local.init.as_mut().map(|(_, init)| &mut **init),
    );
  }

  fn visit_expr_block_mut(&mut self, i: &mut syn::ExprBlock) {
    let mut ctx = self.stack_push();
    visit_mut::visit_expr_block_mut(&mut *ctx, i);
  }

  fn visit_expr_for_loop_mut(&mut self, i: &mut syn::ExprForLoop) {
    let mut ctx = self.stack_push();
    visit_mut::visit_expr_for_loop_mut(&mut *ctx, i);
  }

  fn visit_expr_loop_mut(&mut self, i: &mut syn::ExprLoop) {
    let mut ctx = self.stack_push();
    visit_mut::visit_expr_loop_mut(&mut *ctx, i);
  }

  fn visit_expr_if_mut(&mut self, i: &mut syn::ExprIf) {
    let mut ctx = self.stack_push();
    visit_mut::visit_expr_if_mut(&mut *ctx, i);
  }

  fn visit_arm_mut(&mut self, i: &mut syn::Arm) {
    let mut ctx = self.stack_push();
    visit_mut::visit_arm_mut(&mut *ctx, i);
  }

  fn visit_expr_unsafe_mut(&mut self, i: &mut syn::ExprUnsafe) {
    let mut ctx = self.stack_push();
    visit_mut::visit_expr_unsafe_mut(&mut *ctx, i);
  }

  fn visit_expr_while_mut(&mut self, i: &mut syn::ExprWhile) {
    let mut ctx = self.stack_push();
    visit_mut::visit_expr_while_mut(&mut *ctx, i);
  }

  fn visit_pat_ident_mut(&mut self, i: &mut syn::PatIdent) {
    visit_mut::visit_pat_ident_mut(self, i);

    self
      .analyze_stack
      .last_mut()
      .unwrap_or_else(|| {
        panic!(
          "Crash when visit `{}`, stack should not be empty, at {}:{}:{}",
          quote! { #i },
          file!(),
          line!(),
          column!()
        )
      })
      .push(LocalVariable::local(i.ident.clone()));
  }
}

impl VisitCtx {
  fn let_watch_desugar(&mut self, expr: &mut Expr, semi: Option<Semi>) -> Option<Stmt> {
    fn let_watch_as_watch(expr: &mut Expr) -> bool {
      let Expr::MethodCall(method_call) = expr else {return false;};
      if let Expr::Macro(mac) = &mut *method_call.receiver {
        let path = &mut mac.mac.path;
        if path.is_ident(LET_WATCH_MACRO_NAME) {
          let watch = Ident::new(WATCH_MACRO_NAME, path.span());
          *path = watch.into();
          return true;
        }
      }
      return let_watch_as_watch(&mut method_call.receiver);
    }

    let_watch_as_watch(expr).then(|| {
      let guard = guard_ident(expr.span());

      let move_to_widget = Ident::new(MOVE_TO_WIDGET_MACRO_NAME, expr.span());
      let res = syn::parse2::<Stmt>(quote_spanned! { expr.span() =>{
        let #guard = #expr #semi
        #move_to_widget!(#guard.unsubscribe_when_dropped());
      }});

      res.unwrap_or_else(|err| {
        self.visit_error_occur = true;
        let tokens = err.into_compile_error();
        Stmt::Expr(Expr::Verbatim(tokens))
      })
    })
  }

  pub fn visit_desugared_syntax_mut(&mut self, desugar: &mut Desugared) {
    desugar.named_objs.objs_mut().for_each(|obj| match obj {
      NamedObj::Host(obj) | NamedObj::Builtin { obj, .. } => self.visit_declare_obj_mut(obj, false),
    });

    self.take_current_used_info();

    self.visit_widget_node_mut(&mut desugar.widget.as_mut().unwrap());
    if let Some(finally) = desugar.finally.as_mut() {
      self.visit_finally_mut(finally);
    }
  }

  pub fn visit_init_stmts_mut(&mut self, init: &mut InitStmts) {
    init
      .stmts
      .iter_mut()
      .for_each(|stmt| self.visit_stmt_mut(stmt));
    init.used_name_info = self.take_current_used_info();
  }

  pub fn visit_finally_mut(&mut self, finally: &mut FinallyBlock) {
    finally.stmts.iter_mut().for_each(|stmt| match stmt {
      FinallyStmt::Stmt(s) => self.visit_stmt_mut(s),
      FinallyStmt::Obj(o) => self.visit_declare_obj_mut(o, false),
    });
    finally.used_name_info = self.take_current_used_info();
  }

  pub fn visit_declare_obj_mut(&mut self, obj: &mut DeclareObj, value_obj: bool) {
    let DeclareObj { ty, name, fields, watch_stmts, .. } = obj;
    self.new_scope_visit(
      |ctx| {
        ctx.visit_path_mut(ty);
        fields.iter_mut().for_each(|f| {
          ctx.new_scope_visit(
            |ctx| match &mut f.value {
              FieldValue::Expr(expr) => {
                let origin = expr.expr.clone();
                ctx.visit_track_expr_mut(expr);
                if expr.used_name_info.subscribe_widget().is_some() {
                  let field_fn_name = ribir_suffix_variable(&f.member, "fn");
                  let mut field_fn = parse_quote_spanned! { expr.span() =>
                    let #field_fn_name = move ||  #origin ;
                  };
                  ctx.visit_stmt_mut(&mut field_fn);
                  expr.expr = parse_quote_spanned! {expr.span() => #field_fn_name()};

                  let declare_set = declare_field_name(&f.member);
                  let subscribe_do: Expr = parse_quote_spanned! { expr.span() => {
                    let #name = #name.clone_stateful();
                    move |_| #name.state_ref().#declare_set(#field_fn_name())
                  }};

                  ctx.has_guards_data = true;
                  let guards = guard_vec_ident();

                  // DynWidget is a special object, it's both require data and framework change to
                  // update its children. When user call `.silent()` means no
                  // need relayout and redraw the widget. `DynWidget` as the directly subscriber
                  // also needn't to change.
                  let upstream = if ty.is_ident("DynWidget") && f.member == "dyns" {
                    let mut upstream = expr.used_name_info.upstream_modifies_tokens(true).unwrap();
                    upstream.extend(quote_spanned! {
                      f.member.span() => .filter(|s| s.contains(ModifyScope::FRAMEWORK))
                    });
                    upstream
                  } else {
                    expr.used_name_info.upstream_modifies_tokens(false).unwrap()
                  };
                  let watch_update = parse_quote_spanned! { expr.span() =>
                    #guards.push(AnonymousData::new(Box::new(
                      #upstream
                      .subscribe( #subscribe_do )
                      .unsubscribe_when_dropped()
                    )));
                  };
                  watch_stmts.push(WatchField { field_fn, watch_update });
                }
              }
              FieldValue::Obj(obj) => {
                ctx.visit_declare_obj_mut(obj, true);
              }
            },
            |scope| {
              if scope.subscribe_widget().is_some() {
                scope
                  .iter_mut()
                  .for_each(|(_, info)| info.used_type = UsedType::SCOPE_CAPTURE)
              }
            },
          )
        });

        if !value_obj {
          obj.used_name_info = ctx.take_current_used_info();
        }
      },
      |_| {},
    );
  }

  pub fn visit_track_expr_mut(&mut self, expr: &mut TrackExpr) {
    self.new_scope_visit(
      |ctx| {
        ctx.visit_expr_mut(&mut expr.expr);
        expr.used_name_info = ctx.current_used_info.clone();
      },
      |_| {},
    );
  }

  pub fn visit_widget_node_mut(&mut self, widget: &mut WidgetNode) {
    let WidgetNode { node: parent, children } = widget;
    self.visit_compose_item_mut(parent);
    children
      .iter_mut()
      .for_each(|node| self.visit_widget_node_mut(node));
  }

  pub fn visit_compose_item_mut(&mut self, widget: &mut ComposeItem) {
    match widget {
      ComposeItem::ChainObjs(objs) => objs
        .iter_mut()
        .for_each(|obj| self.visit_declare_obj_mut(obj, false)),
      ComposeItem::Id(_) => {}
    }
  }

  pub fn take_current_used_info(&mut self) -> ScopeUsedInfo { self.current_used_info.take() }

  pub fn stack_push(&mut self) -> StackGuard<'_> { StackGuard::new(self) }

  // return the name of widget that `ident` point to if it's have.
  pub fn find_named_obj<'a>(&'a self, ident: &'a Ident) -> Option<&'a Ident> {
    self
      .analyze_stack
      .iter()
      .rev()
      .flat_map(|local| local.iter().rev())
      .find(|v| &v.name == ident)
      .map(|v| v.alias_of_name.as_ref())
      .unwrap_or_else(|| {
        (self.declare_objs.contains_key(ident) || self.states.contains(ident)).then(|| ident)
      })
  }

  fn path_as_named_obj(&self, expr: &ExprPath) -> Option<Ident> {
    expr
      .path
      .get_ident()
      .and_then(|name| self.find_named_obj(name))
      .cloned()
  }

  fn recursive_visit_assign_mut(&mut self, left: &mut Expr, right: &mut Expr) {
    match (left, right) {
      (Expr::Path(l), Expr::Path(r)) => {
        if let (Some(l), Some(r)) = (self.path_as_named_obj(l), r.path.get_ident()) {
          let local_var = self
            .analyze_stack
            .iter_mut()
            .rev()
            .flat_map(|locals| locals.iter_mut().rev())
            .find(|v| v.name == l);
          if let Some(local_var) = local_var {
            local_var.alias_of_name = Some(r.clone());
          }
        }
      }
      (Expr::Tuple(l), Expr::Tuple(r)) => {
        l.elems
          .iter_mut()
          .zip(r.elems.iter_mut())
          .for_each(|(l, r)| self.recursive_visit_assign_mut(l, r));
      }
      (left, right) => {
        self.visit_expr_mut(left);
        self.visit_expr_mut(right);
      }
    }
  }

  fn recursive_visit_local_mut(&mut self, left: &mut syn::Pat, right: Option<&mut Expr>) {
    match (left, right) {
      (syn::Pat::Ident(i), Some(Expr::Path(path))) => {
        let name = i.ident.clone();
        let var = if let Some(right) = self.path_as_named_obj(path) {
          LocalVariable { name, alias_of_name: Some(right) }
        } else {
          LocalVariable::local(name)
        };
        self.analyze_stack.last_mut().unwrap().push(var);
      }
      (syn::Pat::Tuple(left), Some(Expr::Tuple(right))) => {
        left
          .elems
          .iter_mut()
          .zip(right.elems.iter_mut())
          .for_each(|(l, r)| self.recursive_visit_local_mut(l, Some(r)));
      }
      (left, right) => {
        self.visit_pat_mut(left);
        if let Some(right) = right {
          self.visit_expr_mut(right);
        }
      }
    }
  }

  pub fn add_used_widget(
    &mut self,
    name: Ident,
    builtin: Option<BuiltinUsed>,
    used_type: UsedType,
  ) {
    self.inner_add_used_obj(name.clone(), builtin);
    self.current_used_info.add_used(name, used_type);
  }

  fn inner_add_used_obj(&mut self, name: Ident, builtin: Option<BuiltinUsed>) {
    let span = name.span().unwrap();
    self
      .used_objs
      .entry(name)
      .and_modify(|info| {
        info.spans.push(span);
      })
      .or_insert_with(|| UsedInfo { builtin, spans: vec![span] });
  }

  pub fn visit_builtin_in_expr(
    &mut self,
    expr: &mut syn::Expr,
    span: proc_macro2::Span,
    builtin_ty: &'static str,
  ) -> bool {
    let path = match expr {
      Expr::Path(syn::ExprPath { path, .. }) => path,
      Expr::MethodCall(ExprMethodCall { receiver, method, args, .. })
        if args.is_empty() && (method == "shallow" || method == "silent") =>
      {
        if let Expr::Path(syn::ExprPath { path, .. }) = &mut **receiver {
          path
        } else {
          return false;
        }
      }
      _ => return true,
    };
    let Some(name) = path.get_ident() else { return false };

    if let Some(builtin_name) = self.visit_builtin_name_mut(name, span, builtin_ty) {
      *path = parse_quote! { #builtin_name };
      true
    } else {
      false
    }
  }

  pub fn visit_builtin_name_mut(
    &mut self,
    host: &Ident,
    span: proc_macro2::Span,
    builtin_ty: &'static str,
  ) -> Option<Ident> {
    let name = self.find_named_obj(host)?;

    let ty = self.declare_objs.get(&name)?;

    if !ty.is_ident(builtin_ty) {
      let builtin_name = builtin_var_name(&name, span, builtin_ty);
      let src_name = name.clone();
      self.add_used_widget(
        builtin_name.clone(),
        Some(BuiltinUsed { src_name, builtin_ty }),
        UsedType::USED,
      );
      Some(builtin_name)
    } else {
      None
    }
  }

  pub(crate) fn new_scope_visit(
    &mut self,
    visiter: impl FnOnce(&mut Self),
    update_used_type: impl Fn(&mut ScopeUsedInfo),
  ) {
    let mut outside_used = self.current_used_info.take();
    visiter(self);
    update_used_type(&mut self.current_used_info);
    outside_used.merge(&self.current_used_info);
    self.current_used_info = outside_used;
  }
}

#[must_use]
pub(crate) fn closure_surround_refs(
  body_used: &ScopeUsedInfo,
  c: &mut ExprClosure,
) -> Option<TokenStream> {
  c.capture?;
  let all_capture = body_used.all_used()?;

  let mut tokens = quote!();
  Brace(c.span()).surround(&mut tokens, |tokens| {
    all_capture.for_each(|obj| capture_widget(obj).to_tokens(tokens));
    if body_used.ref_widgets().is_some() {
      let mut refs = quote! {};
      body_used.prepend_bundle_refs(&mut refs);
      let body = &mut *c.body;
      if let Expr::Block(block) = body {
        block
          .block
          .stmts
          .insert(0, Stmt::Expr(Expr::Verbatim(refs)));
      } else {
        *body = parse_quote_spanned!(body.span() => { #refs #body });
      }
    }
    c.to_tokens(tokens);
  });
  Some(tokens)
}
pub struct StackGuard<'a> {
  ctx: &'a mut VisitCtx,
}

pub struct CaptureScopeGuard<'a> {
  ctx: &'a mut VisitCtx,
}

impl<'a> StackGuard<'a> {
  pub fn new(ctx: &'a mut VisitCtx) -> Self {
    ctx.analyze_stack.push(vec![]);
    StackGuard { ctx }
  }
}

impl<'a> Drop for StackGuard<'a> {
  fn drop(&mut self) { self.ctx.analyze_stack.pop(); }
}

impl<'a> std::ops::Deref for StackGuard<'a> {
  type Target = VisitCtx;

  fn deref(&self) -> &Self::Target { self.ctx }
}

impl<'a> std::ops::DerefMut for StackGuard<'a> {
  fn deref_mut(&mut self) -> &mut Self::Target { self.ctx }
}

impl<'a> std::ops::Deref for CaptureScopeGuard<'a> {
  type Target = VisitCtx;

  fn deref(&self) -> &Self::Target { self.ctx }
}

impl<'a> std::ops::DerefMut for CaptureScopeGuard<'a> {
  fn deref_mut(&mut self) -> &mut Self::Target { self.ctx }
}

pub(crate) fn gen_watch_macro(input: TokenStream, ctx: &mut VisitCtx) -> proc_macro::TokenStream {
  let input = input.into();
  let mut watch_expr = TrackExpr::new(parse_macro_input! { input as Expr });

  ctx.new_scope_visit(
    |ctx| ctx.visit_track_expr_mut(&mut watch_expr),
    |scope| {
      scope.iter_mut().for_each(|(_, info)| {
        info.used_type.remove(UsedType::SUBSCRIBE);
        info.used_type |= UsedType::SCOPE_CAPTURE
      });
    },
  );
  if let Some(upstream) = watch_expr.used_name_info.upstream_modifies_tokens(false) {
    let map_closure = closure_surround_refs(
      &watch_expr.used_name_info,
      &mut parse_quote!( move |_| #watch_expr),
    )
    .unwrap();

    quote_spanned! { watch_expr.span() => #upstream.map(#map_closure) }.into()
  } else {
    let mut tokens = quote! {};
    DeclareError::WatchNothing(watch_expr.span().unwrap()).into_compile_error(&mut tokens);
    ctx.visit_error_occur = true;
    tokens.into()
  }
}

#[test]
fn local_var() {
  let mut ctx = VisitCtx::default();
  let v = Ident::new("v", proc_macro2::Span::call_site());
  ctx
    .analyze_stack
    .last_mut()
    .unwrap()
    .push(LocalVariable::local(v.clone()));
  ctx.declare_objs.insert(v.clone(), v.clone().into());

  assert!(ctx.find_named_obj(&v).is_none());
}
