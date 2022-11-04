use crate::WIDGET_MACRO_NAME;

use super::{
  builtin_var_name, capture_widget,
  desugar::{ComposeItem, DeclareObj, Field, FieldValue, NamedObj, SubscribeItem, WidgetNode},
  gen_widget_macro, Desugared, ScopeUsedInfo, TrackExpr, UsedType, WIDGET_OF_BUILTIN_FIELD,
  WIDGET_OF_BUILTIN_METHOD,
};

use proc_macro::Span;
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use std::{
  collections::{HashMap, HashSet},
  hash::Hash,
};
use syn::{
  parse_quote, parse_quote_spanned, spanned::Spanned, visit_mut, visit_mut::VisitMut, Expr,
  ExprMethodCall, Ident, ItemMacro, Member, Path, Stmt,
};

bitflags::bitflags! {
  pub struct IdType: u16 {
    /// Declared by `id: name`,
    const DECLARE = 0x001;
    /// name provide in `track { ... }`
    const USER_SPECIFY = 0x010;
      /// name pass by outside `widget!` macro.
    const FROM_ANCESTOR = 0x100;
  }
}

pub struct VisitCtx {
  /// All declared object.
  pub declare_objs: HashMap<Ident, Path, ahash::RandomState>,
  pub track_names: HashSet<Ident, ahash::RandomState>,
  pub current_used_info: ScopeUsedInfo,
  /// name object has be used and its source name.
  pub used_objs: HashMap<Ident, UsedInfo, ahash::RandomState>,
  pub analyze_stack: Vec<Vec<LocalVariable>>,
}

