//! mod parse the `widget!` macro.
use std::collections::HashSet;

use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned, ToTokens};
use syn::{
  braced,
  parse::{discouraged::Speculative, Parse, ParseStream},
  parse_quote,
  punctuated::Punctuated,
  spanned::Spanned,
  token::{Brace, Colon, Colon2, Comma, Dot, Paren},
  Block, Expr, Ident, Path, Result,
};

use super::ribir_variable;

pub mod kw {
  syn::custom_keyword!(widget);
  syn::custom_keyword!(states);
  syn::custom_keyword!(env);
  syn::custom_keyword!(DynWidget);
  syn::custom_keyword!(id);
  syn::custom_keyword!(skip_nc);
  syn::custom_keyword!(Animate);
  syn::custom_keyword!(State);
  syn::custom_keyword!(Transition);
  syn::custom_punctuation!(FlowArrow, ~>);
  syn::custom_keyword!(on);
  syn::custom_keyword!(change_on);
  syn::custom_keyword!(modify_on);
  syn::custom_keyword!(transition);
  syn::custom_keyword!(change);
}

mod animate_kw {
  syn::custom_keyword!(from);
  syn::custom_keyword!(transition);
  syn::custom_keyword!(animation);
  syn::custom_keyword!(lerp_fn);
}

pub struct MacroSyntax {
  pub env: Option<Env>,
  pub states: Option<States>,
  pub widget: DeclareWidget,
  pub items: Vec<Item>,
}

pub struct States {
  _states_token: kw::states,
  _brace: Brace,
  pub states: Vec<StateField>,
}

pub struct Env {
  _env_token: kw::env,
  pub stmts: Block,
}

#[derive(Debug)]
pub struct StateField {
  pub(crate) member: Ident,
  pub(crate) colon_token: Option<Colon>,
  pub(crate) expr: Expr,
}

#[derive(Debug)]
pub enum DeclareWidget {
  /// Declare widget as struct literal.
  Literal {
    ty: Path,
    brace: Brace,
    fields: Punctuated<DeclareField, Comma>,
    children: Vec<DeclareWidget>,
  },
  /// Declare a widget use a path.
  Path(Path),
  /// Declare a widget across widget construct call, only as a leaf declare.
  /// `X::new(...)`
  Call(ConstructCall),
}

#[derive(Debug)]
pub struct ConstructCall {
  path: Path,
  paren: Paren,
  args: Punctuated<Expr, Comma>,
}
#[derive(Debug)]
pub struct Id {
  pub id: kw::id,
  pub colon: Colon,
  pub name: Ident,
  pub tail_comma: Option<Comma>,
}

#[derive(Clone, Debug)]
pub struct DeclareField {
  pub member: Ident,
  pub colon: Option<Colon>,
  pub expr: Expr,
}
pub struct ChangeOnItem {
  pub change_on_token: kw::change_on,
  pub observe: Observe,
  pub quick_do: QuickDo,
}

pub struct ModifyOnItem {
  pub modify_on_token: kw::modify_on,
  pub observe: Observe,
  pub quick_do: QuickDo,
}

pub enum QuickDo {
  Flow(DataFlow),
  Animate(Animate),
  Transition(Transition),
}

pub struct DataFlow {
  pub flow_arrow: kw::FlowArrow,
  pub to: Expr,
}

pub struct OnEventDo {
  pub on_token: kw::on,
  pub observe: Observe,
  pub brace: Brace,
  pub handlers: Punctuated<DeclareField, Comma>,
}

#[derive(Clone, Debug)]
pub enum Observe {
  Name(Ident),
  Expr(Expr),
}
pub enum Item {
  Transition(Transition),
  Animate(Animate),
  OnEvent(OnEventDo),
  ModifyOn(ModifyOnItem),
  ChangeOn(ChangeOnItem),
}

#[derive(Debug)]
pub struct AnimateState {
  pub state: kw::State,
  pub brace: Brace,
  pub fields: Punctuated<AnimateStateField, Comma>,
}

#[derive(Debug)]
pub struct AnimateStateField {
  path: MemberPath,
  _colon: Option<Colon>,
  value: Expr,
}

