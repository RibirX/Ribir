use ahash::RandomState;
use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use std::collections::{BTreeMap, HashMap};
use syn::{
  parse::{Parse, ParseStream},
  spanned::Spanned,
  token, Expr, Ident, Token,
};

use super::{
  animations::Animations, dataflows::Dataflows, declare_widget::assign_uninit_field, kw,
  widget_def_variable, DeclareCtx, DeclareWidget, FollowInfo, FollowOn, FollowPlace, Follows,
  Result,
};
use crate::{
  error::{DeclareError, DeclareWarning},
  widget_attr_macro::{ribir_variable, BUILD_CTX},
};

pub struct WidgetMacro {
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
  fn parse(content: syn::parse::ParseStream) -> syn::Result<Self> {
    let mut widget: Option<DeclareWidget> = None;
    let mut dataflows: Option<Dataflows> = None;
    let mut animations: Option<Animations> = None;
    loop {
      if content.is_empty() {
        break;
      }
      let lk = content.lookahead1();
      if lk.peek(kw::declare) {
        let w = content.parse()?;
        assign_uninit_field!(widget, w, declare)?;
      } else if lk.peek(kw::dataflows) {
        let d = content.parse()?;
        assign_uninit_field!(dataflows, d, dataflows)?;
      } else if lk.peek(kw::animations) {
        let a = content.parse()?;
        assign_uninit_field!(animations, a, animations)?;
      } else {
        return Err(lk.error());
      }
    }

    let widget = widget.ok_or_else(|| {
      syn::Error::new(content.span(), "must have a `declare { ... }` in `widget!`")
    })?;

    Ok(Self { widget, dataflows, animations })
  }
}

impl WidgetMacro {
  pub fn gen_tokens(&mut self, ctx: &mut DeclareCtx) -> Result<TokenStream> {
    fn circle_stack_to_path(stack: &[CircleCheckStack], ctx: &DeclareCtx) -> Box<[FollowInfo]> {
      stack.iter().map(|c| c.to_follow_path(ctx)).collect()
    }

    ctx.id_collect(self)?;
    ctx.visit_widget_macro_mut(self);
    self.widget.before_generate_check(ctx)?;

    let mut named_objects_tokens = quote! {};
    if !ctx.named_objects.is_empty() {
      let mut follows = self.analyze_object_follows();
      let _init_circle_check = Self::circle_check(&follows, |stack| {
        Err(DeclareError::CircleInit(circle_stack_to_path(stack, ctx)))
      })?;

      let mut named_widgets_def = self.named_objects_def_tokens(ctx);

      Self::deep_follow_iter(&follows, |name| {
        named_objects_tokens.extend(named_widgets_def.remove(name));
      });

      named_widgets_def
        .into_values()
        .for_each(|def_tokens| named_objects_tokens.extend(def_tokens));

      self
        .widget
        .traverses_declare()
        .filter(|w| w.named.is_some())
        .for_each(|w| w.children_tokens(ctx, &mut named_objects_tokens));

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

    let declare_widget = if self.widget.named.is_none() {
      self.widget.widget_full_tokens(ctx)
    } else {
      self.widget.compose_tokens()
    };

    let Self { dataflows, animations, .. } = self;

    let ctx_name = ribir_variable(BUILD_CTX, Span::call_site());
    let def_name = widget_def_variable(&self.widget.widget_identify());

    Ok(quote! {
      (move |#ctx_name: &mut BuildCtx| {
        #named_objects_tokens
        #declare_widget
        #dataflows
        #animations
        #def_name.box_it()
      }).box_it()
    })
  }

  pub fn object_names_iter(&self) -> impl Iterator<Item = &Ident> {
    self
      .widget
      .object_names_iter()
      .chain(self.animations.iter().flat_map(|a| a.names()))
  }

  pub fn warnings(&self) -> impl Iterator<Item = DeclareWarning> + '_ { self.widget.warnings() }

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
    self
      .widget
      .named_objects_def_tokens_iter(ctx)
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
  fn to_follow_path(&self, ctx: &DeclareCtx) -> FollowInfo {
    let on = FollowOn {
      widget: ctx.user_perspective_name(&self.on.widget).map_or_else(
        || self.on.widget.clone(),
        |user| Ident::new(&user.to_string(), self.on.widget.span()),
      ),

      spans: self.on.spans.clone(),
    };

    let widget = ctx
      .user_perspective_name(self.widget)
      .unwrap_or(self.widget);

    let (widget, member) = match self.origin {
      FollowPlace::Field(f) => {
        // same id, but use the one which at the define place to provide more friendly
        // compile error.
        let widget = ctx
          .named_objects
          .get(widget)
          .expect("id must in named widgets")
          .clone();
        (widget, Some(f.member.clone()))
      }
      _ => (widget.clone(), None),
    };

    FollowInfo { widget, member, on }
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