#[derive(Debug, Clone)]
pub struct LocalVariable {
  name: Ident,
  alias_of_name: Option<Ident>,
}

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
      track_names: <_>::default(),
      current_used_info: Default::default(),
      used_objs: Default::default(),
      analyze_stack: vec![vec![]],
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
        let mut outside_used = self.current_used_info.take();
        visit_mut::visit_expr_closure_mut(self, c);
        let mut overwrite_inner_used = UsedType::CAPTURE;
        if c.capture.is_some() {
          if self.current_used_info.refs_widgets().is_some() {
            let mut body_tokens = quote! {};
            self.current_used_info.value_expr_surround_refs(
              &mut body_tokens,
              c.body.span(),
              |tokens| c.body.to_tokens(tokens),
            );
            c.body = parse_quote!(#body_tokens);
          }

          if let Some(all) = self.current_used_info.all_used() {
            let captures = all.map(capture_widget);
            *expr = parse_quote_spanned! {c.span() => {
              #(#captures)*
              #c
            }}
          }
          overwrite_inner_used = UsedType::MOVE_CAPTURE;
        }

        self.current_used_info.iter_mut().for_each(|(_, info)| {
          info.used_type = overwrite_inner_used;
        });

        outside_used.merge(&self.current_used_info);
        self.current_used_info = outside_used;
      }
      _ => {
        visit_mut::visit_expr_mut(self, expr);
      }
    }
  }

  fn visit_stmt_mut(&mut self, i: &mut Stmt) {
    if let syn::Stmt::Item(syn::Item::Macro(ItemMacro { ident: None, mac, .. })) = i {
      if mac.path.is_ident(WIDGET_MACRO_NAME) {
        let expr: TokenStream = gen_widget_macro(mac.tokens.clone().into(), Some(self)).into();
        *i = Stmt::Expr(Expr::Verbatim(expr));
        return;
      }
    }
    visit_mut::visit_stmt_mut(self, i);
  }

  fn visit_expr_field_mut(&mut self, f_expr: &mut syn::ExprField) {
    if let Member::Named(member) = &f_expr.member {
      if let Some(builtin_ty) = WIDGET_OF_BUILTIN_FIELD.get(member.to_string().as_str()) {
        let span = f_expr.span();
        if self
          .visit_builtin_member(&mut f_expr.base, span, builtin_ty)
          .is_some()
        {
          return;
        }
      }
    }

    visit_mut::visit_expr_field_mut(self, f_expr);
  }

  fn visit_expr_method_call_mut(&mut self, i: &mut ExprMethodCall) {
    if let Some(builtin_ty) = WIDGET_OF_BUILTIN_METHOD.get(i.method.to_string().as_str()) {
      let span = i.span();
      if self
        .visit_builtin_member(&mut i.receiver, span, builtin_ty)
        .is_some()
      {
        return;
      }
    }

    visit_mut::visit_expr_method_call_mut(self, i);
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
      let right_name = self.expr_find_name_widget(init).cloned();
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

pub const DYN_WIDGET: &str = "DynWidget";
fn is_dyn_widget_keyword(ty: &Path) -> bool { ty.get_ident().map_or(false, |ty| ty == DYN_WIDGET) }

impl VisitCtx {
  pub fn visit_desugared_syntax_mut(&mut self, desugar: &mut Desugared) {
    desugar.named_objs.objs_mut().for_each(|obj| match obj {
      NamedObj::Host(obj) | NamedObj::Builtin { obj, .. } => self.visit_declare_obj(obj),
      NamedObj::DuplicateListener { objs, .. } => {
        objs.iter_mut().for_each(|obj| self.visit_declare_obj(obj))
      }
    });

    desugar
      .stmts
      .iter_mut()
      .for_each(|item| self.visit_subscribe_item_mut(item));

    self.take_current_used_info();

    self.visit_widget_node_mut(&mut desugar.widget.as_mut().unwrap());
  }
  pub fn visit_declare_obj(&mut self, obj: &mut DeclareObj) {
    let DeclareObj { ty, fields, .. } = obj;
    self.visit_path_mut(ty);
    if is_dyn_widget_keyword(ty) {
      *ty = parse_quote_spanned! { ty.span() => #ty::<_> };
    }
    fields.iter_mut().for_each(|f| self.visit_field(f));
  }

  pub fn visit_subscribe_item_mut(&mut self, item: &mut SubscribeItem) {
    match item {
      SubscribeItem::Obj(obj) => self.visit_declare_obj(obj),
      SubscribeItem::ObserveModifyDo { observe, subscribe_do, .. }
      | SubscribeItem::ObserveChangeDo { observe, subscribe_do, .. } => {
        self.visit_track_expr(observe);
        self.visit_track_expr(subscribe_do);
      }
      SubscribeItem::LetVar { value, .. } => self.visit_track_expr(value),
    }
  }

  pub fn visit_track_expr(&mut self, expr: &mut TrackExpr) {
    self.visit_expr_mut(&mut expr.expr);
    expr.used_name_info = self.take_current_used_info();
  }

  pub fn visit_widget_node_mut(&mut self, widget: &mut WidgetNode) {
    let WidgetNode { parent, children } = widget;
    self.visit_compose_item_mut(parent);
    children
      .iter_mut()
      .for_each(|node| self.visit_widget_node_mut(node));
  }

  pub fn visit_compose_item_mut(&mut self, widget: &mut ComposeItem) {
    match widget {
      ComposeItem::ChainObjs(objs) => objs.iter_mut().for_each(|obj| self.visit_declare_obj(obj)),
      ComposeItem::Id(_) => {}
    }
  }

  pub fn visit_field(&mut self, field: &mut Field) { self.visit_field_value(&mut field.value) }

  pub fn visit_field_value(&mut self, value: &mut FieldValue) {
    match value {
      FieldValue::Expr(e) => self.visit_track_expr(e),
      FieldValue::Obj(obj) => self.visit_declare_obj(obj),
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
      .and_then(|v| v.alias_of_name.as_ref())
      .or_else(|| {
        (self.declare_objs.contains_key(ident) || self.track_names.contains(ident)).then(|| ident)
      })
  }

  pub fn expr_find_name_widget<'a>(&'a self, expr: &'a Expr) -> Option<&'a Ident> {
    if let Expr::Path(syn::ExprPath { path, .. }) = expr {
      path.get_ident().and_then(|name| self.find_named_obj(name))
    } else {
      None
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

  fn visit_builtin_member(
    &mut self,
    expr: &mut syn::Expr,
    span: proc_macro2::Span,
    builtin_ty: &'static str,
  ) -> Option<()> {
    let path = match expr {
      Expr::Path(syn::ExprPath { path, .. }) => path,
      Expr::MethodCall(ExprMethodCall { receiver, method, args, .. })
        if args.is_empty() && (method == "shallow" || method == "silent") =>
      {
        if let Expr::Path(syn::ExprPath { path, .. }) = &mut **receiver {
          path
        } else {
          return None;
        }
      }
      _ => return None,
    };
    let name = path.get_ident()?;
    let name = self.find_named_obj(name)?;

    if self
      .declare_objs
      .get(&name)
      .map_or(false, |ty| !ty.is_ident(builtin_ty))
    {
      let builtin_name = builtin_var_name(&name, span, builtin_ty);
      let src_name = name.clone();
      *path = parse_quote! { #builtin_name };

      self.add_used_widget(
        builtin_name,
        Some(BuiltinUsed { src_name, builtin_ty }),
        UsedType::USED,
      );
      Some(())
    } else {
      None
    }
  }
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