#[derive(Debug)]
pub struct MemberPath {
  pub widget: Ident,
  pub dot: Dot,
  pub member: Ident,
}

#[derive(Debug)]
pub struct Transition {
  pub transition: kw::Transition,
  pub brace: Brace,
  pub fields: Punctuated<DeclareField, Comma>,
}

#[derive(Debug)]
pub struct Animate {
  pub animate_token: Ident,
  _brace_token: Brace,
  pub id: Option<Id>,
  pub from: Option<FromStateField>,
  pub transition: Option<TransitionField>,
  pub lerp_fn: Option<DeclareField>,
}

#[derive(Debug)]
pub struct FromStateField {
  pub from: Ident,
  pub colon: Colon,
  pub state: AnimateState,
}

#[derive(Debug)]
pub enum AnimateTransitionValue {
  Transition(Transition),
  Expr(Expr),
}
#[derive(Debug)]
pub struct TransitionField {
  pub transition_kw: Ident,
  pub colon: Option<Colon>,
  pub value: AnimateTransitionValue,
}

macro_rules! assign_uninit_field {
  ($self: ident.$name: ident, $field: ident) => {
    assign_uninit_field!($self.$name, $field, $name)
  };
  ($left: expr, $right: ident, $name: ident) => {
    if $left.is_none() {
      $left = Some($right);
      Ok(())
    } else {
      Err(syn::Error::new(
        $right.span(),
        format!("`{}` declare more than once", stringify!($name)).as_str(),
      ))
    }
  };
}

impl Parse for MacroSyntax {
  fn parse(input: ParseStream) -> Result<Self> {
    let mut widget: Option<DeclareWidget> = None;
    let mut items = vec![];
    let mut env: Option<Env> = None;
    let mut states: Option<States> = None;
    loop {
      if input.is_empty() {
        break;
      }
      let lk = input.lookahead1();
      if lk.peek(kw::modify_on) {
        items.push(Item::ModifyOn(input.parse()?));
      } else if lk.peek(kw::change_on) {
        items.push(Item::ChangeOn(input.parse()?));
      } else if lk.peek(kw::on) {
        items.push(Item::OnEvent(input.parse()?));
      } else if lk.peek(kw::Animate) {
        items.push(Item::Animate(input.parse()?));
      } else if lk.peek(kw::Transition) {
        items.push(Item::Transition(input.parse()?));
      } else if lk.peek(kw::states) {
        let mut t = input.parse::<States>()?;
        if let Some(ot) = states.take() {
          t.states.extend(ot.states);
        }
        states = Some(t);
      } else if lk.peek(kw::env) {
        let e: Env = input.parse::<Env>()?;
        if let Some(env) = env.as_mut() {
          env.stmts.stmts.extend(e.stmts.stmts);
        } else {
          env = Some(e)
        }
      } else if (lk.peek(Ident) || lk.peek(Colon2))
        && (input.peek2(Brace) || input.peek2(Colon2) || input.peek2(Paren))
      {
        let w: DeclareWidget = input.parse()?;
        if let Some(first) = widget.as_ref() {
          let err = syn::Error::new(
            w.span(),
            &format!(
              "Only one root widget can declare, but `{}` already declared.",
              first.ty_path().to_token_stream()
            ),
          );
          return Err(err);
        }
        widget = Some(w);
      } else {
        return Err(lk.error());
      }
    }
    let widget = widget
      .ok_or_else(|| syn::Error::new(input.span(), "must declare a root widget in `widget!`"))?;
    Ok(Self { widget, items, states, env })
  }
}

impl Parse for States {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    let content;

    let states = States {
      _states_token: input.parse()?,
      _brace: braced!(content in input),
      states: {
        let fields: Punctuated<StateField, Comma> = content.parse_terminated(StateField::parse)?;
        fields.into_iter().collect()
      },
    };
    Ok(states)
  }
}

