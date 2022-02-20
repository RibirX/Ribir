use std::collections::{BTreeMap, HashMap};

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{quote, ToTokens};
use syn::{
  parse_macro_input,
  punctuated::Punctuated,
  spanned::Spanned,
  token::{self, Brace},
  Expr, Ident, Path, Token,
};
pub mod sugar_fields;
use crate::error::{DeclareError, FollowInfo, Result};
use sugar_fields::*;
mod declare_visit_mut;
pub use declare_visit_mut::*;
mod follow_on;
mod parse;

pub use follow_on::*;
mod variable_names;
pub use variable_names::*;

use self::widget_gen::WidgetGen;
mod widget_gen;
pub enum Child {
  Declare(Box<DeclareWidget>),
  Expr(Box<syn::Expr>),
}

pub struct DeclareMacro {
  pub ctx: Ident,
  pub widget: DeclareWidget,
  pub data_flows: Punctuated<DataFlow, Token![;]>,
}

pub struct DeclareWidget {
  path: Path,
  brace_token: Brace,
  // the name of this widget specified by `id` attr.
  named: Option<Id>,
  fields: Vec<DeclareField>,
  sugar_fields: SugarFields,
  children: Vec<Child>,
}

#[derive(Clone)]
pub struct SkipNcAttr {
  pound_token: token::Pound,
  bracket_token: token::Bracket,
  skip_nc_meta: kw::skip_nc,
}

#[derive(Clone)]
pub struct DeclareField {
  skip_nc: Option<SkipNcAttr>,
  pub member: Ident,
  pub if_guard: Option<IfGuard>,
  pub colon_token: Option<Token![:]>,
  pub expr: Expr,
  pub follows: Option<FollowOnVec>,
}

#[derive(Clone)]
pub struct IfGuard {
  pub if_token: Token![if],
  pub cond: Expr,
  pub fat_arrow_token: Token![=>],
}

mod ct {
  syn::custom_punctuation!(RightArrow, ~>);
}

pub struct DataFlowExpr {
  expr: Expr,
  follows: Option<FollowOnVec>,
}
pub struct DataFlow {
  skip_nc: Option<SkipNcAttr>,
  from: DataFlowExpr,
  _arrow_token: ct::RightArrow,
  to: DataFlowExpr,
}

impl ToTokens for SkipNcAttr {
  fn to_tokens(&self, tokens: &mut TokenStream2) {
    self.pound_token.to_tokens(tokens);
    self.bracket_token.surround(tokens, |tokens| {
      self.skip_nc_meta.to_tokens(tokens);
    })
  }
}

impl DataFlow {
  fn gen_tokens(&mut self, tokens: &mut TokenStream2) -> Result<()> {
    let Self { from, to, .. } = self;
    let follows_on = from
      .follows
      .as_ref()
      .ok_or_else(|| DeclareError::DataFlowNoDepends(from.expr.span()))?;

    let upstream = upstream_observable(follows_on);

    let assign = skip_nc_assign(self.skip_nc.is_some(), &to.expr, &from.expr);
    tokens.extend(quote! {
      #upstream.subscribe({
        #assign
        move |_| {
          #assign
        }
      });
    });
    Ok(())
  }
}

#[derive(Clone)]
struct CircleCheckStack<'a> {
  pub widget: &'a Ident,
  pub origin: FollowOrigin<'a>,
  pub on: &'a FollowOn,
}

