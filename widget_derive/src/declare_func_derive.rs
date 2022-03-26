use std::collections::{BTreeMap, HashMap};

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{quote, ToTokens};
use syn::{parse_macro_input, spanned::Spanned, Expr, Ident, Token};
pub mod sugar_fields;
use crate::{
  declare_func_derive::declare_widget::DeclareField,
  error::{DeclareError, FollowInfo, Result},
};
use sugar_fields::*;
mod declare_visit_mut;
pub use declare_visit_mut::*;
mod follow_on;
mod parse;

pub use follow_on::*;
mod variable_names;
use self::{animations::Animations, dataflows::Dataflows, declare_widget::DeclareWidget};
use ahash::RandomState;
pub use variable_names::*;
mod animations;
mod dataflows;
mod declare_widget;
mod widget_gen;
pub mod kw {
  syn::custom_keyword!(widget);
  syn::custom_keyword!(declare);
  syn::custom_keyword!(dataflows);
  syn::custom_keyword!(animations);
  syn::custom_keyword!(id);
  syn::custom_keyword!(skip_nc);
  syn::custom_keyword!(Animate);
  syn::custom_keyword!(State);
  syn::custom_keyword!(Transition);
}

pub enum Child {
  Declare(Box<DeclareWidget>),
  Expr(Box<syn::Expr>),
}

pub struct DeclareMacro {
  pub ctx_name: Ident,
  pub widget: DeclareWidget,
  pub dataflows: Option<Dataflows>,
  pub animations: Option<Animations>,
}

#[derive(Clone, Debug)]
pub struct IfGuard {
  pub if_token: Token![if],
  pub cond: Expr,
  pub fat_arrow_token: Token![=>],
}

#[derive(Clone, Debug)]
struct CircleCheckStack<'a> {
  pub widget: &'a Ident,
  pub origin: FollowPlace<'a>,
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
      FollowPlace::Field(f) => {
        // same id, but use the one which at the define place to provide more friendly
        // compile error.
        let widget = ctx
          .named_objects
          .get(&widget)
          .expect("id must in named widgets")
          .clone();
        (widget, Some(f.member.clone()))
      }
      _ => (widget.clone(), None),
    };

    FollowInfo { widget, member, on }
  }
}

fn is_widget_attr(origin: FollowPlace) -> bool {
  if let FollowPlace::Field(f) = origin {
    SugarFields::BUILTIN_LISTENERS
      .iter()
      .any(|name| f.member == name)
      || SugarFields::BUILTIN_DATA_ATTRS
        .iter()
        .any(|name| f.member == name)
  } else {
    false
  }
}

