// todo: need remove
#![allow(dead_code)]

use quote::{quote, ToTokens};
use syn::{
  braced,
  parse::{Parse, ParseStream},
  punctuated::Punctuated,
  spanned::Spanned,
  token, Error, Expr, Ident, Result,
};

use super::{
  ribir_suffix_variable,
  sugar_fields::{assign_uninit_field, Id},
  SugarFields,
};

use super::kw;

pub struct Animations {
  animations_token: kw::animations,
  brace_token: token::Brace,
  animates_def: Vec<Animate>,
  states_def: Vec<State>,
  transitions_def: Vec<Transition>,
  triggers: Punctuated<Trigger, token::Comma>,
}

struct State {
  state_token: kw::State,
  brace_token: token::Brace,
  id: Option<Id>,
  fields: Punctuated<PathField, token::Comma>,
}

struct Transition {
  transition_token: kw::Transition,
  brace_token: token::Brace,
  id: Option<Id>,
  fields: Punctuated<SimpleField, token::Comma>,
}

struct Animate {
  animate_token: kw::Animate,
  brace_token: token::Brace,
  id: Option<Id>,
  from: StateField,
  transition: TransitionField,
}

mod animate_kw {
  syn::custom_keyword!(from);
  syn::custom_keyword!(transition);
}

enum StateExpr {
  State(State),
  Expr(syn::Expr),
}
struct StateField {
  from_token: animate_kw::from,
  colon_token: Option<token::Colon>,
  expr: Option<StateExpr>,
}

enum TransitionExpr {
  Transition(Transition),
  Expr(syn::Expr),
}
struct TransitionField {
  transition_token: animate_kw::transition,
  colon_token: Option<token::Colon>,
  expr: Option<TransitionExpr>,
}

struct Trigger {
  trigger: MemberPath,
  colon_token: token::Colon,
  expr: AnimateExpr,
}

enum AnimateExpr {
  /// a.on_click: Animate { ... }
  Animate(Animate),
  /// a.color: Transition { ... }
  Transition(Transition),
  /// a.color: if xxx { fade_in_animate } else { fly_in_animate }
  Expr(syn::Expr),
}
struct MemberPath {
  widget: Ident,
  dot_token: Option<token::Dot>,
  member: Ident,
}

struct PathField {
  path: MemberPath,
  colon_token: token::Colon,
  expr: Expr,
}

struct SimpleField {
  pub member: Ident,
  pub colon_token: Option<token::Colon>,
  pub expr: Option<Expr>,
}

struct SimpleStruct<KW, F> {
  name: KW,
  brace_token: token::Brace,
  id: Option<Id>,
  fields: Punctuated<F, token::Comma>,
}

impl<KW, F> Parse for SimpleStruct<KW, F>
where
  KW: Parse,
  F: Parse,
{
  fn parse(input: ParseStream) -> syn::Result<Self> {
    let content;
    let mut res = SimpleStruct {
      name: input.parse()?,
      brace_token: braced!( content in input),
      id: None,
      fields: Punctuated::new(),
    };

    loop {
      if content.is_empty() {
        break;
      }
      if content.peek(kw::id) {
        let id: Id = content.parse()?;
        let _: Option<Id> = assign_uninit_field!(res.id, id)?;
        if content.is_empty() {
          break;
        }
        content.parse::<token::Comma>()?;
      } else {
        let value = content.parse()?;
        res.fields.push_value(value);
        if content.is_empty() {
          break;
        }
        let punct = content.parse()?;
        res.fields.push_punct(punct);
      }
    }

    Ok(res)
  }
}

impl From<SimpleStruct<kw::State, PathField>> for State {
  fn from(s: SimpleStruct<kw::State, PathField>) -> Self {
    let SimpleStruct { id, name, brace_token, fields } = s;
    State {
      state_token: name,
      brace_token,
      id,
      fields,
    }
  }
}

impl From<SimpleStruct<kw::Transition, SimpleField>> for Transition {
  fn from(s: SimpleStruct<kw::Transition, SimpleField>) -> Self {
    let SimpleStruct { id, name, brace_token, fields } = s;
    Transition {
      transition_token: name,
      brace_token,
      id,
      fields,
    }
  }
}

impl Parse for Animations {
  fn parse(input: ParseStream) -> Result<Self> {
    let animations_token = input.parse()?;
    let content;
    let brace_token = braced!(content in input);

    let mut animates_def: Vec<Animate> = vec![];
    let mut states_def: Vec<State> = vec![];
    let mut transitions_def: Vec<Transition> = vec![];
    let mut triggers = Punctuated::new();

    loop {
      if content.is_empty() {
        break;
      }

      let lk = content.lookahead1();
      if lk.peek(kw::Animate) {
        let animate = content.parse::<Animate>()?;
        if animate.id.is_none() {
          return Err(Error::new(animate.animate_token.span(), "miss id"));
        }
        animates_def.push(animate);
      } else if lk.peek(kw::State) {
        let state = content.parse::<State>()?;
        if state.id.is_none() {
          return Err(Error::new(state.state_token.span(), "miss id"));
        }
        states_def.push(state);
      } else if lk.peek(kw::Transition) {
        let transition = content.parse::<Transition>()?;
        if transition.id.is_none() {
          return Err(Error::new(transition.transition_token.span(), "miss id"));
        }
        transitions_def.push(transition);
      } else {
        triggers.push(content.parse()?);
        if !content.is_empty() {
          triggers.push_punct(content.parse()?);
        }
      }
    }

    Ok(Animations {
      animations_token,
      brace_token,
      animates_def,
      states_def,
      transitions_def,
      triggers,
    })
  }
}

