use crate::{error::DeclareError, WIDGET_MACRO_NAME};

use super::{
  capture_widget, declare_widget::BuiltinFieldWidgets, ribir_suffix_variable,
  widget_macro::UsedNameInfo, widget_state_ref, NameUsedSpans, WidgetMacro,
};

use proc_macro::{Diagnostic, Level};
use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned};
use std::collections::{HashMap, HashSet};
use syn::{
  parse_quote, parse_quote_spanned, spanned::Spanned, visit_mut, visit_mut::VisitMut, Expr, Ident,
  ItemMacro, Member,
};

#[derive(Clone, PartialEq, Debug)]
pub enum IdType {
  /// name pass by outside `widget!` macro.
  OutsideWidgetMacroPass,
  /// name provide in `track { ... }`
  UserSpecifyTrack,
  /// Declared by `id: name`,
  DeclareDefine,
}
#[derive(Default)]
pub struct DeclareCtx {
  /// All name we need to reactive to its change
  pub named_objects: HashMap<Ident, IdType, ahash::RandomState>,
  pub current_used: HashMap<Ident, Vec<Span>, ahash::RandomState>,
  pub current_capture: HashMap<Ident, Vec<Span>, ahash::RandomState>,
  /// name object has by used.
  used_widgets: HashSet<Ident, ahash::RandomState>,
  analyze_stack: Vec<Vec<LocalVariable>>,
  /// Some builtin widget (like margin, padding) implicit defined by user,
  /// shared the `id` with host widget in user perspective.
  user_perspective_name: HashMap<Ident, Ident>,
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
          *expr = unwrap_expr(self.expand_widget_macro(mac.tokens.clone()));
        } else {
          visit_mut::visit_expr_macro_mut(self, m);
        }
      }
      Expr::Path(p) => {
        visit_mut::visit_expr_path_mut(self, p);
        if let Some(name) = p.path.get_ident() {
          if let Some(name) = self.find_named_widget(name).cloned() {
            self.add_used_widget(name)
          }
        }
      }
      Expr::Closure(c) => {
        let old_follows = std::mem::take(&mut self.current_used);
        let outside_capture = self.current_capture.drain().collect::<Vec<_>>();
        visit_mut::visit_expr_closure_mut(self, c);
        if c.capture.is_some() {
          if !self.current_used.is_empty() {
            let used_widgets = self.current_used.keys();
            let refs = used_widgets.map(widget_state_ref);
            let body = &c.body;
            c.body = parse_quote_spanned! { body.span() => { #(#refs)*  #body }};
          }
          self.current_capture.extend(self.current_used.clone());
          if !self.current_capture.is_empty() {
            let captures = self.current_capture.keys().map(capture_widget);
            *expr = parse_quote_spanned! {c.span() => {
              #(#captures)*
              #c
            }}
          }
        }

        // todo: not capture closure should keep used widget, but not need track its
        // inner used. needn't follow anything of closure inner.
        self.current_used = old_follows;
        self.current_capture.extend(outside_capture);
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
        let res = self.expand_widget_macro(mac.tokens.clone());
        *i = expr_to_stmt(unwrap_expr(res));
        return;
      }
    }
    visit_mut::visit_stmt_mut(self, i);
  }

  fn visit_expr_field_mut(&mut self, f_expr: &mut syn::ExprField) {
    if let Some(mut name) = self.expr_find_name_widget(&f_expr.base).cloned() {
      if let Member::Named(ref field_name) = f_expr.member {
        if let Some(suffix) = BuiltinFieldWidgets::as_builtin_widget(field_name) {
          // fixme: outside pass widget maybe also was declared by user
          if self.named_objects.get(&name) == Some(&IdType::DeclareDefine) {
            name.set_span(name.span().join(field_name.span()).unwrap());
            let wrap_name = ribir_suffix_variable(&name, &suffix.to_string());
            *f_expr.base = parse_quote! { #wrap_name };
            self
              .user_perspective_name
              .insert(wrap_name.clone(), name.clone());
            self.add_used_widget(wrap_name);
            return;
          }
        }
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
  pub fn id_collect(&mut self, d: &WidgetMacro) -> super::Result<()> {
    d.object_names_iter().try_for_each(|(name, track)| {
      if self.named_objects.contains_key(name) {
        Err(DeclareError::DuplicateID([(*name).clone(), name.clone()]))
      } else {
        self.named_objects.insert(name.clone(), track);
        Ok(())
      }
    })
  }

  pub fn is_used(&self, name: &Ident) -> bool { self.used_widgets.contains(name) }

  pub fn user_perspective_name(&self, name: &Ident) -> Option<&Ident> {
    self.user_perspective_name.get(name)
  }

  pub fn take_current_used_info(&mut self) -> UsedNameInfo {
    let follows =
      (!self.current_used.is_empty()).then(|| solid_depends_info(self.current_used.drain()));

    let captures =
      (!self.current_capture.is_empty()).then(|| solid_depends_info(self.current_capture.drain()));

    UsedNameInfo { used_names: follows, captures }
  }

  pub fn clone_current_used_info(&mut self) -> UsedNameInfo {
    let follows = (!self.current_used.is_empty())
      .then(|| solid_depends_info(self.current_used.clone().into_iter()));
    let captures = (!self.current_capture.is_empty())
      .then(|| solid_depends_info(self.current_capture.clone().into_iter()));

    UsedNameInfo { used_names: follows, captures }
  }

  pub fn emit_unused_id_warning(&self) {
    self
      .named_objects
      .iter()
      .filter(|(id, ty)| {
        !self.used_widgets.contains(id)
          && !id.to_string().starts_with('_')
          && *ty != &IdType::OutsideWidgetMacroPass
      })
      .for_each(|(id, _)| {
        Diagnostic::spanned(
          vec![id.span().unwrap()],
          Level::Warning,
          format!("`{}` does not be used", id),
        )
        .span_help(vec![id.span().unwrap()], "Remove this line.")
        .emit()
      });
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
      .or_else(|| self.named_objects.contains_key(ident).then(|| ident))
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

  pub fn add_used_widget(&mut self, name: Ident) {
    Self::add_info(name, &mut self.current_used, &mut self.used_widgets)
  }

  pub fn add_capture(&mut self, name: Ident) {
    Self::add_info(name, &mut self.current_capture, &mut self.used_widgets)
  }

  fn add_info(
    name: Ident,
    info: &mut HashMap<Ident, Vec<Span>, ahash::RandomState>,
    used_widgets: &mut HashSet<Ident, ahash::RandomState>,
  ) {
    info.entry(name.clone()).or_default().push(name.span());
    used_widgets.insert(name);
  }

  fn expand_widget_macro(&mut self, tokens: TokenStream) -> syn::Result<Expr> {
    let mut widget_macro: WidgetMacro = syn::parse2(tokens)?;
    let mut new_ctx = DeclareCtx::default();
    new_ctx.analyze_stack = self.analyze_stack.clone();
    // all named objects should as outside define for embed `widget!` macro.
    self.named_objects.keys().for_each(|name| {
      new_ctx
        .named_objects
        .insert(name.clone(), IdType::OutsideWidgetMacroPass);
    });

    let tokens = widget_macro
      .gen_tokens(&mut new_ctx)
      .unwrap_or_else(|err| err.into_compile_error());

    widget_macro.warnings().for_each(|w| w.emit_warning());
    // inner `widget!` used means need be captured.

    new_ctx.used_widgets.iter().for_each(|name| {
      if self.named_objects.contains_key(name) {
        self.add_capture(name.clone())
      }
    });
    let inner_captures = new_ctx
      .used_widgets
      .iter()
      .filter(|w| self.named_objects.contains_key(w))
      .map(capture_widget);
    let tokens = quote_spanned!(tokens.span()=> {#(#inner_captures)* #tokens} );

    syn::parse2(tokens)
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

impl<'a> std::ops::Deref for CaptureScopeGuard<'a> {
  type Target = DeclareCtx;

  fn deref(&self) -> &Self::Target { self.ctx }
}

impl<'a> std::ops::DerefMut for CaptureScopeGuard<'a> {
  fn deref_mut(&mut self) -> &mut Self::Target { self.ctx }
}

fn solid_depends_info(info: impl Iterator<Item = (Ident, Vec<Span>)>) -> Vec<NameUsedSpans> {
  info
    .map(|(widget, spans)| NameUsedSpans {
      widget,
      spans: spans.into_boxed_slice(),
    })
    .collect()
}
