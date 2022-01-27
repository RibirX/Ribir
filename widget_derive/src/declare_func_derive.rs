use std::collections::{BTreeMap, HashMap};

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{quote, quote_spanned, ToTokens};
use syn::{
  parse_macro_input,
  punctuated::Punctuated,
  spanned::Spanned,
  token::{self, Brace, Comma},
  Expr, Ident, Path, Token,
};
pub mod sugar_fields;
use crate::error::{DeclareError, FollowInfo, Result};
use sugar_fields::*;
mod declare_visit_mut;
pub use declare_visit_mut::*;
mod follow_on;
mod parse;
use crate::declare_derive::field_convert_method;
pub use follow_on::*;

enum Child {
  Declare(Box<DeclareWidget>),
  Expr(Box<syn::Expr>),
}

pub struct DeclareMacro {
  pub widget: DeclareWidget,
  pub data_flows: Punctuated<DataFlow, Token![;]>,
}

pub struct DeclareWidget {
  path: Path,
  brace_token: Brace,
  // the name of this widget specified by `id` attr.
  named: Option<Id>,
  fields: Punctuated<DeclareField, Comma>,
  sugar_fields: SugarFields,
  rest: Option<RestExpr>,
  children: Vec<Child>,
}

pub struct SkipNcAttr {
  pound_token: token::Pound,
  bracket_token: token::Bracket,
  skip_nc_meta: kw::skip_nc,
}

