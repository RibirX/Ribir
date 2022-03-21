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

pub mod kw {
  syn::custom_keyword!(id);
  syn::custom_keyword!(dataflows);
  syn::custom_keyword!(skip_nc);
  syn::custom_keyword!(animations);
  syn::custom_keyword!(Animate);
  syn::custom_keyword!(State);
  syn::custom_keyword!(Transition);
}

use self::{animations::Animations, widget_gen::WidgetGen};
mod animations;
mod widget_gen;
pub enum Child {
  Declare(Box<DeclareWidget>),
  Expr(Box<syn::Expr>),
}

pub struct DeclareMacro {
  pub ctx_name: Ident,
  pub widget: DeclareWidget,
  pub dataflows: Option<Punctuated<DataFlow, Token![;]>>,
  pub animations: Option<Animations>,
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
      .ok_or_else(|| DeclareError::DataFlowNoDepends(from.expr.span().unwrap()))?;

    let upstream = upstream_observable(follows_on);

    let assign = skip_nc_assign(self.skip_nc.is_some(), &to.expr, &from.expr);
    tokens.extend(quote! {
      #upstream.subscribe(move |_| { #assign });
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

impl<'a> CircleCheckStack<'a> {
  fn into_follow_path(&self, ctx: &DeclareCtx) -> FollowInfo {
    let on = FollowOn {
      widget: ctx.user_perspective_name(&self.on.widget).map_or_else(
        || self.on.widget.clone(),
        |user| Ident::new(&user.to_string(), self.on.widget.span()),
      ),

      spans: self.on.spans.clone(),
    };

    let widget = ctx
      .user_perspective_name(&self.widget)
      .unwrap_or_else(|| &self.widget);

    let (widget, member) = match self.origin {
      FollowOrigin::Field(f) => {
        // same id, but use the one which at the define place to provide more friendly
        // compile error.
        let widget = ctx
          .named_objects
          .get(&widget)
          .expect("id must in named widgets")
          .clone();
        (widget, Some(f.member.clone()))
      }
      FollowOrigin::DataFlow(_) => (widget.clone(), None),
    };

    FollowInfo { widget, member, on }
  }
}

impl DeclareMacro {
  fn gen_tokens(&mut self, ctx: &mut DeclareCtx) -> Result<TokenStream2> {
    fn circle_stack_to_path(stack: &[CircleCheckStack], ctx: &DeclareCtx) -> Box<[FollowInfo]> {
      stack.iter().map(|c| c.into_follow_path(ctx)).collect()
    }

    ctx.ctx_name = self.ctx_name.clone();
    ctx.id_collect(self)?;
    ctx.visit_declare_macro_mut(self);

    self.before_generate_check(ctx)?;
    let mut tokens = quote! {};
    if !ctx.named_objects.is_empty() {
      let follows = self.analyze_widget_follows();
      let _init_circle_check = Self::circle_check(&follows, |stack| {
        Err(DeclareError::CircleInit(circle_stack_to_path(stack, ctx)))
      })?;

      // data flow should not effect the named widget order, and allow circle
      // follow with circle. So we clone the follow relationship and individual check
      // the circle follow error.
      if let Some(dataflows) = self.dataflows.as_ref() {
        if !dataflows.is_empty() {
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
              Err(DeclareError::CircleFollow(circle_stack_to_path(stack, ctx)))
            }
          })?;
        }
      }

      let (mut named_widgets_def, compose) = self.named_objects_def_tokens(ctx)?;

      Self::deep_follow_iter(&follows, |name| {
        tokens.extend(named_widgets_def.remove(name));
      });

      named_widgets_def
        .into_values()
        .for_each(|def_tokens| tokens.extend(def_tokens));
      tokens.extend(compose);
    }

    if self.widget.named.is_none() {
      self.widget.widget_full_tokens(ctx, &mut tokens);
    } else {
      tokens.extend(self.widget.compose_tokens());
    }

    if let Some(dataflows) = self.dataflows.as_mut() {
      dataflows
        .iter_mut()
        .try_for_each(|df| df.gen_tokens(&mut tokens))?;
    }

    if let Some(ref animations) = self.animations {
      animations.to_tokens(&mut tokens);
    }

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
    let dataflows = if let Some(dataflows) = self.dataflows.as_ref() {
      dataflows
    } else {
      return;
    };
    dataflows.iter().for_each(|df| {
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
  fn named_objects_def_tokens(
    &self,
    ctx: &DeclareCtx,
  ) -> Result<(HashMap<Ident, TokenStream2>, TokenStream2)> {
    let mut named_defs = HashMap::new();

    let mut compose_tokens = quote! {};
    self.widget.recursive_call(|w| {
      if w.named.is_some() {
        let (name, def_tokens) = w.host_widget_tokens(ctx);
        named_defs.insert(name.clone(), def_tokens);

        w.sugar_fields
          .gen_wrap_widgets_tokens(&name, ctx, |name, wrap_tokens| {
            named_defs.insert(name, wrap_tokens);
          });
        w.children_tokens(ctx, &mut compose_tokens);
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
  fn host_widget_tokens(&self, ctx: &DeclareCtx) -> (Ident, TokenStream2) {
    let Self { path: ty, fields, .. } = self;
    let attrs_follow = self
      .sugar_fields
      .normal_attr_iter()
      .any(|f| f.follows.is_some());

    let name = self.widget_identify();
    let ctx_name = &ctx.ctx_name;
    let gen = WidgetGen { ty, name, fields, ctx_name };

    let mut tokens = gen.gen_widget_tokens(ctx, attrs_follow);
    self.normal_attrs_tokens(&mut tokens);
    self.listeners_tokens(&mut tokens);
    (gen.name.clone(), tokens)
  }

  fn children_tokens(&self, ctx: &DeclareCtx, tokens: &mut TokenStream2) {
    self
      .children
      .iter()
      .enumerate()
      .for_each(|(idx, c)| match c {
        Child::Declare(d) => {
          if d.named.is_none() {
            let child_widget_name = widget_def_variable(&d.widget_identify());
            let c_def_name = widget_def_variable(&child_variable(c, idx));
            let mut child_tokens = quote! {};
            d.widget_full_tokens(ctx, &mut child_tokens);
            tokens.extend(quote! { let #c_def_name = { #child_tokens  #child_widget_name }; });
          } else {
            tokens.extend(d.compose_tokens());
          }
        }
        Child::Expr(expr) => {
          let c_name = widget_def_variable(&child_variable(c, idx));
          tokens.extend(quote! { let #c_name = #expr; });
        }
      });
  }

  fn compose_tokens(&self) -> TokenStream2 {
    let mut compose_tokens = quote! {};
    let name = &self.widget_identify();
    let def_name = widget_def_variable(name);
    if !self.children.is_empty() {
      // Must be MultiChild if there are multi child. Give this hint for better
      // compile error if wrong size child declared.
      let hint = (self.children.len() > 1).then(|| quote! {: MultiChild<_>});

      let children = self.children.iter().enumerate().map(|(idx, c)| {
        let c_name = match c {
          Child::Declare(d) if d.named.is_some() => d.widget_identify(),
          _ => child_variable(c, idx),
        };
        let c_def_name = widget_def_variable(&c_name);
        quote! { .have_child(#c_def_name) }
      });
      compose_tokens.extend(quote! { let #def_name #hint = #def_name #(#children)*; });
    }
    compose_tokens.extend(self.sugar_fields.gen_wrap_widget_compose_tokens(&name));

    compose_tokens
  }

  // return this widget tokens and its def name;
  fn widget_full_tokens(&self, ctx: &DeclareCtx, tokens: &mut TokenStream2) {
    let (name, widget_tokens) = self.host_widget_tokens(ctx);
    tokens.extend(widget_tokens);

    self
      .sugar_fields
      .gen_wrap_widgets_tokens(&name, ctx, |_, wrap_widget| {
        tokens.extend(wrap_widget);
      });

    self.children_tokens(ctx, tokens);
    tokens.extend(self.compose_tokens());
  }

  pub(crate) fn recursive_call<'a, F>(&'a self, mut f: F) -> Result<()>
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
      _ => ribir_variable("ribir", self.path.span()),
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
          let mut assign_value = quote! { #self_ref.silent().#set_attr(#value); };
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

    let (guards, without_guards) = self
      .sugar_fields
      .listeners_iter()
      .partition::<Vec<_>, _>(|f| f.if_guard.is_some());
    guards
      .iter()
      .for_each(|DeclareField { expr, member, if_guard, .. }| {
        let if_guard = if_guard.as_ref().unwrap();
        tokens.extend(quote! {
          let #name =  #if_guard {
            #name.#member(#expr)
          } else {
            // insert a empty attr for if-else type compatibility
            #name.insert_attr(())
          };
        });
      });

    if !without_guards.is_empty() {
      let attrs = without_guards
        .iter()
        .map(|DeclareField { expr, member, .. }| {
          quote! {
            .#member(#expr)
          }
        });

      tokens.extend(quote! { let #name = #name #(#attrs)*; });
    }
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
        (None, Some(attr)) => Err(DeclareError::UnnecessarySkipNc(attr.span().unwrap())),
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
        let wrap_name = ribir_prefix_variable(&f.member, &w_ref.to_string());

        if ctx.be_followed(&wrap_name) {
          let if_guard_span = f.if_guard.as_ref().unwrap().span().unwrap();
          let mut use_spans = vec![];
          self.recursive_call(|w| {
            w.all_syntax_fields()
              .filter_map(|f| f.follows.as_ref())
              .flat_map(|follows| follows.iter())
              .filter(|f| f.widget == wrap_name)
              .for_each(|f| use_spans.extend(f.spans.iter().map(|s| s.unwrap())));
            Ok(())
          })?;

          let host_span = w_ref.span().unwrap();
          let wrap_span = wrap_name.span().unwrap();
          return Err(DeclareError::DependOnWrapWidgetWithIfGuard {
            wrap_def_spans: [host_span, wrap_span, if_guard_span],
            use_spans,
            wrap_name,
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
    err.into_compile_error()
  });
  ctx.emit_unused_id_warning();

  tokens.into()
}