impl Parse for State {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    Ok(input.parse::<SimpleStruct<_, _>>()?.into())
  }
}

impl Parse for Transition {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    Ok(input.parse::<SimpleStruct<_, _>>()?.into())
  }
}

impl Parse for Animate {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    let animate_token: kw::Animate = input.parse()?;
    let content;
    let brace_token = braced!(content in input);
    #[derive(Default)]
    struct Fields {
      id: Option<Id>,
      from: Option<StateField>,
      transition: Option<TransitionField>,
    }

    let mut fields = Fields::default();

    loop {
      if content.is_empty() {
        break;
      }
      let lk = content.lookahead1();
      if lk.peek(kw::id) {
        let id = content.parse()?;
        let _: Option<Id> = assign_uninit_field!(fields.id, id)?;
      } else if lk.peek(animate_kw::from) {
        let from = content.parse()?;
        let _: Option<StateField> = assign_uninit_field!(fields.from, from)?;
      } else if lk.peek(animate_kw::transition) {
        let transition = content.parse()?;
        let _: Option<SimpleField> = assign_uninit_field!(fields.transition, transition)?;
      } else {
        Err(lk.error())?;
      }

      if !content.is_empty() {
        content.parse::<token::Comma>()?;
      }
    }

    let Fields { id, from, transition } = fields;
    let from = from.ok_or_else(|| Error::new(animate_token.span(), "miss `from` field."))?;
    let transition =
      transition.ok_or_else(|| Error::new(animate_token.span(), "miss `transition` field."))?;

    Ok(Animate {
      animate_token,
      brace_token,
      id,
      from,
      transition,
    })
  }
}

impl Parse for SimpleField {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    let member = input.parse()?;
    let (colon_token, expr) = if input.peek(token::Colon) {
      (Some(input.parse()?), Some(input.parse()?))
    } else {
      (None, None)
    };
    Ok(SimpleField { member, colon_token, expr })
  }
}

impl Parse for StateField {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    let from_token = input.parse()?;
    let colon_token: Option<token::Colon> = input.parse()?;
    let mut expr = None;
    if colon_token.is_some() {
      expr = Some(input.parse()?);
    }

    Ok(StateField { from_token, colon_token, expr })
  }
}

impl Parse for StateExpr {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    let expr = if input.peek(kw::State) {
      StateExpr::State(input.parse()?)
    } else {
      StateExpr::Expr(input.parse()?)
    };
    Ok(expr)
  }
}

impl Parse for TransitionField {
  fn parse(input: ParseStream) -> Result<Self> {
    let transition_token = input.parse()?;
    let colon_token: Option<_> = input.parse()?;
    let mut expr = None;
    if colon_token.is_some() {
      expr = Some(input.parse()?);
    }
    Ok(TransitionField { transition_token, colon_token, expr })
  }
}

impl Parse for TransitionExpr {
  fn parse(input: ParseStream) -> Result<Self> {
    let expr = if input.peek(kw::Transition) {
      TransitionExpr::Transition(input.parse()?)
    } else {
      TransitionExpr::Expr(input.parse()?)
    };
    Ok(expr)
  }
}

impl Parse for MemberPath {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    Ok(Self {
      widget: input.parse()?,
      dot_token: input.parse()?,
      member: input.parse()?,
    })
  }
}

impl Parse for PathField {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    Ok(Self {
      path: input.parse()?,
      colon_token: input.parse()?,
      expr: input.parse()?,
    })
  }
}

impl Parse for Trigger {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    Ok(Trigger {
      trigger: input.parse()?,
      colon_token: input.parse()?,
      expr: input.parse()?,
    })
  }
}

impl Parse for AnimateExpr {
  fn parse(input: ParseStream) -> Result<Self> {
    let lk = input.lookahead1();
    let expr = if lk.peek(kw::Animate) {
      AnimateExpr::Animate(input.parse()?)
    } else if lk.peek(kw::Transition) {
      AnimateExpr::Transition(input.parse()?)
    } else {
      AnimateExpr::Expr(input.parse()?)
    };
    Ok(expr)
  }
}

impl ToTokens for Animations {
  fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
    self.brace_token.surround(tokens, |tokens| {
      tokens.extend(quote! {todo!()});
      // self.triggers.to_tokens(tokens);
    });
  }
}

