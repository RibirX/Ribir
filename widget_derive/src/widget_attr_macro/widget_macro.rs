use ahash::RandomState;
use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned, ToTokens};
use std::collections::{BTreeMap, HashMap};
use syn::{
  parse::{Parse, ParseStream},
  spanned::Spanned,
  token,
  visit_mut::VisitMut,
  Expr, Ident, Path, Token,
};

use super::{
  animations::Animations, child_variable, dataflows::Dataflows,
  declare_widget::assign_uninit_field, kw, track::Track, DeclareCtx, DeclareWidget, IdType,
  NameUsed, NameUsedInfo, Result, Scope, ScopeUsedInfo, UsedInfo,
};
use crate::{
  error::DeclareError,
  widget_attr_macro::{ribir_variable, BUILD_CTX},
};
pub const EXPR_WIDGET: &str = "ExprWidget";
pub const EXPR_FIELD: &str = "expr";

pub struct WidgetMacro {
  widget: DeclareWidget,
  track: Option<Track>,
  dataflows: Option<Dataflows>,
  animations: Option<Animations>,
}

#[derive(Clone, Debug)]
pub struct IfGuard {
  pub if_token: token::If,
  pub cond: Expr,
  pub fat_arrow_token: Token![=>],
  pub used_name_info: ScopeUsedInfo,
}

#[derive(Clone, Debug)]
struct CircleCheckStack<'a> {
  pub widget: &'a Ident,
  pub scope: Scope<'a>,
  pub used_widget: &'a Ident,
  pub used_info: &'a NameUsedInfo,
}

pub fn is_expr_keyword(ty: &Path) -> bool { ty.get_ident().map_or(false, |ty| ty == EXPR_WIDGET) }

impl Parse for WidgetMacro {
  fn parse(content: syn::parse::ParseStream) -> syn::Result<Self> {
    let mut widget: Option<DeclareWidget> = None;
    let mut dataflows: Option<Dataflows> = None;
    let mut animations: Option<Animations> = None;
    let mut track: Option<Track> = None;
    loop {
      if content.is_empty() {
        break;
      }
      let lk = content.lookahead1();
      if lk.peek(kw::dataflows) {
        let d = content.parse()?;
        assign_uninit_field!(dataflows, d, dataflows)?;
      } else if lk.peek(kw::animations) {
        let a = content.parse()?;
        assign_uninit_field!(animations, a, animations)?;
      } else if lk.peek(kw::track) {
        let t = content.parse()?;
        assign_uninit_field!(track, t, track)?;
      } else if lk.peek(Ident) && content.peek2(token::Brace) {
        let w = content.parse()?;
        assign_uninit_field!(widget, w, declare)?;
      } else {
        return Err(lk.error());
      }
    }

    let widget = widget.ok_or_else(|| {
      syn::Error::new(content.span(), "must have a `declare { ... }` in `widget!`")
    })?;

    Ok(Self { widget, dataflows, animations, track })
  }
}

