use crate::error::DeclareError;

use super::{ribir_suffix_variable, sugar_fields::SugarFields, FollowOn, WidgetMacro};

use proc_macro::{Diagnostic, Level};
use proc_macro2::Span;
use quote::quote;
use std::collections::{HashMap, HashSet};
use syn::{parse_quote, visit_mut, visit_mut::VisitMut, Expr, Ident};

const DECLARE_MACRO_NAME: &str = "declare";

pub struct DeclareCtx {
  /// All name defined in `declare!` by `id`.
  pub named_objects: HashSet<Ident>,
  pub current_follows: HashMap<Ident, Vec<Span>>,
  // Key is the name of widget which has been depended by other, and value is a bool represent if
  // it's depended directly or just be depended by its wrap widget, if guard or child gen
  // expression.
  be_followed: HashMap<Ident, ReferenceInfo>,
  analyze_stack: Vec<Vec<LocalVariable>>,
  forbid_warnings: bool,
  /// Some wrap widget (like margin, padding) implicit defined by user, shared
  /// the `id` with host widget in user perspective.
  user_perspective_name: HashMap<Ident, Ident>,
  id_capture_scope: Vec<bool>,
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
#[derive(Debug)]
struct LocalVariable {
  name: Ident,
  alias_of_name: Option<Ident>,
}

impl VisitMut for DeclareCtx {
  fn visit_expr_mut(&mut self, expr: &mut Expr) {
    match expr {
      Expr::Macro(m) if m.mac.path.is_ident(DECLARE_MACRO_NAME) => {
        let tokens = std::mem::replace(&mut m.mac.tokens, quote! {});
        *expr = self.extend_declare_macro_to_expr(tokens.into());
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
    match i {
      syn::Stmt::Item(syn::Item::Macro(m))
        if m.ident.is_none() && m.mac.path.is_ident(DECLARE_MACRO_NAME) =>
      {
        let tokens = std::mem::replace(&mut m.mac.tokens, quote! {});
        let expr = self.extend_declare_macro_to_expr(tokens.into());
        *i = syn::Stmt::Expr(expr);
      }
      _ => {
        visit_mut::visit_stmt_mut(self, i);
      }
    }
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
    self.stack_push();
    visit_mut::visit_block_mut(self, i);
    self.stack_pop();
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
    self.stack_push();
    visit_mut::visit_expr_block_mut(self, i);
    self.stack_pop();
  }

  fn visit_expr_for_loop_mut(&mut self, i: &mut syn::ExprForLoop) {
    self.stack_push();
    visit_mut::visit_expr_for_loop_mut(self, i);
    self.stack_pop();
  }

  fn visit_expr_loop_mut(&mut self, i: &mut syn::ExprLoop) {
    self.stack_push();
    visit_mut::visit_expr_loop_mut(self, i);
    self.stack_pop();
  }

  fn visit_expr_if_mut(&mut self, i: &mut syn::ExprIf) {
    self.stack_push();
    visit_mut::visit_expr_if_mut(self, i);
    self.stack_pop();
  }

  fn visit_arm_mut(&mut self, i: &mut syn::Arm) {
    self.stack_push();
    visit_mut::visit_arm_mut(self, i);
    self.stack_pop();
  }

  fn visit_expr_unsafe_mut(&mut self, i: &mut syn::ExprUnsafe) {
    self.stack_push();
    visit_mut::visit_expr_unsafe_mut(self, i);
    self.stack_pop();
  }

  fn visit_expr_while_mut(&mut self, i: &mut syn::ExprWhile) {
    self.stack_push();
    visit_mut::visit_expr_while_mut(self, i);
    self.stack_pop();
  }

  #[track_caller]
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
    if self.forbid_warnings {
      return;
    }
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

  pub fn forbid_warnings(&mut self, b: bool) { self.forbid_warnings = b; }

  pub fn borrow_capture_scope(&mut self, capture_scope: bool) -> CaptureScopeGuard {
    CaptureScopeGuard::new(self, capture_scope)
  }

  pub fn stack_push(&mut self) { self.analyze_stack.push(vec![]); }

  pub fn stack_pop(&mut self) { self.analyze_stack.pop(); }

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
}

impl Default for DeclareCtx {
  fn default() -> Self {
    Self {
      named_objects: Default::default(),
      current_follows: Default::default(),
      be_followed: Default::default(),
      analyze_stack: Default::default(),
      forbid_warnings: Default::default(),
      user_perspective_name: Default::default(),
      id_capture_scope: Default::default(),
    }
  }
}

pub struct CaptureScopeGuard<'a> {
  ctx: &'a mut DeclareCtx,
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
