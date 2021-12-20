use crate::error::DeclareError;

use super::{
  sugar_fields::{Id, SugarFields},
  DataFlow, DeclareField, DeclareMacro, DeclareWidget, FollowOnVec,
};
use proc_macro::{Diagnostic, Level, TokenStream};
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{quote, quote_spanned};
use std::collections::{HashMap, HashSet};
use syn::{
  parse_quote, spanned::Spanned, visit_mut, visit_mut::VisitMut, Expr, ExprClosure, Ident,
};

const DECLARE_MACRO_NAME: &'static str = "declare";

#[derive(Default)]
pub struct DeclareCtx {
  // Key is the name of widget which has been depended by other, and value is a bool represent if
  // it's depended directly or just be depended by its wrap widget, if guard or child gen
  // expression.
  pub be_followed: HashMap<Ident, bool>,
  // the key is the widget name which depends to the value
  pub named_widgets: HashSet<Ident>,
  pub current_follows: HashMap<Ident, Vec<Span>>,
  pub contain_capture_closure: bool,
  analyze_stack: Vec<Vec<LocalVariable>>,
  forbid_warnings: bool,
  widget_name_to_id: HashMap<Ident, Ident>,
  follow_scopes: Vec<bool>,
}

#[derive(Debug)]
struct LocalVariable {
  name: Ident,
  alias_of_name: Option<Ident>,
}

