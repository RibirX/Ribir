use crate::{
  widget_attr_macro::{declare_widget::DeclareWidget, widget_def_variable},
  error::DeclareError,
  WIDGET_MACRO_NAME,
};

use super::{
  declare_widget::SugarFields, ribir_suffix_variable, FollowOn, WidgetMacro, DECLARE_WRAP_MACRO,
};

use proc_macro::{Diagnostic, Level};
use proc_macro2::{Span, TokenStream};
use quote::quote;
use std::collections::{HashMap, HashSet};
use syn::{parse_quote, visit_mut, visit_mut::VisitMut, Expr, Ident, ItemMacro};

pub struct DeclareCtx {
  /// All name defined in `widget!` by `id`.
  pub named_objects: HashSet<Ident>,
  pub current_follows: HashMap<Ident, Vec<Span>>,
  // Key is the name of widget which has been depended by other, and value is a bool represent if
  // it's depended directly or just be depended by its wrap widget, if guard or child gen
  // expression.
  be_followed: HashMap<Ident, ReferenceInfo>,
  analyze_stack: Vec<Vec<LocalVariable>>,
  /// Some wrap widget (like margin, padding) implicit defined by user, shared
  /// the `id` with host widget in user perspective.
  user_perspective_name: HashMap<Ident, Ident>,
  id_capture_scope: Vec<bool>,
  ctx_name: Ident,
  _self_name: Ident,
}

#[derive(PartialEq, Clone, Copy)]
pub enum ReferenceInfo {
  // reference by other expression, but not follow its change and needn't capture its state
  // reference
  Reference,
  // Followed by others and need follow its change or need capture its state reference to modify
  // its state.
  BeFollowed,
  // not be referenced or followed, but its wrap widget maybe.
  WrapWidgetRef,
}
#[derive(Debug, Clone)]
struct LocalVariable {
  name: Ident,
  alias_of_name: Option<Ident>,
}