impl DeclareMacro {
  fn gen_tokens(&mut self, ctx: &mut DeclareCtx) -> Result<TokenStream2> {
    fn circle_stack_to_path(stack: &[CircleCheckStack]) -> Box<[FollowInfo]> {
      stack
        .iter()
        .map(|o| FollowInfo {
          widget: o.widget.clone(),
          member: match o.origin {
            FollowOrigin::Field(f) => Some(f.member.clone()),
            FollowOrigin::DataFlow(_) => None,
          },
          on: o.on.clone(),
        })
        .collect()
    }

    ctx.id_collect(&self.widget)?;
    ctx.visit_declare_macro_mut(self);

    self.before_generate_check(ctx)?;
    let mut tokens = quote! {};
    if !ctx.named_widgets.is_empty() {
      let follows = self.analyze_widget_follows();
      let _init_circle_check = Self::circle_check(&follows, |stack| {
        Err(DeclareError::CircleInit(circle_stack_to_path(stack)))
      })?;

      // data flow should not effect the named widget order, and allow circle
      // follow with circle. So we clone the follow relationship and individual check
      // the circle follow error.
      if !self.data_flows.is_empty() {
        let mut follows = follows.clone();
        self.analyze_data_flow_follows(&mut follows);
        let _circle_follows_check = Self::circle_check(&follows, |stack| {
          if stack.iter().any(|s| -> bool {
            match &s.origin {
              FollowOrigin::Field(f) => f.skip_nc.is_some(),
              FollowOrigin::DataFlow(df) => df.skip_nc.is_some(),
            }
          }) {
            Ok(())
          } else {
            Err(DeclareError::CircleFollow(circle_stack_to_path(stack)))
          }
        })?;
      }

      let (mut named_widgets_def, compose) = self.named_widgets_def_tokens(ctx)?;

      Self::deep_follow_iter(&follows, |name| {
        tokens.extend(named_widgets_def.remove(name));
      });

      named_widgets_def
        .into_values()
        .for_each(|def_tokens| tokens.extend(def_tokens));
      tokens.extend(compose);
    }

    if self.widget.named.is_none() {
      self.widget.widget_full_tokens(ctx, &self.ctx, &mut tokens);
    }

    self
      .data_flows
      .iter_mut()
      .try_for_each(|df| df.gen_tokens(&mut tokens))?;

    let def_name = widget_def_variable(&self.widget.widget_identify());
    Ok(quote! {{ #tokens #def_name.box_it() }})
  }

  /// return follow relationship of the named widgets,it is a key-value map,
  /// schema like
  /// ``` ascii
  /// {
  ///   widget_name: [field, {depended_widget: [position]}]
  /// }
  /// ```
  fn analyze_widget_follows(&self) -> BTreeMap<Ident, WidgetFollows> {
    let mut follows: BTreeMap<Ident, WidgetFollows> = BTreeMap::new();
    self
      .widget
      .recursive_call(|w| {
        let ref_name = w.widget_identify();
        w.sugar_fields
          .collect_wrap_widget_follows(&ref_name, &mut follows);

        if w.named.is_some() {
          let w_follows: WidgetFollows = w
            .fields
            .iter()
            .filter_map(FieldFollows::clone_from)
            .chain(
              w.sugar_fields
                .normal_attr_iter()
                .chain(w.sugar_fields.listeners_iter())
                .filter_map(FieldFollows::clone_from)
                .filter_map(|mut f_follows| {
                  let follows = &mut f_follows.follows;
                  *follows = follows
                    .iter()
                    .filter(|f| f.widget != ref_name)
                    .cloned()
                    .collect();
                  (!follows.is_empty()).then(|| f_follows)
                }),
            )
            .map(WidgetFollowPart::Field)
            .collect();
          if !w_follows.is_empty() {
            follows.insert(ref_name, w_follows);
          }
        }
        Ok(())
      })
      .expect("should always success.");

    follows
  }

  fn analyze_data_flow_follows<'a>(&'a self, follows: &mut BTreeMap<Ident, WidgetFollows<'a>>) {
    self.data_flows.iter().for_each(|df| {
      if let Some(to) = df.to.follows.as_ref() {
        let df_follows = DataFlowFollows::clone_from(df);
        let part = WidgetFollowPart::DataFlow(df_follows);
        to.names().for_each(|name| {
          if let Some(w_follows) = follows.get_mut(name) {
            *w_follows = w_follows
              .iter()
              .cloned()
              .chain(Some(part.clone()).into_iter())
              .collect();
          } else {
            follows.insert(name.clone(), WidgetFollows::from_single_part(part.clone()));
          }
        })
      }
    });
  }

  // return the key-value map of the named widget define tokens.
  fn named_widgets_def_tokens(
    &self,
    ctx: &DeclareCtx,
  ) -> Result<(HashMap<Ident, TokenStream2>, TokenStream2)> {
    let mut named_defs = HashMap::new();

    let mut compose_tokens = quote! {};
    self.widget.recursive_call(|w| {
      if let Some(Id { name, .. }) = w.named.as_ref() {
        let def_tokens = w.widget_def_tokens(ctx, &self.ctx);
        named_defs.insert(name.clone(), def_tokens);
        let wrap_widgets =
          w.sugar_fields
            .gen_wrap_widgets_tokens(&w.widget_identify(), &self.ctx, ctx);
        w.children_tokens(ctx, &self.ctx, &mut compose_tokens);
        wrap_widgets.into_iter().for_each(|w| {
          named_defs.insert(w.name, w.def_and_ref_tokens);
          compose_tokens.extend(w.compose_tokens);
        });
      }
      Ok(())
    })?;
    Ok((named_defs, compose_tokens))
  }

  fn circle_check<F>(follow_infos: &BTreeMap<Ident, WidgetFollows>, err_detect: F) -> Result<()>
  where
    F: Fn(&Vec<CircleCheckStack>) -> Result<()>,
  {
    #[derive(PartialEq, Debug)]
    enum CheckState {
      Checking,
      Checked,
    }

    let mut check_info = HashMap::new();
    let mut stack = vec![];

    // return if the widget follow contain circle.
    fn widget_follow_circle_check<'a, F>(
      name: &'a Ident,
      follow_infos: &'a BTreeMap<Ident, WidgetFollows>,
      check_info: &mut HashMap<&'a Ident, CheckState>,
      stack: &mut Vec<CircleCheckStack<'a>>,
      err_detect: &F,
    ) -> Result<()>
    where
      F: Fn(&Vec<CircleCheckStack>) -> Result<()>,
    {
      match check_info.get(&name) {
        None => {
          if let Some(follows) = follow_infos.get(name) {
            check_info.insert(name, CheckState::Checking);
            follows.follow_iter().try_for_each(|(origin, on)| {
              stack.push(CircleCheckStack { widget: name, origin, on });
              widget_follow_circle_check(&on.widget, follow_infos, check_info, stack, err_detect)?;
              stack.pop();
              Ok(())
            })?;
            debug_assert_eq!(check_info.get(name), Some(&CheckState::Checking));
            check_info.insert(name, CheckState::Checked);
          };
          Ok(())
        }
        Some(CheckState::Checking) => err_detect(stack),
        Some(CheckState::Checked) => Ok(()),
      }
    }

    follow_infos.keys().try_for_each(|name| {
      widget_follow_circle_check(name, follow_infos, &mut check_info, &mut stack, &err_detect)
    })
  }

  fn deep_follow_iter<F: FnMut(&Ident)>(follows: &BTreeMap<Ident, WidgetFollows>, mut callback: F) {
    fn widget_deep_iter<F: FnMut(&Ident)>(
      name: &Ident,
      follows: &BTreeMap<Ident, WidgetFollows>,
      callback: &mut F,
    ) {
      if let Some(f) = follows.get(name) {
        f.follow_iter().for_each(|(_, target)| {
          widget_deep_iter(&target.widget, follows, callback);
          callback(&target.widget);
        });
      }
    }

    follows
      .keys()
      .for_each(|name| widget_deep_iter(name, follows, &mut callback));
  }

  fn before_generate_check(&self, ctx: &DeclareCtx) -> Result<()> {
    self.widget.recursive_call(|w| {
      if w.named.is_some() {
        w.unnecessary_skip_nc_check()?;
        w.wrap_widget_if_guard_check(ctx)?;
      }
      w.sugar_fields.key_follow_check()?;
      Ok(())
    })
  }
}

impl Spanned for DeclareWidget {
  fn span(&self) -> Span { self.path.span().join(self.brace_token.span).unwrap() }
}

impl Spanned for Child {
  fn span(&self) -> Span {
    match self {
      Child::Declare(d) => d.span(),
      Child::Expr(e) => e.span(),
    }
  }
}

impl ToTokens for IfGuard {
  fn to_tokens(&self, tokens: &mut TokenStream2) {
    self.if_token.to_tokens(tokens);
    self.cond.to_tokens(tokens);
  }
}

impl ToTokens for DeclareField {
  fn to_tokens(&self, tokens: &mut TokenStream2) {
    self.member.to_tokens(tokens);
    self.colon_token.to_tokens(tokens);
    let expr = &self.expr;
    if let Some(if_guard) = self.if_guard.as_ref() {
      tokens.extend(quote! {
        #if_guard {
          #expr
        } else {
          <_>::default()
        }
      })
    } else if self.colon_token.is_some() {
      expr.to_tokens(tokens)
    }
  }
}