pub struct DeclareField {
  skip_nc: Option<SkipNcAttr>,
  pub member: Ident,
  pub if_guard: Option<IfGuard>,
  pub colon_token: Option<Token![:]>,
  pub expr: Expr,
  pub follows: Option<FollowOnVec>,
}

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
  fn gen_tokens(&mut self, tokens: &mut TokenStream2, ctx: &DeclareCtx) -> Result<()> {
    let Self { from, to, .. } = self;
    let follows_on = from
      .follows
      .as_ref()
      .ok_or_else(|| DeclareError::DataFlowNoDepends(from.expr.span()))?;

    let upstream = upstream_observable(follows_on);

    let assign = skip_nc_assign(self.skip_nc.is_some(), &to.expr, &from.expr, ctx);
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
      let follows = self.analyze_widget_follows(ctx);
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
      self.widget.widget_full_tokens(ctx, &mut tokens);
    }

    self
      .data_flows
      .iter_mut()
      .try_for_each(|df| df.gen_tokens(&mut tokens, ctx))?;

    let def_name = self.widget.widget_def_name(ctx);
    Ok(quote! {{ #tokens #def_name.box_it() }})
  }

  /// return follow relationship of the named widgets,it is a key-value map,
  /// schema like
  /// ``` ascii
  /// {
  ///   widget_name: [field, {depended_widget: [position]}]
  /// }
  /// ```
  fn analyze_widget_follows(&self, ctx: &DeclareCtx) -> BTreeMap<Ident, WidgetFollows> {
    let mut follows: BTreeMap<Ident, WidgetFollows> = BTreeMap::new();
    self
      .widget
      .recursive_call(|w| {
        let ref_name = w.widget_ref_name(ctx);
        w.sugar_fields
          .wrap_widget_follows(&ref_name, ctx, &mut follows);

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
        let def_tokens = w.widget_def_tokens(ctx);
        named_defs.insert(name.clone(), def_tokens);
        let wrap_widgets = w.sugar_fields.gen_wrap_widgets_tokens(
          &w.widget_def_name(ctx),
          &w.widget_ref_name(ctx),
          ctx,
        );
        w.children_tokens(ctx, &mut compose_tokens);
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
struct RestExpr(Token![..], Expr);

impl ToTokens for RestExpr {
  fn to_tokens(&self, tokens: &mut TokenStream2) {
    self.0.to_tokens(tokens);
    self.1.to_tokens(tokens);
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

impl DeclareField {
  /// Generate field tokens with three part, the first is a tuple of field value
  /// and the follow condition, the second part is the field value declare in
  /// struct literal, the last part is expression to follow the other widgets
  /// change.
  ///
  /// The return value is the name of the follow condition;
  pub fn gen_tokens(
    &self,
    ref_name: &Ident,
    widget_ty: &Path,
    value_before: &mut TokenStream2,
    widget_def: &mut TokenStream2,
    follow_after: &mut TokenStream2,
    ctx: &DeclareCtx,
  ) -> Option<Ident> {
    let Self { if_guard, member, .. } = self;
    let expr_tokens = self.field_value_tokens(widget_ty);
    // we need to calculate field value before define widget to avoid twice
    // calculate it, only if filed  have `if guard`
    if let Some(if_guard) = if_guard {
      let follow_cond = Ident::new(&format!("{}_follow", member), Span::call_site());

      value_before.extend(quote! {
          let (#member, #follow_cond) = #if_guard {
            (#expr_tokens, true)
          } else {
            (<_>::default(), false)
          };
      });

      member.to_tokens(widget_def);

      if let Some(field_follow) = self.follow_tokens(ref_name, widget_ty, ctx) {
        follow_after.extend(quote! {
          if #follow_cond {
            #field_follow
          }
        });
      }
      Some(follow_cond)
    } else {
      member.to_tokens(widget_def);
      let colon = self.colon_token.unwrap_or_default();
      colon.to_tokens(widget_def);
      expr_tokens.to_tokens(widget_def);
      if let Some(follow) = self.follow_tokens(ref_name, widget_ty, ctx) {
        follow_after.extend(follow);
      }
      None
    }
  }

  pub fn follow_tokens(
    &self,
    ref_name: &Ident,
    widget_ty: &Path,
    ctx: &DeclareCtx,
  ) -> Option<TokenStream2> {
    let Self {
      member, follows: depends_on, skip_nc, ..
    } = self;

    let expr_tokens = self.field_value_tokens(widget_ty);

    depends_on.as_ref().map(|follows| {
      let assign = skip_nc_assign(
        skip_nc.is_some(),
        &quote! { #ref_name.#member},
        &expr_tokens,
        ctx,
      );
      let upstream = upstream_observable(follows);

      quote! {
          #upstream.subscribe( move |_|{ #assign } );
      }
    })
  }

  fn field_value_tokens(&self, widget_ty: &Path) -> TokenStream2 {
    let Self { member, expr, .. } = self;
    let field_converter = field_convert_method(member);
    quote_spanned! { expr.span() => <#widget_ty as Declare>::Builder::#field_converter(#expr) }
  }
}

impl DeclareWidget {
  fn widget_def_tokens(&self, ctx: &DeclareCtx) -> TokenStream2 {
    let Self { fields, rest, path, brace_token, .. } = self;

    let builder_ty = ctx.no_config_builder_type_name();
    let stateful = self.is_state_full(ctx).then(|| quote! { .into_stateful()});
    let def_name = self.widget_def_name(ctx);
    let ref_name = self.widget_ref_name(ctx);

    let mut value_before = quote! {};
    let mut build_widget = quote! {};
    let mut follow_after = quote! {};

    builder_ty.to_tokens(&mut build_widget);
    brace_token.surround(&mut build_widget, |content| {
      fields.pairs().for_each(|pair| {
        let (f, comma) = pair.into_tuple();
        f.gen_tokens(
          &ref_name,
          path,
          &mut value_before,
          content,
          &mut follow_after,
          ctx,
        );
        comma.to_tokens(content);
      });
      rest.to_tokens(content)
    });
    build_widget.extend(quote! {.build()#stateful});

    let state_ref = if self.is_stateful(ctx) {
      Some(quote! { let mut #ref_name = unsafe { #def_name.state_ref() }; })
    } else if ctx.be_reference(&ref_name) {
      Some(quote! { let #ref_name = &mut #def_name; })
    } else {
      None
    };

    let mut tokens = quote! {
      let mut #def_name = {
        type #builder_ty = <#path as Declare>::Builder;
        #value_before
        #build_widget
      };
      #state_ref
      #follow_after
    };

    self.normal_attrs_tokens(ctx, &mut tokens);
    self.listeners_tokens(ctx, &mut tokens);
    tokens
  }

  fn is_state_full(&self, ctx: &DeclareCtx) -> bool {
    // named widget is followed by others or its attributes.
    ctx.be_followed(&self.widget_ref_name(ctx))
      // unnamed widget is followed by its attributes.
      || (self.named.is_none() &&  self
      .fields
      .iter()
      .chain(self.sugar_fields.normal_attr_iter())
      .chain(self.sugar_fields.listeners_iter())
      .filter_map(|f| f.follows.as_ref().map(|d| (&f.member, d))).next()
        .is_some())
  }

  fn children_tokens(&self, ctx: &DeclareCtx, tokens: &mut TokenStream2) {
    if self.children.is_empty() {
      return;
    }

    let mut compose_tokens = quote! {};

    // Must be MultiChild if there are multi child. Give this hint for better
    // compile error if wrong size child declared.
    let hint = (self.children.len() > 1).then(|| quote! {: MultiChild<_>});
    let name = self.widget_def_name(ctx);

    self
      .children
      .iter()
      .enumerate()
      .for_each(|(idx, c)| match c {
        Child::Declare(d) => {
          let child_widget_name = d.widget_def_name(ctx);
          let c_name = if d.named.is_some() {
            child_widget_name
          } else {
            let c_name = ctx.no_conflict_child_name(idx);
            let mut child_tokens = quote! {};
            d.widget_full_tokens(ctx, &mut child_tokens);
            tokens.extend(quote! { let #c_name = { #child_tokens #child_widget_name }; });
            c_name
          };
          compose_tokens.extend(quote! { let #name #hint = (#name, #c_name).compose(); });
        }
        Child::Expr(expr) => {
          let c_name = ctx.no_conflict_child_name(idx);
          tokens.extend(quote! { let #c_name = #expr; });
          compose_tokens.extend(quote! { let #name #hint = (#name, #c_name).compose(); })
        }
      });
    tokens.extend(compose_tokens);
  }

  // return this widget tokens and its def name;
  fn widget_full_tokens(&self, ctx: &DeclareCtx, tokens: &mut TokenStream2) {
    tokens.extend(self.widget_def_tokens(ctx));

    let wrap_widgets = self.sugar_fields.gen_wrap_widgets_tokens(
      &self.widget_def_name(ctx),
      &self.widget_ref_name(ctx),
      ctx,
    );
    let (def_tokens, compose_tokens): (Vec<_>, Vec<_>) = wrap_widgets
      .into_iter()
      .map(|w| (w.def_and_ref_tokens, w.compose_tokens))
      .unzip();

    tokens.extend(def_tokens);
    self.children_tokens(ctx, tokens);
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

  fn widget_def_name(&self, ctx: &DeclareCtx) -> Ident {
    let ref_name = self.widget_ref_name(ctx);
    ctx.no_conflict_widget_def_name(&ref_name)
  }

  fn widget_ref_name(&self, ctx: &DeclareCtx) -> Ident {
    match &self.named {
      Some(Id { name, .. }) => name.clone(),
      _ => ctx.unnamed_widget_ref_name(),
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
  pub fn normal_attrs_tokens(&self, ctx: &DeclareCtx, tokens: &mut TokenStream2) {
    let w_name = self.widget_def_name(ctx);

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

          let self_ref = self.widget_ref_name(ctx);
          let value = ctx.new_no_conflict_name("v");
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
                let mut #self_ref = #self_ref.silent_ref();
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
              #w_name.into_attr_widget()
            };
          })
        } else {
          tokens.extend(attr_tokens)
        }
      },
    )
  }

  pub fn listeners_tokens(&self, ctx: &DeclareCtx, tokens: &mut TokenStream2) {
    let name = self.widget_def_name(ctx);

    self.sugar_fields.listeners_iter().for_each(
      |DeclareField { expr, member, if_guard, .. }| {
        if if_guard.is_some() {
          tokens.extend(quote! {
            let #name =  #if_guard {
              #name.#member(#expr)
            } else {
              #name.into_attr_widget()
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
        let w_ref = self.widget_ref_name(ctx);
        let wrap_ref = ctx.no_conflict_name_with_suffix(&w_ref, &f.member);
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

fn skip_nc_assign<L, R>(skip_nc: bool, left: &L, right: &R, ctx: &DeclareCtx) -> TokenStream2
where
  L: ToTokens,
  R: ToTokens,
{
  if skip_nc {
    let v = ctx.new_no_conflict_name("v");
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
