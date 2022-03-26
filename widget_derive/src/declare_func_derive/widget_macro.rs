use std::collections::{BTreeMap, HashMap};

use ahash::RandomState;
use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{
  parse::{Parse, ParseStream},
  parse_quote,
  spanned::Spanned,
  token, Expr, Ident, Token,
};

use super::{
  animations::Animations, dataflows::Dataflows, declare_widget::assign_uninit_field,
  declare_widget::SugarFields, kw, widget_def_variable, DeclareCtx, DeclareWidget, FollowInfo,
  FollowOn, FollowPlace, Follows, Result,
};
use crate::error::DeclareError;

pub struct WidgetMacro {
  // todo: remove this
  pub ctx_name: Ident,
  // widget_token: kw::widget,
  // bang_token: token::Bang,
  // brace_token: token::Brace,
  widget: DeclareWidget,
  dataflows: Option<Dataflows>,
  animations: Option<Animations>,
}

#[derive(Clone, Debug)]
pub struct IfGuard {
  pub if_token: token::If,
  pub cond: Expr,
  pub fat_arrow_token: Token![=>],
}

#[derive(Clone, Debug)]
struct CircleCheckStack<'a> {
  pub widget: &'a Ident,
  pub origin: FollowPlace<'a>,
  pub on: &'a FollowOn,
}

impl Parse for WidgetMacro {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    // let widget_token = input.parse::<kw::widget>()?;
    // let bang_token = input.parse()?;
    // let content;
    // let brace_token = braced!(content in input);

    let content = input;
    let ctx = if !input.peek2(token::Brace) {
      let ctx = input.parse()?;
      input.parse::<token::Comma>()?;
      ctx
    } else {
      Ident::new(CTX_DEFAULT_NAME, Span::call_site())
    };

    let mut widget: Option<DeclareWidget> = None;
    let mut dataflows: Option<Dataflows> = None;
    let mut animations: Option<Animations> = None;
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
      } else {
        let w = content.parse()?;
        assign_uninit_field!(widget, w, declare)?;
      }
    }
    // let declare = declare.ok_or_else(|| {
    //   syn::Error::new(
    //     widget_token.span(),
    //     "must have a `declare { ... }` in `widget!`",
    //   )
    // })?;

    Ok(Self {
      ctx_name: ctx,
      // widget_token,
      // bang_token,
      // brace_token,
      widget: widget.unwrap(),
      dataflows,
      animations,
    })
  }
}

impl WidgetMacro {
  pub fn gen_tokens(&mut self, ctx: &mut DeclareCtx) -> Result<TokenStream> {
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

  pub fn object_names_iter(&self) -> impl Iterator<Item = &Ident> {
    self
      .widget
      .object_names_iter()
      .chain(self.animations.iter().flat_map(|a| a.object_names_iter()))
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
  fn named_objects_def_tokens(&self, ctx: &DeclareCtx) -> HashMap<Ident, TokenStream, RandomState> {
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

const CTX_DEFAULT_NAME: &str = "ctx";

impl DeclareCtx {
  pub fn visit_declare_macro_mut(&mut self, d: &mut WidgetMacro) {
    self.visit_declare_widget_mut(&mut d.widget);
    if let Some(dataflows) = d.dataflows.as_mut() {
      self.visit_dataflows_mut(dataflows)
    }
    if let Some(animations) = d.animations.as_mut() {
      self.visit_animations_mut(animations);
    }
  }

  pub fn extend_declare_macro_to_expr(&mut self, tokens: proc_macro::TokenStream) -> Expr {
    let mut declare: WidgetMacro = syn::parse(tokens).expect("extend declare macro failed!");
    let named = self.named_objects.clone();

    let tokens = {
      let mut ctx = self.borrow_capture_scope(true);

      declare.gen_tokens(&mut *ctx).unwrap_or_else(|err| {
        // forbid warning.
        ctx.forbid_warnings(true);
        err.into_compile_error()
      })
    };

    // trigger warning and restore named widget.
    named.iter().for_each(|k| {
      self.named_objects.remove(k);
    });
    self.emit_unused_id_warning();
    self.named_objects = named;

    parse_quote!(#tokens)
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
    })
  }
}