impl DeclareWidget {
  fn widget_def_tokens<'a>(&'a self, ctx: &DeclareCtx, ctx_name: &'a Ident) -> TokenStream2 {
    let Self { path: ty, fields, .. } = self;
    let force_stateful = self
      .sugar_fields
      .normal_attr_iter()
      .any(|f| f.follows.is_some());

    let name = self.widget_identify();

    let mut tokens =
      WidgetGen { ty, name, fields, ctx_name }.gen_widget_tokens(ctx, force_stateful);

    self.normal_attrs_tokens(&mut tokens);
    self.listeners_tokens(&mut tokens);
    tokens
  }

  fn children_tokens(&self, ctx: &DeclareCtx, build_ctx_name: &Ident, tokens: &mut TokenStream2) {
    if self.children.is_empty() {
      return;
    }

    let mut compose_tokens = quote! {};

    // Must be MultiChild if there are multi child. Give this hint for better
    // compile error if wrong size child declared.
    let hint = (self.children.len() > 1).then(|| quote! {: MultiChild<_>});
    let name = widget_def_variable(&self.widget_identify());

    self
      .children
      .iter()
      .enumerate()
      .for_each(|(idx, c)| match c {
        Child::Declare(d) => {
          let child_widget_name = widget_def_variable(&d.widget_identify());
          let c_name = if d.named.is_some() {
            child_widget_name
          } else {
            let c_name = child_variable(c, idx);
            let mut child_tokens = quote! {};
            d.widget_full_tokens(ctx, build_ctx_name, &mut child_tokens);
            tokens.extend(quote! { let #c_name = { #child_tokens #child_widget_name }; });
            c_name
          };
          compose_tokens.extend(quote! { let #name #hint = (#name, #c_name).compose(); });
        }
        Child::Expr(expr) => {
          let c_name = child_variable(c, idx);
          tokens.extend(quote! { let #c_name = #expr; });
          compose_tokens.extend(quote! { let #name #hint = (#name, #c_name).compose(); })
        }
      });
    tokens.extend(compose_tokens);
  }

  // return this widget tokens and its def name;
  fn widget_full_tokens(
    &self,
    ctx: &DeclareCtx,
    build_ctx_name: &Ident,
    tokens: &mut TokenStream2,
  ) {
    let widget_tokens = self.widget_def_tokens(ctx, build_ctx_name);
    tokens.extend(widget_tokens);

    let wrap_widgets =
      self
        .sugar_fields
        .gen_wrap_widgets_tokens(&self.widget_identify(), build_ctx_name, ctx);
    let (def_tokens, compose_tokens): (Vec<_>, Vec<_>) = wrap_widgets
      .into_iter()
      .map(|w| (w.def_and_ref_tokens, w.compose_tokens))
      .unzip();

    tokens.extend(def_tokens);
    self.children_tokens(ctx, build_ctx_name, tokens);
    tokens.extend(compose_tokens);
  }

  fn recursive_call<'a, F>(&'a self, mut f: F) -> Result<()>
  where
    F: FnMut(&'a DeclareWidget) -> Result<()>,
  {
    fn inner<'a, F>(w: &'a DeclareWidget, f: &mut F) -> Result<()>
    where
      F: FnMut(&'a DeclareWidget) -> Result<()>,
    {
      f(w)?;
      w.children.iter().try_for_each(|w| match w {
        Child::Declare(w) => inner(w, f),
        Child::Expr(_) => Ok(()), // embed declare in express will extend tokens individual.
      })
    }
    inner(self, &mut f)
  }

  fn widget_identify(&self) -> Ident {
    match &self.named {
      Some(Id { name, .. }) => name.clone(),
      _ => ribir_variable("w", self.path.span()),
    }
  }
}

pub fn upstream_observable(depends_on: &FollowOnVec) -> TokenStream2 {
  let upstream = depends_on
    .names()
    .map(|depend_w| quote! { #depend_w.change_stream() });

  if depends_on.len() > 1 {
    quote! {  observable::from_iter([#(#upstream),*]).merge_all(usize::MAX) }
  } else {
    quote! { #(#upstream)* }
  }
}

impl DeclareWidget {
  pub fn normal_attrs_tokens(&self, tokens: &mut TokenStream2) {
    let w_name = widget_def_variable(&self.widget_identify());

    // todo: split fields by if it has `if-guard` and generate chain or not.
    self.sugar_fields.normal_attr_iter().for_each(
      |DeclareField {
         expr,
         member,
         follows,
         skip_nc,
         if_guard,
         ..
       }| {
        let method = Ident::new(&format!("with_{}", quote! {#member}), member.span());
        let depends_tokens = follows.as_ref().map(|follows| {
          let upstream = upstream_observable(follows);
          let set_attr = Ident::new(&format!("try_set_{}", quote! {#member}), member.span());
          let get_attr = Ident::new(&format!("get_{}", quote! {#member}), member.span());

          let self_ref = self.widget_identify();
          let value = ribir_variable("v", expr.span());
          let mut assign_value = quote! { let _ = #self_ref.#set_attr(#value); };
          if skip_nc.is_some() {
            assign_value = quote! {
              if #self_ref.#get_attr().as_ref() != Some(&#value) {
                #assign_value
              }
            };
          }

          quote! {
            #upstream.subscribe(
              move |_| {
                let #value = #expr;
                let mut #self_ref = #self_ref.silent();
                #assign_value
              }
            );
          }
        });
        let attr_tokens = quote! {
          #depends_tokens
          let #w_name = #w_name.#method(#expr);
        };
        if let Some(if_guard) = if_guard {
          tokens.extend(quote! {
            let #w_name = #if_guard {
              #attr_tokens
              #w_name
            }  else {
              // insert a empty attr for if-else type compatibility
              #w_name.insert_attr(())
            };
          })
        } else {
          tokens.extend(attr_tokens)
        }
      },
    )
  }

  pub fn listeners_tokens(&self, tokens: &mut TokenStream2) {
    let name = widget_def_variable(&self.widget_identify());

    self.sugar_fields.listeners_iter().for_each(
      |DeclareField { expr, member, if_guard, .. }| {
        if if_guard.is_some() {
          tokens.extend(quote! {
            let #name =  #if_guard {
              #name.#member(#expr)
            } else {
              // insert a empty attr for if-elsetype compatibility
              #name.insert_attr(())
            };
          });
        } else {
          tokens.extend(quote! { let #name = #name.#member(#expr); });
        }
      },
    );
  }

  /// Return a iterator of all syntax fields, include attributes and wrap
  /// widget.
  pub fn all_syntax_fields(&self) -> impl Iterator<Item = &DeclareField> {
    self
      .fields
      .iter()
      .chain(self.sugar_fields.normal_attr_iter())
      .chain(self.sugar_fields.listeners_iter())
      .chain(self.sugar_fields.widget_wrap_field_iter())
  }

  fn unnecessary_skip_nc_check(&self) -> Result<()> {
    debug_assert!(self.named.is_some());
    fn unnecessary_skip_nc(
      DeclareField { skip_nc, follows: depends_on, .. }: &DeclareField,
    ) -> Result<()> {
      match (depends_on, skip_nc) {
        (None, Some(attr)) => Err(DeclareError::UnnecessarySkipNc(attr.span())),
        _ => Ok(()),
      }
    }

    // normal widget
    self
      .fields
      .iter()
      .chain(self.sugar_fields.normal_attr_iter())
      .try_for_each(unnecessary_skip_nc)?;

    self
      .sugar_fields
      .widget_wrap_field_iter()
      .try_for_each(unnecessary_skip_nc)
  }

  fn wrap_widget_if_guard_check(&self, ctx: &DeclareCtx) -> Result<()> {
    debug_assert!(self.named.is_some());

    self
      .sugar_fields
      .widget_wrap_field_iter()
      .filter(|f| f.if_guard.is_some())
      .try_for_each(|f| {
        let w_ref = self.widget_identify();
        let wrap_ref = ribir_suffix_variable(&w_ref, &f.member.to_string());
        if ctx.be_followed(&wrap_ref) {
          let if_guard_span = f.if_guard.as_ref().unwrap().span();
          return Err(DeclareError::DependOnWrapWidgetWithIfGuard {
            wrap_def_pos: [w_ref.span(), wrap_ref.span(), if_guard_span],
            wrap_name: wrap_ref,
          });
        }
        Ok(())
      })
  }
}

fn skip_nc_assign<L, R>(skip_nc: bool, left: &L, right: &R) -> TokenStream2
where
  L: ToTokens,
  R: ToTokens,
{
  if skip_nc {
    let v = ribir_variable("v", left.span());
    quote! {
      let #v = #right;
      if #v != #left {
        #left = #v;
      }
    }
  } else {
    quote! { #left = #right; }
  }
}

pub(crate) fn declare_func_macro(input: TokenStream) -> TokenStream {
  let mut declare = parse_macro_input! { input as DeclareMacro };
  let mut ctx = DeclareCtx::default();

  let tokens = declare.gen_tokens(&mut ctx).unwrap_or_else(|err| {
    // forbid warning.
    ctx.forbid_warnings(true);
    err.into_compile_error(&ctx, &declare)
  });
  ctx.emit_unused_id_warning();

  tokens.into()
}
