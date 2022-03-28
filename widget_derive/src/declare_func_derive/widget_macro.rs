use ahash::RandomState;
use proc_macro2::{Delimiter, Group, Punct, Spacing, TokenStream, TokenTree};
use quote::{quote, ToTokens};
use std::collections::{BTreeMap, HashMap};
use syn::{
  braced,
  buffer::Cursor,
  parse::{discouraged::Speculative, Parse, ParseStream},
  spanned::Spanned,
  token, Expr, Ident, Token,
};

use super::{
  animations::Animations, dataflows::Dataflows, declare_widget::assign_uninit_field,
  declare_widget::SugarFields, kw, widget_def_variable, DeclareCtx, DeclareWidget, FollowInfo,
  FollowOn, FollowPlace, Follows, Result,
};
use crate::{declare_func_derive::DECLARE_WRAP_MACRO, error::DeclareError};

pub struct WidgetMacro {
  _widget_token: kw::widget,
  _bang_token: token::Bang,
  _brace_token: token::Brace,
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
    let widget_token = input.parse::<kw::widget>()?;
    let bang_token = input.parse()?;
    let content;
    let brace_token = braced!(content in input);

    let mut widget: Option<DeclareWidget> = None;
    let mut dataflows: Option<Dataflows> = None;
    let mut animations: Option<Animations> = None;
    loop {
      if content.is_empty() {
        break;
      }
      let lk = content.lookahead1();
      if lk.peek(kw::declare) {
        let macro_wrap = content.fork();
        let declare = macro_wrap.parse::<Ident>()?;
        let name = macro_wrap.parse::<Ident>()?;
        let declare_content;
        let braced = braced!(declare_content in macro_wrap);
        let wrapped_tokens =
          declare_content.step(|step_cursor| Ok(macro_wrap_declare_keyword(*step_cursor)))?;
        let w: DeclareWidget = if let Some(tts) = wrapped_tokens {
          let mut tokens = quote! { #declare #name };
          braced.surround(&mut tokens, |tokens| tokens.extend(tts));
          content.advance_to(&macro_wrap);
          syn::parse2(tokens)?
        } else {
          content.parse()?
        };
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
      syn::Error::new(
        widget_token.span(),
        "must have a `declare { ... }` in `widget!`",
      )
    })?;

    Ok(Self {
      _widget_token: widget_token,
      _bang_token: bang_token,
      _brace_token: brace_token,
      widget,
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
    ctx.visit_widget_macro_mut(self);
    self.widget.before_generate_check(ctx)?;

    let mut named_objects_tokens = quote! {};
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

    let ctx_name = ctx.ctx_name();

    let dataflows_tokens = self
      .dataflows
      .as_ref()
      .map(|dataflows| quote! { #dataflows});

    let animations_tokens = self
      .animations
      .as_ref()
      .map(|animations| animations.to_tokens(ctx_name));

    let def_name = widget_def_variable(&self.widget.widget_identify());
    Ok(quote! {{
      #named_objects_tokens
      #declare_widget
      #dataflows_tokens
      #animations_tokens
      #def_name.box_it()
    }})
  }

  pub fn object_names_iter(&self) -> impl Iterator<Item = &Ident> {
    self
      .widget
      .object_names_iter()
      .chain(self.animations.iter().flat_map(|a| a.names()))
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
    let ctx_name = ctx.ctx_name();
    self
      .widget
      .named_objects_def_tokens_iter(ctx)
      .chain(
        self
          .animations
          .as_ref()
          .map(|a| a.named_objects_def_tokens_iter(ctx_name))
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

// todo: only expr child need wrap, 
/// Wrap `declare Row {...}` with macro `ribir_declare_ಠ_ಠ!`, let our syntax
/// as a valid rust expression,  so we can use rust syntax to parse and
/// needn't reimplemented, and easy to interop with rust syntax.
///
/// return new tokens if do any wrap else
fn macro_wrap_declare_keyword(mut cursor: Cursor) -> (Option<Vec<TokenTree>>, Cursor) {
  fn sub_token_stream(mut begin: Cursor, end: Option<Cursor>, tts: &mut Vec<TokenTree>) {
    while Some(begin) != end {
      match begin.token_tree() {
        Some((tt, rest)) => {
          tts.push(tt);
          begin = rest;
        }
        None => break,
      }
    }
  }

  fn group_inner_wrap(cursor: Cursor, delim: Delimiter) -> Option<(Group, Cursor)> {
    cursor
      .group(delim)
      .and_then(|(group_cursor, span, cursor)| {
        macro_wrap_declare_keyword(group_cursor).0.map(|tts| {
          let mut group = Group::new(delim, tts.into_iter().collect());
          group.set_span(span);
          (group, cursor)
        })
      })
  }

  let mut tts = vec![];
  let mut stream_cursor = cursor.clone();
  loop {
    if let Some((n, c)) = cursor.ident() {
      if n == "widget" {
        let widget_macro = c
          .punct()
          .filter(|(p, _)| p.as_char() == '!')
          .and_then(|(_, c)| {
            c.group(Delimiter::Brace)
              .or_else(|| c.group(Delimiter::Parenthesis))
              .or_else(|| c.group(Delimiter::Bracket))
          });
        if let Some((_, _, c)) = widget_macro {
          // skip inner widget! macro, wrap wait itself parse.
          cursor = c;
          continue;
        }
      } else if n == "declare" {
        let declare_group =
          c.ident()
            .map(|(name, c)| (n, name, c))
            .and_then(|(declare, name, c)| {
              c.group(Delimiter::Brace)
                .map(|(body_cursor, span, cursor)| (declare, name, body_cursor, span, cursor))
            });
        if let Some((declare, name, body_cursor, body_span, c)) = declare_group {
          sub_token_stream(stream_cursor, Some(cursor), &mut tts);
          let body = macro_wrap_declare_keyword(body_cursor).0.map_or_else(
            || body_cursor.token_stream(),
            |tokens| tokens.into_iter().collect(),
          );

          tts.push(TokenTree::Ident(Ident::new(
            DECLARE_WRAP_MACRO,
            declare.span(),
          )));
          let mut bang = Punct::new('!', Spacing::Alone);
          bang.set_span(declare.span());
          tts.push(TokenTree::Punct(bang));

          let mut declare_group = Group::new(Delimiter::Brace, body);
          declare_group.set_span(body_span);
          let mut macro_group =
            Group::new(Delimiter::Brace, quote! { #declare #name #declare_group});
          macro_group.set_span(declare.span());
          tts.push(TokenTree::Group(macro_group));

          cursor = c;
          stream_cursor = c;
          continue;
        }
      }
      cursor = c;
    } else if let Some((group, c)) = group_inner_wrap(cursor, Delimiter::Brace)
      .or_else(|| group_inner_wrap(cursor, Delimiter::Bracket))
      .or_else(|| group_inner_wrap(cursor, Delimiter::Parenthesis))
    {
      sub_token_stream(stream_cursor, Some(cursor), &mut tts);
      tts.push(TokenTree::Group(group));
      cursor = c;
      stream_cursor = c;
    } else if let Some((_, c)) = cursor.token_tree() {
      cursor = c;
    } else {
      break;
    }
  }

  let tts = (!tts.is_empty()).then(|| {
    sub_token_stream(stream_cursor, None, &mut tts);
    tts.into_iter().collect()
  });
  (tts, cursor)
}