impl VisitMut for DeclareCtx {
  fn visit_expr_mut(&mut self, expr: &mut Expr) {
    match expr {
      Expr::Macro(m) => {
        let mac = &m.mac;
        if mac.path.is_ident(WIDGET_MACRO_NAME) {
          *expr = unwrap_expr(self.expand_widget_macro(quote! {#mac}));
        } else if mac.path.is_ident(DECLARE_WRAP_MACRO) {
          *expr = unwrap_expr(self.expand_declare_wrap_macro(mac.tokens.clone()));
        } else {
          visit_mut::visit_expr_macro_mut(self, m);
        }
      }
      Expr::Path(p) => {
        visit_mut::visit_expr_path_mut(self, p);
        if let Some(name) = p.path.get_ident() {
          if let Some(name) = self.find_named_widget(name).cloned() {
            self.add_follow(name)
          }
        }
      }
      _ => {
        visit_mut::visit_expr_mut(self, expr);
      }
    }
  }

  fn visit_stmt_mut(&mut self, i: &mut syn::Stmt) {
    if let syn::Stmt::Item(syn::Item::Macro(ItemMacro { ident: None, mac, semi_token, .. })) = i {
      let mut expr_to_stmt = |expr| {
        if let Some(semi) = semi_token.take() {
          syn::Stmt::Semi(expr, semi)
        } else {
          syn::Stmt::Expr(expr)
        }
      };
      if mac.path.is_ident(WIDGET_MACRO_NAME) {
        let res = self.expand_widget_macro(quote! {#mac});
        *i = expr_to_stmt(unwrap_expr(res));
        return;
      } else if mac.path.is_ident(DECLARE_WRAP_MACRO) {
        let res = self.expand_declare_wrap_macro(mac.tokens.clone());
        *i = expr_to_stmt(unwrap_expr(res));
        return;
      }
    }
    visit_mut::visit_stmt_mut(self, i);
  }

  fn visit_expr_field_mut(&mut self, f_expr: &mut syn::ExprField) {
    if let Some(mut name) = self.expr_find_name_widget(&f_expr.base).cloned() {
      if let Some(suffix) = SugarFields::wrap_widget_from_member(&f_expr.member) {
        name.set_span(name.span().join(suffix.span()).unwrap());
        let wrap_name = ribir_suffix_variable(&name, &suffix.to_string());
        *f_expr.base = parse_quote! { #wrap_name };
        self
          .user_perspective_name
          .insert(wrap_name.clone(), name.clone());
        self.add_follow(wrap_name);
        self.add_reference(name, ReferenceInfo::WrapWidgetRef);
        return;
      }
    }
    visit_mut::visit_expr_field_mut(self, f_expr);
  }

  fn visit_expr_assign_mut(&mut self, assign: &mut syn::ExprAssign) {
    visit_mut::visit_expr_assign_mut(self, assign);

    let local_alias = self.expr_find_name_widget(&assign.left).and_then(|local| {
      self
        .expr_find_name_widget(&assign.right)
        .map(|named| (local.clone(), named.clone()))
    });
    if let Some((local, named)) = local_alias {
      let local_var = self
        .analyze_stack
        .iter_mut()
        .rev()
        .flat_map(|locals| locals.iter_mut().rev())
        .find(|v| v.name == local);
      if let Some(local_var) = local_var {
        local_var.alias_of_name = Some(named);
      }
    }
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
    visit_mut::visit_local_mut(self, local);

    if let Some((_, init)) = &local.init {
      let right_name = self.expr_find_name_widget(&*init).cloned();
      let var_name = self.analyze_stack.last_mut().unwrap().last_mut();
      // var_name maybe none if
      // `let _ = xxx`
      if let Some(var) = var_name {
        var.alias_of_name = right_name;
      }
    }
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
      .push(LocalVariable {
        name: i.ident.clone(),
        alias_of_name: None,
      });
  }
}

impl DeclareCtx {
  pub fn new(self_name: Ident, ctx_name: Ident) -> Self {
    Self {
      named_objects: Default::default(),
      current_follows: Default::default(),
      be_followed: Default::default(),
      analyze_stack: vec![],
      user_perspective_name: Default::default(),
      id_capture_scope: Default::default(),
      ctx_name,
      _self_name: self_name,
    }
  }

  pub fn id_collect(&mut self, d: &WidgetMacro) -> super::Result<()> {
    d.object_names_iter().try_for_each(|name| {
      if let Some(old) = self.named_objects.get(name) {
        Err(DeclareError::DuplicateID([(*old).clone(), name.clone()]))
      } else {
        self.named_objects.insert(name.clone());
        Ok(())
      }
    })
  }

  pub fn ctx_name(&self) -> &Ident {
    // todo: track ctx rename
    &self.ctx_name
  }

  pub fn _self_name(&self) -> &Ident {
    // todo: track self rename
    &self._self_name
  }

  pub fn be_followed(&self, name: &Ident) -> bool {
    self
      .be_followed
      .get(name)
      .map(|r| r == &ReferenceInfo::BeFollowed)
      .unwrap_or(false)
  }

  pub fn be_reference(&self, name: &Ident) -> bool {
    self
      .be_followed
      .get(name)
      .map(|r| r == &ReferenceInfo::Reference)
      .unwrap_or(false)
  }

  pub fn user_perspective_name(&self, name: &Ident) -> Option<&Ident> {
    self.user_perspective_name.get(name)
  }

  pub fn take_current_follows(&mut self) -> Option<Vec<FollowOn>> {
    (!self.current_follows.is_empty()).then(|| {
      self
        .current_follows
        .drain()
        .map(|(widget, spans)| FollowOn {
          widget,
          spans: spans.into_iter().collect(),
        })
        .collect()
    })
  }

  pub fn emit_unused_id_warning(&self) {
    self
      .named_objects
      .iter()
      .filter(|k| !self.be_followed.contains_key(k) && !k.to_string().starts_with('_'))
      .for_each(|id| {
        Diagnostic::spanned(
          vec![id.span().unwrap()],
          Level::Warning,
          format!("`{}` does not be used", id),
        )
        .span_help(vec![id.span().unwrap()], "Remove this line.")
        .emit()
      });
  }

  pub fn borrow_capture_scope(&mut self, capture_scope: bool) -> CaptureScopeGuard {
    CaptureScopeGuard::new(self, capture_scope)
  }

  pub fn stack_push(&mut self) -> StackGuard { StackGuard::new(self) }

  // return the name of widget that `ident` point to if it's have.
  pub fn find_named_widget<'a>(&'a self, ident: &'a Ident) -> Option<&'a Ident> {
    self
      .analyze_stack
      .iter()
      .rev()
      .flat_map(|local| local.iter().rev())
      .find(|v| &v.name == ident)
      .and_then(|v| v.alias_of_name.as_ref())
      .or_else(|| self.named_objects.contains(ident).then(|| ident))
  }

  pub fn expr_find_name_widget<'a>(&'a self, expr: &'a Expr) -> Option<&'a Ident> {
    if let Expr::Path(syn::ExprPath { path, .. }) = expr {
      path
        .get_ident()
        .and_then(|name| self.find_named_widget(name))
    } else {
      None
    }
  }

  pub fn add_follow(&mut self, name: Ident) {
    self
      .current_follows
      .entry(name.clone())
      .or_default()
      .push(name.span());

    let in_follow_scope = self.id_capture_scope.last().cloned().unwrap_or(true);
    self.add_reference(
      name,
      if in_follow_scope {
        ReferenceInfo::BeFollowed
      } else {
        ReferenceInfo::Reference
      },
    );
  }

  pub fn add_reference(&mut self, name: Ident, ref_info: ReferenceInfo) {
    let v = self
      .be_followed
      .entry(name)
      .or_insert(ReferenceInfo::Reference);
    match (*v, ref_info) {
      (ReferenceInfo::Reference, _) => *v = ref_info,
      (ReferenceInfo::WrapWidgetRef, _) => *v = ref_info,
      _ => {}
    }
  }

  fn expand_widget_macro(&mut self, tokens: TokenStream) -> syn::Result<Expr> {
    let mut widget_macro: WidgetMacro = syn::parse2(tokens)?;
    let named = self.named_objects.clone();

    let mut ctx = self.borrow_capture_scope(true);
    let tokens = widget_macro
      .gen_tokens(&mut *ctx)
      .unwrap_or_else(|err| err.into_compile_error());

    // trigger warning and restore named widget.
    named.iter().for_each(|k| {
      ctx.named_objects.remove(k);
    });

    ctx.emit_unused_id_warning();
    widget_macro.warnings().for_each(|w| w.emit_warning());
    ctx.named_objects = named;
    syn::parse2(tokens)
  }

  fn expand_declare_wrap_macro(&mut self, tokens: TokenStream) -> syn::Result<Expr> {
    let mut widget: DeclareWidget = syn::parse2(tokens)?;
    let mut ctx = self.borrow_capture_scope(true);
    ctx.visit_declare_widget_mut(&mut widget);
    let tokens = widget.widget_full_tokens(&*ctx);
    let name = widget.widget_identify();
    let name = widget_def_variable(&name);
    syn::parse2(quote! {{ #tokens #name }})
  }
}

fn unwrap_expr(res: syn::Result<Expr>) -> Expr {
  match res {
    Ok(expr) => expr,
    Err(e) => {
      let tokens = e.into_compile_error();
      parse_quote!(#tokens)
    }
  }
}

pub struct StackGuard<'a> {
  ctx: &'a mut DeclareCtx,
}

pub struct CaptureScopeGuard<'a> {
  ctx: &'a mut DeclareCtx,
}

impl<'a> StackGuard<'a> {
  pub fn new(ctx: &'a mut DeclareCtx) -> Self {
    ctx.analyze_stack.push(vec![]);
    StackGuard { ctx }
  }
}

impl<'a> Drop for StackGuard<'a> {
  fn drop(&mut self) { self.ctx.analyze_stack.pop(); }
}

impl<'a> std::ops::Deref for StackGuard<'a> {
  type Target = DeclareCtx;

  fn deref(&self) -> &Self::Target { self.ctx }
}

impl<'a> std::ops::DerefMut for StackGuard<'a> {
  fn deref_mut(&mut self) -> &mut Self::Target { self.ctx }
}

impl<'a> CaptureScopeGuard<'a> {
  pub fn new(ctx: &'a mut DeclareCtx, follow_scope: bool) -> Self {
    ctx.id_capture_scope.push(follow_scope);
    CaptureScopeGuard { ctx }
  }
}

impl<'a> Drop for CaptureScopeGuard<'a> {
  fn drop(&mut self) { self.ctx.id_capture_scope.pop(); }
}

impl<'a> std::ops::Deref for CaptureScopeGuard<'a> {
  type Target = DeclareCtx;

  fn deref(&self) -> &Self::Target { self.ctx }
}

impl<'a> std::ops::DerefMut for CaptureScopeGuard<'a> {
  fn deref_mut(&mut self) -> &mut Self::Target { self.ctx }
}