impl DeclareMacro {
  fn gen_tokens(&mut self, ctx: &mut DeclareCtx) -> Result<TokenStream2> {
    fn circle_stack_to_path(stack: &[CircleCheckStack], ctx: &DeclareCtx) -> Box<[FollowInfo]> {
      stack.iter().map(|c| c.into_follow_path(ctx)).collect()
    }

    ctx.id_collect(self)?;
    ctx.visit_declare_macro_mut(self);
    self.widget.before_generate_check(ctx)?;

    let mut tokens = quote! {};
    if !ctx.named_objects.is_empty() {
      let mut follows = self.analyze_object_follows();
      let _init_circle_check = Self::circle_check(&follows, |stack| {
        let head_is_attr = is_widget_attr(stack[0].origin);
        // fixme: we allow widget attr dependence widget self when init, but not support
        // indirect follow now.
        // `!is_widget_attr(stack.last().unwrap().on.widget.spans.all_widget_field)`
        // unit case `fix_attr_indirect_follow_host_fail.rs`, update its stderr if
        // fixed.
        let tail_on_widget = head_is_attr && false;
        if head_is_attr && stack.len() == 1 || tail_on_widget {
          Ok(())
        } else {
          Err(DeclareError::CircleInit(circle_stack_to_path(stack, ctx)))
        }
      })?;

      let mut named_widgets_def = self.named_objects_def_tokens(ctx);

      Self::deep_follow_iter(&follows, |name| {
        tokens.extend(named_widgets_def.remove(name));
      });

      named_widgets_def
        .into_values()
        .for_each(|def_tokens| tokens.extend(def_tokens));

      self
        .widget
        .traverses_declare()
        .filter(|w| w.named.is_some())
        .for_each(|w| w.children_tokens(ctx, &mut tokens));

      // data flow should not effect the named object init order, and we allow circle
      // follow with skip_nc attribute. So we add the data flow relationship and
      // individual check the circle follow error.
      if let Some(dataflows) = self.dataflows.as_ref() {
        dataflows.analyze_data_flow_follows(&mut follows);
        let _circle_follows_check = Self::circle_check(&follows, |stack| {
          if stack.iter().any(|s| match &s.origin {
            FollowPlace::Field(f) => f.skip_nc.is_some(),
            FollowPlace::DataFlow(df) => df.skip_nc.is_some(),
            _ => false,
          }) {
            Ok(())
          } else {
            Err(DeclareError::CircleFollow(circle_stack_to_path(stack, ctx)))
          }
        })?;
      }
    }

    if self.widget.named.is_none() {
      self.widget.widget_full_tokens(ctx, &mut tokens);
    } else {
      tokens.extend(self.widget.compose_tokens());
    }

    if let Some(dataflows) = self.dataflows.as_mut() {
      dataflows.to_tokens(&mut tokens);
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
  fn analyze_object_follows(&self) -> BTreeMap<Ident, Follows> {
    let mut follows = self.widget.analyze_object_follows();

    if let Some(animations) = self.animations.as_ref() {
      follows.extend(animations.follows_iter());
    }
    follows
  }

  // return the key-value map of the named widget define tokens.
  fn named_objects_def_tokens(
    &self,
    ctx: &DeclareCtx,
  ) -> HashMap<Ident, TokenStream2, RandomState> {
    let mut named_defs = HashMap::default();
    self.widget.named_objects_def_tokens(&mut named_defs, ctx);
    if let Some(ref a) = self.animations {
      a.named_objects_def_tokens(&mut named_defs);
    }
    named_defs
  }

  fn circle_check<F>(follow_infos: &BTreeMap<Ident, Follows>, err_detect: F) -> Result<()>
  where
    F: Fn(&[CircleCheckStack]) -> Result<()>,
  {
    #[derive(PartialEq, Debug)]
    enum CheckState {
      Checking,
      Checked,
    }

    let mut check_info: HashMap<_, _, RandomState> = HashMap::default();
    let mut stack = vec![];

    // return if the widget follow contain circle.
    fn widget_follow_circle_check<'a, F>(
      name: &'a Ident,
      follow_infos: &'a BTreeMap<Ident, Follows>,
      check_info: &mut HashMap<&'a Ident, CheckState, RandomState>,
      stack: &mut Vec<CircleCheckStack<'a>>,
      err_detect: &F,
    ) -> Result<()>
    where
      F: Fn(&[CircleCheckStack]) -> Result<()>,
    {
      match check_info.get(name) {
        None => {
          if let Some(follows) = follow_infos.get(name) {
            follows.follow_iter().try_for_each(|(origin, on)| {
              check_info.insert(name, CheckState::Checking);
              stack.push(CircleCheckStack { widget: name, origin, on });
              widget_follow_circle_check(&on.widget, follow_infos, check_info, stack, err_detect)?;
              stack.pop();
              Ok(())
            })?;
            debug_assert_eq!(check_info.get(name), Some(&CheckState::Checking));
            check_info.insert(name, CheckState::Checked);
          };
        }
        Some(CheckState::Checking) => {
          let start = stack.iter().position(|v| v.widget == name).unwrap();
          err_detect(&stack[start..])?;
        }
        Some(CheckState::Checked) => {}
      };
      Ok(())
    }

    follow_infos.keys().try_for_each(|name| {
      widget_follow_circle_check(name, follow_infos, &mut check_info, &mut stack, &err_detect)
    })
  }

  fn deep_follow_iter<F: FnMut(&Ident)>(follows: &BTreeMap<Ident, Follows>, mut callback: F) {
    // circular may exist widget attr follow widget self to init.
    let mut stacked = std::collections::HashSet::<_, RandomState>::default();

    let mut stack = follows.keys().rev().collect::<Vec<_>>();
    while let Some(w) = stack.pop() {
      match follows.get(w) {
        Some(f) if !stacked.contains(w) => {
          stack.push(w);
          stack.extend(f.follow_iter().map(|(_, target)| &target.widget));
        }
        _ => callback(w),
      }
      stacked.insert(w);
    }
  }
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

  let ctx_name = &declare.ctx_name;
  let build_ctx = build_ctx_name(declare.ctx_name.span());
  let tokens = quote! {{
    let #build_ctx = #ctx_name;
    #tokens
  }}
  .into();

  tokens
}