impl Parse for Env {
  fn parse(input: ParseStream) -> Result<Self> {
    Ok(Self {
      _env_token: input.parse()?,
      stmts: input.parse()?,
    })
  }
}
impl Parse for StateField {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    let member = input.parse::<Ident>()?;
    let (colon_token, expr) = if input.peek(Colon) {
      (Some(input.parse()?), input.parse()?)
    } else {
      (None, parse_quote!(#member))
    };
    Ok(StateField { member, colon_token, expr })
  }
}

impl Parse for Observe {
  fn parse(input: ParseStream) -> Result<Self> {
    if input.peek(Ident) && input.peek2(Brace) {
      Ok(Observe::Name(input.parse()?))
    } else {
      let mut expr: Expr = input.parse()?;
      if let Expr::Paren(p) = expr {
        expr = *p.expr;
      }
      if let Expr::Path(p) = expr {
        if let Some(name) = p.path.get_ident() {
          Ok(Observe::Name(name.clone()))
        } else {
          Ok(Observe::Expr(Expr::Path(p)))
        }
      } else {
        Ok(Observe::Expr(expr))
      }
    }
  }
}
impl Parse for OnEventDo {
  fn parse(input: ParseStream) -> Result<Self> {
    let content;
    let on_token = input.parse()?;
    let observe: Observe = if input.peek(Ident) && input.peek2(Brace) {
      let target: Ident = input.parse()?;
      parse_quote!(#target)
    } else {
      input.parse()?
    };
    if input.is_empty() || !input.peek(Brace) {
      return Err(syn::Error::new(
        observe.span(),
        "No block found for `on` item, maybe ambiguity observe \
          expressions, try to parenthesis the expression you observe on.",
      ));
    }
    let on_event = Self {
      on_token,
      observe,
      brace: braced!(content in input),
      handlers: content.parse_terminated(DeclareField::parse)?,
    };

    check_duplicate_field(&on_event.handlers)?;
    Ok(on_event)
  }
}

impl Parse for ChangeOnItem {
  fn parse(input: ParseStream) -> Result<Self> {
    Ok(Self {
      change_on_token: input.parse()?,
      observe: input.parse()?,
      quick_do: input.parse()?,
    })
  }
}

impl Parse for ModifyOnItem {
  fn parse(input: ParseStream) -> Result<Self> {
    Ok(Self {
      modify_on_token: input.parse()?,
      observe: input.parse()?,
      quick_do: input.parse()?,
    })
  }
}

impl Parse for QuickDo {
  fn parse(input: ParseStream) -> Result<Self> {
    let lk = input.lookahead1();
    if lk.peek(kw::FlowArrow) {
      Ok(QuickDo::Flow(input.parse()?))
    } else if lk.peek(kw::Animate) {
      Ok(QuickDo::Animate(input.parse()?))
    } else if lk.peek(kw::Transition) {
      Ok(QuickDo::Transition(input.parse()?))
    } else {
      Err(lk.error())
    }
  }
}

impl Parse for DataFlow {
  fn parse(input: ParseStream) -> Result<Self> {
    Ok(Self {
      flow_arrow: input.parse()?,
      to: input.parse()?,
    })
  }
}

impl Parse for Id {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    Ok(Self {
      id: input.parse()?,
      colon: input.parse()?,
      name: input.parse()?,
      tail_comma: input.parse()?,
    })
  }
}

impl Parse for DeclareWidget {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    let path: Path = input.parse()?;

    // we not allow an ident as a widget, ambiguous with shorthand field init.
    if input.peek(Paren) {
      let content;
      Ok(DeclareWidget::Call(ConstructCall {
        path,
        paren: syn::parenthesized!(content in input),
        args: content.parse_terminated(Expr::parse)?,
      }))
    } else if path.get_ident().is_some() || input.peek(Brace) {
      let content;
      let brace = syn::braced!(content in input);
      let mut fields = Punctuated::default();
      let mut children = vec![];

      loop {
        if content.is_empty() {
          break;
        }

        let fork = content.fork();
        if let Ok(w) = fork.parse::<DeclareWidget>() {
          content.advance_to(&fork);
          children.push(w);
        } else {
          let f: DeclareField = content.parse()?;
          if !children.is_empty() {
            return Err(syn::Error::new(
              f.span(),
              "Field should always declare before children.",
            ));
          }
          fields.push(f);
          if !content.is_empty() {
            content.parse::<Comma>()?;
          }
        }
      }
      check_duplicate_field(&fields)?;

      Ok(DeclareWidget::Literal { ty: path, brace, fields, children })
    } else {
      Ok(DeclareWidget::Path(path))
    }
  }
}