pub fn state_ref_tokens<'a, I: IntoIterator<Item = &'a Ident>>(follows: I) -> TokenStream2 {
  let ref_cells = follows.into_iter().map(|follow_w| {
    quote_spanned! { follow_w.span() => let #follow_w = #follow_w.ref_cell(); }
  });

  quote! { #(#ref_cells)*}
}

pub fn state_mut_ref<'a, I: IntoIterator<Item = &'a Ident>>(
  follows: I,
  ctx: &DeclareCtx,
) -> TokenStream2 {
  let ref_cells = follows.into_iter().map(|ref_name| {
    // named widget can use and must use StateRefCell to access, because is may
    // captured in closure.
    if ctx.be_followed(ref_name) {
      quote_spanned! { ref_name.span() =>
        #[allow(unused_mut)]
        let mut #ref_name = #ref_name.borrow_mut();
      }
    } else {
      let def_name = ctx.no_conflict_widget_def_name(&ref_name);

      // We should not allow modify the widget in the declare macro except is across
      // StateRefCell.
      quote_spanned! { ref_name.span() =>
        let #ref_name = &#def_name;
      }
    }
  });

  quote! { #(#ref_cells)*}
}

impl VisitMut for DeclareCtx {
  fn visit_expr_mut(&mut self, expr: &mut Expr) {
    match expr {
      Expr::Closure(c) if c.capture.is_some() => {
        self.visit_capture_mut_and_expand(c);
      }
      Expr::Macro(m) if m.mac.path.is_ident(DECLARE_MACRO_NAME) => {
        let tokens = std::mem::replace(&mut m.mac.tokens, quote! {});
        *expr = self.extend_declare_macro_to_expr(tokens.into());
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
    visit_mut::visit_expr_field_mut(self, f_expr);

    if let Some(name) = self.expr_find_name_widget(&f_expr.base).cloned() {
      if let Some(suffix) = SugarFields::as_widget_wrap_name_field(&f_expr.member) {
        let wrap_name = self.no_conflict_name_with_suffix(&name, suffix);
        *f_expr.base = parse_quote! { #wrap_name };
        self
          .widget_name_to_id
          .insert(wrap_name.clone(), name.clone());
        self.add_depend(wrap_name);
        self.add_be_depended(name, false);
      } else {
        self.add_depend(name);
      }
    }
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
        .map(|locals| locals.iter_mut().rev())
        .flatten()
        .find(|v| v.name == local);
      if let Some(local_var) = local_var {
        local_var.alias_of_name = Some(named.clone());
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

  fn visit_expr_method_call_mut(&mut self, i: &mut syn::ExprMethodCall) {
    visit_mut::visit_expr_method_call_mut(self, i);
    if let Some(name) = self.expr_find_name_widget(&i.receiver).cloned() {
      self.add_depend(name);
    }
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
      .expect(&format!(
        "Crash when visit `{}`, stack should not be empty, at {}:{}:{}",
        quote! { #i },
        file!(),
        line!(),
        column!()
      ))
      .push(LocalVariable {
        name: i.ident.clone(),
        alias_of_name: None,
      });
  }
}

impl DeclareCtx {
  fn extend_declare_macro_to_expr(&mut self, tokens: TokenStream) -> Expr {
    let mut declare: DeclareMacro = syn::parse(tokens).expect("extend declare macro failed!");
    let named = self.named_widgets.clone();
    self.save_follow_scope(true);
    let tokens = declare.gen_tokens(self).unwrap_or_else(|err| {
      // forbid warning.
      self.forbid_warnings(true);
      err.into_compile_error(self, &declare)
    });
    self.pop_follow_scope();

    // trigger warning and restore named widget.
    named.iter().for_each(|k| {
      self.named_widgets.remove(k);
    });
    self.emit_unused_id_warning();
    self.named_widgets = named;

    parse_quote!(#tokens)
  }

  fn visit_capture_mut_and_expand(&mut self, c: &mut ExprClosure) {
    let mut old_follows = std::mem::take(&mut self.current_follows);
    visit_mut::visit_expr_closure_mut(self, c);

    let closure_follows = std::mem::take(&mut self.current_follows);
    if !closure_follows.is_empty() {
      let mut_refs = state_mut_ref(closure_follows.keys(), self);
      let mut_refs = quote_spanned! { c.body.span() => #mut_refs };

      let body = &mut c.body;
      let body_tokens = quote_spanned! { body.span() => {
        #mut_refs
        #body
      }};
      *body = Box::new(parse_quote! { #body_tokens });
      self.contain_capture_closure = true;
      old_follows.extend(closure_follows);
    }

    self.current_follows = old_follows;
  }
}

impl DeclareCtx {
  pub fn visit_declare_macro_mut(&mut self, d: &mut DeclareMacro) {
    self.visit_declare_widget_mut(&mut d.widget);
    d.data_flows
      .iter_mut()
      .for_each(|df| self.visit_data_flows_mut(df));
  }

  pub fn visit_data_flows_mut(&mut self, df: &mut DataFlow) {
    self.visit_expr_mut(&mut df.from.expr);
    df.from.follows = self.take_current_follows();
    self.visit_expr_mut(&mut df.to.expr);
    df.to.follows = self.take_current_follows();
  }

  pub fn visit_declare_field_mut(&mut self, f: &mut DeclareField) {
    self.visit_ident_mut(&mut f.member);
    if let Some(if_guard) = f.if_guard.as_mut() {
      self.save_follow_scope(false);
      self.visit_expr_mut(&mut if_guard.cond);
      self.pop_follow_scope()
    }
    self.visit_expr_mut(&mut f.expr);

    f.follows = self.take_current_follows();
    if let Some(follows) = f.follows.as_ref() {
      let refs = if self.contain_capture_closure {
        state_ref_tokens(follows.names())
      } else {
        state_mut_ref(follows.names(), self)
      };
      let expr = &mut f.expr;
      let expr_tokens = quote_spanned! { expr.span() => {
        #refs
        #expr
      }};
      *expr = parse_quote! { #expr_tokens };
    }
    self.contain_capture_closure = false;
  }

  pub fn visit_declare_widget_mut(&mut self, w: &mut DeclareWidget) {
    fn visit_self_only(w: &mut DeclareWidget, ctx: &mut DeclareCtx) {
      ctx.stack_push();
      w.fields
        .iter_mut()
        .for_each(|f| ctx.visit_declare_field_mut(f));
      if let Some(rest_expr) = &mut w.rest {
        ctx.visit_expr_mut(&mut rest_expr.1);
      }
      ctx.visit_sugar_field_mut(&mut w.sugar_fields);
      if let Some(Id { name, .. }) = w.named.as_ref() {
        let followed_by_attr = w
          .sugar_fields
          .normal_attr_iter()
          .chain(w.sugar_fields.listeners_iter())
          .any(|f| f.follows.is_some());
        if followed_by_attr {
          ctx.add_depend(name.clone());
        }
      }

      ctx.stack_pop()
    }
    visit_self_only(w, self);
    w.children.iter_mut().for_each(|c| match c {
      super::Child::Declare(d) => visit_self_only(d, self),
      super::Child::Expr(expr) => {
        let old_follows = std::mem::take(&mut self.current_follows);
        self.stack_push();
        self.save_follow_scope(false);
        self.visit_expr_mut(expr);
        if let Some(follows) = self.take_current_follows() {
          let mut_refs = state_mut_ref(follows.names(), self);
          *expr = parse_quote! {{
            #mut_refs
            #expr
          }};
        }
        self.current_follows = old_follows;
        self.pop_follow_scope();
        self.stack_pop(); 
      }
    })
  }

  pub fn visit_sugar_field_mut(&mut self, sugar_field: &mut SugarFields) {
    sugar_field.visit_sugar_field_mut(self);
  }
}

impl DeclareCtx {
  pub fn id_collect(&mut self, widget: &DeclareWidget) -> super::Result<()> {
    widget.recursive_call(|w| {
      if let Some(Id { name, .. }) = w.named.as_ref() {
        if let Some(old) = self.named_widgets.get(name) {
          return Err(DeclareError::DuplicateID([(*old).clone(), name.clone()]));
        } else {
          self.named_widgets.insert(name.clone());
        }
      }
      Ok(())
    })
  }

  pub fn be_followed(&self, name: &Ident) -> bool {
    self.be_followed.get(name).cloned().unwrap_or(false)
  }

  pub fn widget_name_to_id<'a>(&'a self, name: &'a Ident) -> &'a Ident {
    self.widget_name_to_id.get(name).unwrap_or(name)
  }

  pub fn take_current_follows(&mut self) -> Option<FollowOnVec> {
    (!self.current_follows.is_empty()).then(|| self.current_follows.drain().into())
  }

  pub fn emit_unused_id_warning(&self) {
    if self.forbid_warnings {
      return;
    }
    self
      .named_widgets
      .iter()
      .filter(|k| !self.be_followed.contains_key(k) && !k.to_string().starts_with("_"))
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

  pub fn no_conflict_widget_def_name(&self, name: &Ident) -> Ident {
    let def_name = format!("{}_def", name);
    self.new_no_conflict_name(&def_name)
  }

  pub fn unnamed_widget_ref_name(&self) -> Ident { self.new_no_conflict_name("w") }

  // Get a no conflict name for a widget wrap by the common widget like `Margin`,
  // `Padding`.
  pub fn no_conflict_name_with_suffix(&self, widget_name: &Ident, suffix: &Ident) -> Ident {
    let mut wrap_name = self.new_no_conflict_name(&format!("{}_{}", widget_name, &suffix));
    let span1 = widget_name.span();
    let span2 = suffix.span();
    wrap_name.set_span(span1.join(span2).unwrap_or(span2));
    wrap_name
  }

  pub fn no_config_builder_type_name(&self) -> Ident { self.new_no_conflict_name("Builder") }

  pub fn no_conflict_child_name(&self, idx: usize) -> Ident {
    self.new_no_conflict_name(&format!("c{}", idx))
  }

  pub fn forbid_warnings(&mut self, b: bool) { self.forbid_warnings = b; }

  pub fn new_no_conflict_name(&self, name: &str) -> Ident {
    let mut name = Ident::new(name, Span::call_site());
    while self.named_widgets.contains(&name) {
      let suffix = format! {"{}_", name};
      name = Ident::new(&suffix, Span::call_site())
    }
    name
  }

  fn save_follow_scope(&mut self, follow_scope: bool) { self.follow_scopes.push(follow_scope); }

  fn pop_follow_scope(&mut self) { self.follow_scopes.pop(); }

  fn stack_push(&mut self) { self.analyze_stack.push(vec![]); }

  fn stack_pop(&mut self) { self.analyze_stack.pop(); }

  // return the name of widget that `ident` point to if it's have.
  fn find_named_widget<'a>(&'a self, ident: &'a Ident) -> Option<&'a Ident> {
    self
      .analyze_stack
      .iter()
      .rev()
      .map(|local| local.iter().rev())
      .flatten()
      .find(|v| &v.name == ident)
      .and_then(|v| v.alias_of_name.as_ref())
      .or_else(|| self.named_widgets.contains(ident).then(|| ident))
  }

  fn expr_find_name_widget<'a>(&'a self, expr: &'a Expr) -> Option<&'a Ident> {
    if let Expr::Path(syn::ExprPath { path, .. }) = expr {
      path
        .get_ident()
        .and_then(|name| self.find_named_widget(name))
    } else {
      None
    }
  }

  fn add_depend(&mut self, name: Ident) {
    self
      .current_follows
      .entry(name.clone())
      .or_default()
      .push(name.span());

    let in_follow_scope = self.follow_scopes.last().cloned().unwrap_or(true);
    self.add_be_depended(name, true && in_follow_scope);
  }

  fn add_be_depended(&mut self, name: Ident, directly: bool) {
    let v = self.be_followed.entry(name).or_default();
    *v = *v || directly;
  }
}