impl WidgetMacro {
  pub fn gen_tokens(&mut self, ctx: &mut DeclareCtx) -> Result<TokenStream> {
    fn circle_stack_to_path(stack: &[CircleCheckStack], ctx: &DeclareCtx) -> Box<[UsedInfo]> {
      stack.iter().map(|c| c.to_follow_path(ctx)).collect()
    }

    ctx.id_collect(self)?;
    ctx.visit_widget_macro_mut(self);
    self.widget.before_generate_check(ctx)?;

    let mut follows = (!ctx.named_objects.is_empty()).then(|| self.analyze_object_dependencies());
    if let Some(follows) = follows.as_mut() {
      let _init_circle_check = Self::circle_check(&follows, |stack| {
        Err(DeclareError::CircleInit(circle_stack_to_path(stack, ctx)))
      })?;

      // data flow should not effect the named object init order, and we allow circle
      // follow with skip_nc attribute. So we add the data flow relationship and
      // individual check the circle follow error.
      if let Some(dataflows) = self.dataflows.as_ref() {
        dataflows.analyze_data_flow_follows(follows);
        let _circle_follows_check = Self::circle_check(&follows, |stack| {
          if stack.iter().any(|s| match &s.scope {
            Scope::Field(f) => f.skip_nc.is_some(),
            Scope::DataFlow(df) => df.skip_nc.is_some(),
            _ => false,
          }) {
            Ok(())
          } else {
            Err(DeclareError::CircleFollow(circle_stack_to_path(stack, ctx)))
          }
        })?;
      }
    }

    let ctx_name = ribir_variable(BUILD_CTX, Span::call_site());
    let mut tokens = quote! {};
    let closure_widget = |tokens: &mut TokenStream| {
      token::Paren::default().surround(tokens, |tokens| {
        tokens.extend(quote! { move |#ctx_name: &mut BuildCtx| });
        token::Brace::default().surround(tokens, |tokens| {
          if !ctx.named_objects.is_empty() {
            let mut named_widgets_def = self.named_objects_def_tokens(ctx);

            if let Some(follows) = follows.as_ref() {
              Self::deep_follow_iter(&follows, |name| {
                tokens.extend(named_widgets_def.remove(name));
              });
            }
            named_widgets_def
              .into_values()
              .for_each(|def_tokens| tokens.extend(def_tokens));
          }
          self.all_anonyms_widgets_tokens(ctx, tokens);
          self.dataflows.to_tokens(tokens);
          self.animations.to_tokens(tokens);
          self.compose_tokens(ctx, tokens);
          let name = self.widget_identify();
          tokens.extend(quote! {  #name.into_widget() })
        });
      });
      tokens.extend(quote! { .into_widget() });
    };

    if self.track.is_some() {
      token::Brace::default().surround(&mut tokens, |tokens| {
        self.track.to_tokens(tokens);
        closure_widget(tokens)
      });
    } else {
      closure_widget(&mut tokens);
    }

    ctx
      .unused_id_warning()
      .chain(self.widget.warnings())
      .for_each(|w| w.emit_warning());

    Ok(tokens)
  }

  pub fn object_names_iter(&self) -> impl Iterator<Item = (&Ident, IdType)> {
    self
      .widget
      .object_names_iter()
      .chain(self.animations.iter().flat_map(|a| a.names()))
      .map(|name| (name, IdType::DeclareDefine))
      .chain(
        self
          .track
          .iter()
          .flat_map(|t| t.track_names().map(|n| (n, IdType::UserSpecifyTrack))),
      )
  }

  /// return follow relationship of the named widgets,it is a key-value map,
  /// schema like
  /// ``` ascii
  /// {
  ///   widget_name: [field, {depended_widget: [position]}]
  /// }
  /// ```
  fn analyze_object_dependencies(&self) -> BTreeMap<Ident, NameUsed> {
    let mut follows = self.widget.analyze_object_follows();

    if let Some(animations) = self.animations.as_ref() {
      follows.extend(animations.follows_iter());
    }
    follows
  }

  // return the key-value map of the named widget define tokens.
  fn named_objects_def_tokens(&self, ctx: &DeclareCtx) -> HashMap<Ident, TokenStream, RandomState> {
    self
      .widget
      .traverses_widget()
      .filter_map(|w| {
        w.name()
          .map(|name| w.host_and_builtin_widgets_tokens(name, ctx))
      })
      .flatten()
      .chain(
        self
          .animations
          .as_ref()
          .map(|a| a.named_objects_def_tokens_iter())
          .into_iter()
          .flatten(),
      )
      .collect()
  }

  pub fn all_anonyms_widgets_tokens(&self, ctx: &DeclareCtx, tokens: &mut TokenStream) {
    fn anonyms_widgets_tokens(
      name: &Ident,
      w: &DeclareWidget,
      ctx: &DeclareCtx,
      tokens: &mut TokenStream,
    ) {
      if w.name().is_none() {
        w.host_and_builtin_widgets_tokens(&name, ctx)
          .for_each(|(_, widget)| tokens.extend(widget));
      }

      w.children
        .iter()
        .enumerate()
        .filter(|(_, c)| c.name().is_none())
        .for_each(|(idx, c)| {
          let c_name = child_variable(&name, idx);
          anonyms_widgets_tokens(&c_name, c, ctx, tokens);
        })
    }
    let name = self.widget_identify();
    anonyms_widgets_tokens(&name, &self.widget, ctx, tokens)
  }

  pub fn compose_tokens(&self, ctx: &DeclareCtx, tokens: &mut TokenStream) {
    fn compose_widget(w: &DeclareWidget, name: &Ident, ctx: &DeclareCtx, tokens: &mut TokenStream) {
      let mut compose_children = quote! {};
      w.children.iter().enumerate().for_each(|(idx, c)| {
        let c_name = c
          .name()
          .cloned()
          .unwrap_or_else(|| child_variable(name, idx));

        // deep compose first.
        compose_widget(c, &c_name, ctx, tokens);

        let compose_child = if c.builtin.is_empty() && c.is_expr_widget() {
          quote_spanned! { c.span() => .have_expr_child(#c_name)  }
        } else {
          quote_spanned! { c.span() => .have_child(#c_name) }
        };
        compose_children.extend(compose_child);
      });
      if !compose_children.is_empty() {
        let hint = (w.children.len() > 1).then(|| quote! {: MultiChildWidget<_>});
        tokens.extend(quote! { let #name #hint = #name #compose_children; });
      }

      w.builtin.compose_tokens(name, w.is_expr_widget(), tokens);
    }

    let name = self.widget_identify();
    compose_widget(&self.widget, &name, ctx, tokens);
  }

  pub fn widget_identify(&self) -> Ident {
    if let Some(name) = self.widget.name() {
      name.clone()
    } else {
      ribir_variable("ribir", self.widget.path.span())
    }
  }

  fn circle_check<F>(follow_infos: &BTreeMap<Ident, NameUsed>, err_detect: F) -> Result<()>
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
      follow_infos: &'a BTreeMap<Ident, NameUsed>,
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
            follows
              .follow_iter()
              .try_for_each(|(scope, used_widget, used_info)| {
                check_info.insert(name, CheckState::Checking);
                stack.push(CircleCheckStack {
                  widget: name,
                  scope,
                  used_widget,
                  used_info,
                });
                widget_follow_circle_check(
                  used_widget,
                  follow_infos,
                  check_info,
                  stack,
                  err_detect,
                )?;
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

  fn deep_follow_iter<F: FnMut(&Ident)>(follows: &BTreeMap<Ident, NameUsed>, mut callback: F) {
    // circular may exist widget attr follow widget self to init.
    let mut stacked = std::collections::HashSet::<_, RandomState>::default();

    let mut stack = follows.keys().rev().collect::<Vec<_>>();
    while let Some(w) = stack.pop() {
      match follows.get(w) {
        Some(f) if !stacked.contains(w) => {
          stack.push(w);
          stack.extend(f.follow_iter().map(|(_, name, _)| name));
        }
        _ => callback(w),
      }
      stacked.insert(w);
    }
  }
}

impl<'a> CircleCheckStack<'a> {
  fn to_follow_path(&self, ctx: &DeclareCtx) -> UsedInfo {
    let widget = ctx
      .user_perspective_name(self.widget)
      .unwrap_or(self.widget);

    let (widget, member) = match self.scope {
      Scope::Field(f) => {
        // same id, but use the one which at the define place to provide more friendly
        // compile error.
        let (widget, _) = ctx
          .named_objects
          .get_key_value(widget)
          .expect("id must in named widgets")
          .clone();
        (widget.clone(), Some(f.member.clone()))
      }
      _ => (widget.clone(), None),
    };

    let used_widget = ctx.user_perspective_name(&self.used_widget).map_or_else(
      || self.used_widget.clone(),
      |user| Ident::new(&user.to_string(), self.used_widget.span()),
    );
    UsedInfo {
      widget,
      member,
      used_widget,
      used_info: self.used_info.clone(),
    }
  }
}

impl DeclareCtx {
  pub fn visit_widget_macro_mut(&mut self, d: &mut WidgetMacro) {
    let mut ctx = self.stack_push();
    ctx.visit_declare_widget_mut(&mut d.widget);
    if let Some(dataflows) = d.dataflows.as_mut() {
      ctx.visit_dataflows_mut(dataflows)
    }
    if let Some(animations) = d.animations.as_mut() {
      ctx.visit_animations_mut(animations);
    }
  }

  pub fn visit_if_guard_mut(&mut self, if_guard: &mut IfGuard) {
    self.visit_expr_mut(&mut if_guard.cond);
    if_guard.used_name_info = self.clone_current_used_info();
  }
}

impl ToTokens for IfGuard {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    self.if_token.to_tokens(tokens);
    self.cond.to_tokens(tokens);
  }
}

impl Parse for IfGuard {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    Ok(IfGuard {
      if_token: input.parse()?,
      cond: input.parse()?,
      fat_arrow_token: input.parse()?,
      used_name_info: <_>::default(),
    })
  }
}