impl Parse for DeclareField {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    let member: Ident = input.parse()?;
    let colon_token: Option<_> = input.parse()?;
    let expr = if colon_token.is_some() {
      input.parse()?
    } else {
      parse_quote!(#member)
    };

    Ok(DeclareField { member, colon: colon_token, expr })
  }
}

impl Parse for Transition {
  fn parse(input: ParseStream) -> Result<Self> {
    let content;
    let res = Self {
      transition: input.parse()?,
      brace: braced!( content in input),
      fields: content.parse_terminated(DeclareField::parse)?,
    };
    check_duplicate_field(&res.fields)?;
    Ok(res)
  }
}

impl Parse for Animate {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    let animate_token = input.parse::<Ident>()?;
    let content;
    let _brace_token = braced!(content in input);
    #[derive(Default)]
    struct Fields {
      id: Option<Id>,
      from: Option<FromStateField>,
      transition: Option<TransitionField>,
      lerp_fn: Option<DeclareField>,
    }

    let mut fields = Fields::default();

    loop {
      let lk = content.lookahead1();
      if lk.peek(kw::id) {
        let id = content.parse()?;
        assign_uninit_field!(fields.id, id)?;
        continue;
      } else if lk.peek(animate_kw::from) {
        let from = content.parse()?;
        assign_uninit_field!(fields.from, from)?;
      } else if lk.peek(animate_kw::transition) {
        let transition = content.parse()?;
        assign_uninit_field!(fields.transition, transition)?;
      } else if lk.peek(animate_kw::lerp_fn) {
        let lerp_fn = content.parse()?;
        assign_uninit_field!(fields.lerp_fn, lerp_fn)?;
      } else {
        return Err(lk.error());
      }
      if !content.is_empty() {
        content.parse::<Comma>()?;
      }
      if content.is_empty() {
        break;
      }
    }

    let Fields { id, from, transition, lerp_fn } = fields;

    Ok(Animate {
      animate_token,
      _brace_token,
      id,
      from,
      transition,
      lerp_fn,
    })
  }
}

impl Parse for AnimateState {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    let content;
    Ok(Self {
      state: input.parse()?,
      brace: braced!(content in input),
      fields: Punctuated::parse_terminated(&content)?,
    })
  }
}

impl Parse for FromStateField {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    Ok(FromStateField {
      from: input.parse()?,
      colon: input.parse()?,
      state: input.parse()?,
    })
  }
}

impl Parse for TransitionField {
  fn parse(input: ParseStream) -> Result<Self> {
    let transition_token: animate_kw::transition = input.parse()?;
    let transition_token = parse_quote! {#transition_token};
    let _colon_token: Option<Colon> = input.parse()?;
    let value = if _colon_token.is_some() {
      input.parse()?
    } else {
      parse_quote! {#transition_token}
    };
    Ok(TransitionField {
      transition_kw: transition_token,
      colon: _colon_token,
      value,
    })
  }
}

impl Parse for AnimateTransitionValue {
  fn parse(input: ParseStream) -> Result<Self> {
    let expr = if input.peek(kw::Transition) {
      AnimateTransitionValue::Transition(input.parse()?)
    } else {
      AnimateTransitionValue::Expr(input.parse()?)
    };
    Ok(expr)
  }
}

impl Parse for MemberPath {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    Ok(Self {
      widget: input.parse()?,
      dot: input.parse()?,
      member: input.parse()?,
    })
  }
}

impl Parse for AnimateStateField {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    let path = input.parse()?;
    let _colon_token: Option<_> = input.parse()?;
    let value = if _colon_token.is_some() {
      input.parse()?
    } else {
      parse_quote!(#path)
    };

    Ok(Self { path, _colon: _colon_token, value })
  }
}

impl ToTokens for MemberPath {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    let Self { widget, dot, member } = self;
    widget.to_tokens(tokens);
    dot.to_tokens(tokens);
    member.to_tokens(tokens);
  }
}