impl ToTokens for Animate {
  fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
    self.animate_token.to_tokens(tokens);
    self.brace_token.surround(tokens, |tokens| {
      tokens.extend(quote! {
        todo!()
      });
    })
  }
}

impl ToTokens for StateField {
  fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
    self.from_token.to_tokens(tokens);
    self.colon_token.to_tokens(tokens);
    if self.colon_token.is_some() {
      self.expr.as_ref().unwrap().to_tokens(tokens);
    }
  }
}

impl ToTokens for StateExpr {
  fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
    match self {
      StateExpr::State(s) => s.to_tokens(tokens),
      StateExpr::Expr(e) => e.to_tokens(tokens),
    }
  }
}

impl ToTokens for TransitionField {
  fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
    self.transition_token.to_tokens(tokens);
    self.colon_token.to_tokens(tokens);
    self.expr.to_tokens(tokens);
  }
}

impl ToTokens for TransitionExpr {
  fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
    match self {
      TransitionExpr::Transition(t) => t.to_tokens(tokens),
      TransitionExpr::Expr(e) => e.to_tokens(tokens),
    }
  }
}

impl ToTokens for State {
  fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
    self.state_token.to_tokens(tokens);
    self
      .brace_token
      .surround(tokens, |tokens| self.fields.to_tokens(tokens));
  }
}

impl ToTokens for Transition {
  fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
    self.transition_token.to_tokens(tokens);
    self.brace_token.surround(tokens, |tokens| {
      self.fields.to_tokens(tokens);
    })
  }
}

impl ToTokens for Trigger {
  fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
    fn state_trigger(widget: &Ident, member: &Ident, tokens: &mut proc_macro2::TokenStream) {
      tokens.extend(quote! {
        #widget
        .#member
        .state_change(|w| #widget.#member.clone())
        .subscribe(|change|{
          if change.before != change.after {
            todo!("trigger animate with before value");
          }
        })
      });
    }

    let Trigger {
      trigger: MemberPath { widget, member: trigger, .. },
      ..
    } = self;
    if let Some(suffix) = SugarFields::wrap_widget_from_field_name(trigger) {
      let mut w = widget.clone();
      w.set_span(w.span().join(suffix.span()).unwrap());
      let wrap_name = ribir_suffix_variable(&w, &suffix.to_string());
      state_trigger(&wrap_name, trigger, tokens);
    } else if SugarFields::BUILTIN_LISTENERS.iter().any(|v| trigger == v) {
      tokens.extend(quote! { #widget.#trigger(|_| {
        todo!("trigger animate");
      }); })
    }
    if SugarFields::BUILTIN_DATA_ATTRS.iter().any(|v| trigger == v) {
      let get_attr = Ident::new(&format!("get_{}", quote! {#trigger}), trigger.span());
      tokens.extend(quote! {
        #widget
          .#trigger
          .state_change(|w| w.#get_attr())
          .subscribe(|change| {
            if change.before != change.after {
              todo!("trigger animate");
            }
          });
      })
    } else {
      state_trigger(widget, trigger, tokens);
    }
  }
}

impl ToTokens for MemberPath {
  fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
    self.widget.to_tokens(tokens);
    self.dot_token.to_tokens(tokens);
    self.member.to_tokens(tokens);
  }
}

impl ToTokens for PathField {
  fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
    self.path.to_tokens(tokens);
    self.colon_token.to_tokens(tokens);
    self.expr.to_tokens(tokens);
  }
}

impl ToTokens for SimpleField {
  fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
    self.member.to_tokens(tokens);
    if let Some(colon) = self.colon_token {
      colon.to_tokens(tokens);
      self.expr.to_tokens(tokens);
    }
  }
}

impl Animations {
  pub fn object_names_iter(&self) -> impl Iterator<Item = &Ident> {
    let Self {
      animates_def,
      states_def,
      transitions_def,
      triggers,
      ..
    } = self;

    animates_def
      .iter()
      .map(|a| &a.id.as_ref().unwrap().name)
      .chain(states_def.iter().map(|s| &s.id.as_ref().unwrap().name))
      .chain(transitions_def.iter().map(|t| &t.id.as_ref().unwrap().name))
      .chain(triggers.iter().flat_map(|t| {
        let ids = match &t.expr {
          AnimateExpr::Animate(a) => {
            let animate_id = a.id.as_ref();
            let state_id = a.from.expr.as_ref().and_then(|e| match e {
              StateExpr::State(s) => s.id.as_ref(),
              StateExpr::Expr(_) => None,
            });
            let transition_id = a.transition.expr.as_ref().and_then(|t| match t {
              TransitionExpr::Transition(t) => t.id.as_ref(),
              TransitionExpr::Expr(_) => None,
            });
            [animate_id, state_id, transition_id]
          }
          AnimateExpr::Transition(Transition { id, .. }) => [id.as_ref(), None, None],
          _ => [None, None, None],
        };
        ids.into_iter().filter_map(|id| id.map(|id| &id.name))
      }))
  }
}