impl Spanned for DeclareWidget {
  fn span(&self) -> proc_macro2::Span {
    match self {
      DeclareWidget::Literal { ty, brace, .. } => ty.span().join(brace.span).unwrap(),
      DeclareWidget::Path(path) => path.span(),
      DeclareWidget::Call(ConstructCall { path: ty, paren, .. }) => {
        ty.span().join(paren.span).unwrap()
      }
    }
  }
}

impl DeclareWidget {
  pub(crate) fn ty_path(&self) -> &Path {
    match self {
      DeclareWidget::Literal { ty, .. } => ty,
      DeclareWidget::Call(call) => &call.path,
      DeclareWidget::Path(path) => path,
    }
  }
}

impl ToTokens for DeclareField {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    self.member.to_tokens(tokens);
    if self.colon.is_some() {
      self.colon.to_tokens(tokens);
      self.expr.to_tokens(tokens);
    }
  }
}

impl ToTokens for ConstructCall {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    let Self { path, paren, args } = self;
    path.to_tokens(tokens);
    paren.surround(tokens, |tokens| args.to_tokens(tokens));
  }
}

impl ToTokens for Id {
  fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
    self.id.to_tokens(tokens);
    self.colon.to_tokens(tokens);
    self.name.to_tokens(tokens);
  }
}

impl ToTokens for FromStateField {
  fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
    self.from.to_tokens(tokens);
    self.colon.to_tokens(tokens);
    self.state.to_tokens(tokens);
  }
}

impl ToTokens for AnimateState {
  fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
    let Self { state: state_token, brace, .. } = self;
    let state_span = state_token.span.join(brace.span).unwrap();

    let init_value = self.maybe_tuple_value(|AnimateStateField { value: expr, .. }| quote! {#expr});
    let target_value = self.maybe_tuple_value(|AnimateStateField { path, .. }| {
      quote! { #path.clone()}
    });

    let target_assign = self.maybe_tuple_value(|AnimateStateField { path, .. }| {
      let MemberPath { widget, dot, member } = path;
      quote! { #widget #dot shallow() #dot #member }
    });

    let v = ribir_variable("v", state_span);

    quote_spanned! { state_span =>
      AnimateState::new(
        Box::new(move ||  #init_value),
        Box::new(move || #target_value),
        Box::new(move |#v| #target_assign = #v)
      )
    }
    .to_tokens(tokens);
  }
}

impl ToTokens for Transition {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    self.transition.to_tokens(tokens);
    self.brace.surround(tokens, |tokens| {
      self.fields.to_tokens(tokens);
    });
  }
}

impl Spanned for TransitionField {
  fn span(&self) -> Span {
    match &self.value {
      AnimateTransitionValue::Transition(t) => t.span(),
      AnimateTransitionValue::Expr(expr) => expr.span(),
    }
  }
}

impl Spanned for Animate {
  #[inline]
  fn span(&self) -> proc_macro2::Span {
    self
      .animate_token
      .span()
      .join(self._brace_token.span)
      .unwrap()
  }
}

impl AnimateState {
  fn maybe_tuple_value(
    &self,
    value_by_field: impl Fn(&AnimateStateField) -> TokenStream,
  ) -> TokenStream {
    let value_tokens = self.fields.iter().map(|s| value_by_field(s));
    if self.fields.len() > 1 {
      quote! { (#(#value_tokens), *)}
    } else {
      quote! { #(#value_tokens), *}
    }
  }
}

pub fn check_duplicate_field(fields: &Punctuated<DeclareField, Comma>) -> syn::Result<()> {
  let mut sets = HashSet::<&Ident, ahash::RandomState>::default();
  for f in fields {
    if !sets.insert(&f.member) {
      return Err(syn::Error::new(
        f.member.span(),
        format!("`{}` declare more than once", f.member.to_string()).as_str(),
      ));
    }
  }
  Ok(())
}
