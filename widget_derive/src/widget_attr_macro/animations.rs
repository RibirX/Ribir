use proc_macro2::TokenStream;
use quote::{quote, quote_spanned, ToTokens};
use syn::{
  braced,
  parse::{Parse, ParseStream},
  parse_quote,
  punctuated::Punctuated,
  spanned::Spanned,
  token,
  visit_mut::VisitMut,
  Error, Expr, Ident, Result,
};

use crate::widget_attr_macro::Id;

use super::{
  capture_widget,
  declare_widget::{assign_uninit_field, BuiltinFieldWidgets},
  ribir_suffix_variable, ribir_variable, widget_def_variable,
  widget_macro::UsedNameInfo,
  widget_state_ref, DeclareCtx, DependIn, Depends, BUILD_CTX,
};

use super::kw;

pub struct Animations {
  _animations_token: kw::animations,
  brace_token: token::Brace,
  animates_def: Vec<Animate>,
  states_def: Vec<State>,
  transitions_def: Vec<Transition>,
  triggers: Punctuated<Trigger, token::Comma>,
}

#[derive(Debug)]
pub struct State {
  state_token: kw::State,
  _brace_token: token::Brace,
  id: Option<Id>,
  fields: Punctuated<PathField, token::Comma>,
  used_name_info: UsedNameInfo,
}

#[derive(Debug)]
pub struct Transition {
  transition_token: kw::Transition,
  _brace_token: token::Brace,
  id: Option<Id>,
  fields: Punctuated<SimpleField, token::Comma>,
  used_name_info: UsedNameInfo,
}

#[derive(Debug)]
pub struct Animate {
  animate_token: kw::Animate,
  brace_token: token::Brace,
  id: Option<Id>,
  from: FromStateField,
  transition: TransitionField,
  used_name_info: UsedNameInfo,
}

mod animate_kw {
  syn::custom_keyword!(from);
  syn::custom_keyword!(transition);
}

#[derive(Debug)]
enum StateExpr {
  State(State),
  Expr(syn::Expr),
}
#[derive(Debug)]
struct FromStateField {
  expr: StateExpr,
}
#[derive(Debug)]
enum TransitionExpr {
  Transition(Transition),
  Expr(syn::Expr),
}
#[derive(Debug)]
struct TransitionField {
  expr: TransitionExpr,
}

struct Trigger {
  path: MemberPath,
  _colon_token: token::Colon,
  expr: AnimateExpr,
}

enum AnimateExpr {
  /// a.on_click: Animate { ... }
  Animate(Box<Animate>),
  /// a.color: Transition { ... }
  Transition(Transition),
  /// a.color: if xxx { fade_in_animate } else { fly_in_animate }
  Expr(syn::Expr),
}
#[derive(Debug)]
struct MemberPath {
  widget: Ident,
  dot_token: token::Dot,
  member: Ident,
}

#[derive(Debug)]
struct PathField {
  path: MemberPath,
  _colon_token: token::Colon,
  expr: Expr,
}

#[derive(Debug)]
pub struct SimpleField {
  pub(crate) member: Ident,
  pub(crate) colon_token: Option<token::Colon>,
  pub(crate) expr: Expr,
}

struct SimpleStruct<KW, F> {
  name: KW,
  brace_token: token::Brace,
  id: Option<Id>,
  fields: Punctuated<F, token::Comma>,
}

fn widget_from_field_name(widget: &Ident, field: &Ident) -> Ident {
  if let Some(suffix) = BuiltinFieldWidgets::as_builtin_widget(field) {
    let mut w = widget.clone();
    w.set_span(w.span().join(suffix.span()).unwrap());
    ribir_suffix_variable(&w, &suffix.to_string())
  } else {
    widget.clone()
  }
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
        assign_uninit_field!(res.id, id)?;
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
      _brace_token: brace_token,
      id,
      fields,
      used_name_info: <_>::default(),
    }
  }
}

impl From<SimpleStruct<kw::Transition, SimpleField>> for Transition {
  fn from(s: SimpleStruct<kw::Transition, SimpleField>) -> Self {
    let SimpleStruct { id, name, brace_token, fields } = s;
    Transition {
      transition_token: name,
      _brace_token: brace_token,
      id,
      fields,
      used_name_info: <_>::default(),
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
      _animations_token: animations_token,
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
      from: Option<FromStateField>,
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
        assign_uninit_field!(fields.id, id)?;
      } else if lk.peek(animate_kw::from) {
        let from = content.parse()?;
        assign_uninit_field!(fields.from, from)?;
      } else if lk.peek(animate_kw::transition) {
        let transition = content.parse()?;
        assign_uninit_field!(fields.transition, transition)?;
      } else {
        return Err(lk.error());
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
      used_name_info: <_>::default(),
    })
  }
}

impl Parse for SimpleField {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    let member = input.parse::<Ident>()?;
    let (colon_token, expr) = if input.peek(token::Colon) {
      (Some(input.parse()?), input.parse()?)
    } else {
      (None, parse_quote!(#member))
    };
    Ok(SimpleField { member, colon_token, expr })
  }
}

impl Parse for FromStateField {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    let from_token: animate_kw::from = input.parse()?;
    let colon_token: Option<token::Colon> = input.parse()?;
    let expr = if colon_token.is_some() {
      input.parse()?
    } else {
      parse_quote!(#from_token)
    };

    Ok(FromStateField { expr })
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
    let transition_token: animate_kw::transition = input.parse()?;
    let colon_token: Option<token::Colon> = input.parse()?;
    let expr = if colon_token.is_some() {
      input.parse()?
    } else {
      parse_quote! {#transition_token}
    };
    Ok(TransitionField { expr })
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
      _colon_token: input.parse()?,
      expr: input.parse()?,
    })
  }
}

impl Parse for Trigger {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    Ok(Trigger {
      path: input.parse()?,
      _colon_token: input.parse()?,
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
  fn to_tokens(&self, tokens: &mut TokenStream) {
    self.brace_token.surround(tokens, |tokens| {
      self.triggers.iter().for_each(|t| t.to_tokens(tokens));
    });
  }
}

impl ToTokens for Animate {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    let Self {
      animate_token,
      id,
      from,
      transition,
      used_name_info,
      ..
    } = self;

    let animate_span = animate_token.span();
    let ctx_name = ribir_variable(BUILD_CTX, animate_span);

    let mut animate_def_tokens = quote_spanned! { animate_span =>
      #animate_token::new(#from, &#transition, #ctx_name)
    };

    if used_name_info.captures.is_some() {
      let captures = used_name_info.capture_widgets().map(capture_widget);
      animate_def_tokens = quote_spanned!(animate_span => {
        #(#captures)*
        #animate_def_tokens
      });
    }

    if let Some(Id { name, .. }) = id.as_ref() {
      animate_def_tokens = quote_spanned! {animate_span =>
        #[allow(unused_mut)]
        let mut #name = #animate_def_tokens;
      };
    }

    tokens.extend(animate_def_tokens);
  }
}

impl Animate {
  fn embed_as_expr_tokens(&self, tokens: &mut TokenStream) {
    if let Some(Id { name, .. }) = self.id.as_ref() {
      name.to_tokens(tokens)
    } else {
      self.to_tokens(tokens)
    }
  }
}

impl ToTokens for FromStateField {
  fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) { self.expr.to_tokens(tokens); }
}

impl ToTokens for StateExpr {
  fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
    match self {
      StateExpr::State(s) => {
        if let Some(Id { name, .. }) = s.id.as_ref() {
          name.to_tokens(tokens);
        } else {
          s.to_tokens(tokens);
        }
      }
      StateExpr::Expr(e) => e.to_tokens(tokens),
    }
  }
}

impl ToTokens for TransitionField {
  fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
    // let Self { transition_token, colon_token, expr } = self;
    self.expr.to_tokens(tokens);
  }
}

impl ToTokens for TransitionExpr {
  fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
    match self {
      TransitionExpr::Transition(t) => {
        // named object is already define before
        if let Some(Id { name, .. }) = t.id.as_ref() {
          name.to_tokens(tokens);
        } else {
          t.to_tokens(tokens);
        }
      }
      TransitionExpr::Expr(e) => e.to_tokens(tokens),
    }
  }
}

impl ToTokens for State {
  fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
    let Self {
      state_token,
      id,
      fields,
      used_name_info,
      ..
    } = self;

    let state_span = state_token.span();

    let refs = self.used_name_info.used_widgets().map(widget_state_ref);

    let mut state_tokens = if fields.len() > 1 {
      let init_value = fields.iter().map(|f| &f.expr);
      // let path_members = fields.iter().map(|f| &f.path);
      let widgets = fields
        .iter()
        .map(|f| widget_from_field_name(&f.path.widget, &f.path.member));
      let widgets2 = widgets.clone();
      let members = fields.iter().map(|f| &f.path.member);
      let members2 = members.clone();
      let indexes = (0..fields.len()).map(syn::Index::from);

      quote! {
        #(#refs)*;
        let state_init = (#(#init_value),*);
        let state_final = (#(#widgets2.#members2.clone()),*);
        move |p: f32| {
          #(#widgets.shallow().#members
            = Tween::tween(&state_init.#indexes, &state_final.#indexes, p);)*
        }
      }
    } else {
      let PathField { path, _colon_token, expr } = &fields[0];
      let MemberPath { widget, member, dot_token } = &path;
      let widget = widget_from_field_name(&widget, &member);
      quote! {
        #(#refs)*;
        let state_init = #expr;
        let state_final = #widget #dot_token #member.clone();
        move |p: f32| { #widget.shallow().#member =  Tween::tween(&state_init, &state_final, p); }
      }
    };

    state_tokens = if self.used_name_info.use_or_capture_any_name() {
      let captures = used_name_info.use_or_capture_name().map(capture_widget);
      quote_spanned! { state_span => move |_, _| {
        #(#captures)*
        #state_tokens
      }}
    } else {
      quote_spanned! { state_span => move |_, _| {
        #state_tokens
      }}
    };

    if let Some(Id { name, .. }) = id.as_ref() {
      state_tokens = quote_spanned! {state_span =>  let #name = #state_tokens;};
    }
    tokens.extend(state_tokens);
  }
}

impl ToTokens for Transition {
  fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
    let Self { transition_token, id, fields, .. } = self;

    let fields = fields.iter();
    let mut transition = quote_spanned! { transition_token.span() =>
      <#transition_token as Declare>::builder()
        #(#fields)*
        .build_without_ctx()
    };

    if let Some(Id { name, .. }) = id.as_ref() {
      transition = quote_spanned! { self.transition_token.span() =>  let #name = #transition ;}
    }
    tokens.extend(transition)
  }
}

impl ToTokens for Trigger {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    let Self {
      path: path @ MemberPath { widget, member, dot_token },
      expr,
      ..
    } = self;

    let trigger_span = widget.span().join(expr.span()).unwrap();
    let animate = ribir_variable("animate", expr.span());

    // todo: need a way to detect if it trigger by listener.
    let is_listener = false;
    if is_listener {
      let expr = match expr {
        AnimateExpr::Animate(a) => {
          let mut tokens = quote! {};
          a.embed_as_expr_tokens(&mut tokens);
          tokens
        }
        AnimateExpr::Transition(t) => quote_spanned! { t.transition_token.span() =>
          compile_error!("`Transition can not directly use for listener trigger, use `Animate` instead of.`")
        },
        AnimateExpr::Expr(e) => {
          quote! { #e }
        }
      };
      tokens.extend(quote_spanned! { trigger_span =>
        let mut #animate = #expr;
        #path (move |_|{ #animate.start();} );
      })
    } else {
      let widget = widget_from_field_name(&widget, &member);

      let expr = match expr {
        AnimateExpr::Animate(a) => {
          let mut tokens = quote! {};
          a.embed_as_expr_tokens(&mut tokens);
          tokens
        }
        AnimateExpr::Transition(t) => {
          let transition = if let Some(Id { name, .. }) = t.id.as_ref() {
            quote! {#name}
          } else {
            quote! {#t}
          };
          let ctx_name = ribir_variable(BUILD_CTX, t.span());
          quote_spanned! { t.transition_token.span() =>
            Animate::new(
              move |init_v, final_v| move |p| {
                #widget.shallow().#member = Tween::tween(&init_v, &final_v, p);
              },
              &#transition,
              #ctx_name)
          }
        }
        AnimateExpr::Expr(e) => {
          quote! {#e}
        }
      };

      let w_def = widget_def_variable(&widget);
      tokens.extend(quote_spanned! { trigger_span =>
        let mut #animate = #expr;
        #w_def
        .state_change(move |w| w #dot_token #member.clone())
        .subscribe(move |change| {
          // todo: should remove after support state change hook before change notify
          #animate.cancel();
          if change.before != change.after {
            #animate.restart(change.before, change.after);
          }
        });
      })
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

impl ToTokens for SimpleField {
  fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
    let Self { member, colon_token, expr } = self;
    if colon_token.is_some() {
      tokens.extend(quote! { .#member(#expr) })
    } else {
      tokens.extend(quote! { .#member(#member) })
    }
  }
}

pub enum AnimationObject<'a> {
  Animate(&'a Animate),
  Transition(&'a Transition),
  State(&'a State),
}

impl DeclareCtx {
  pub fn visit_animations_mut(&mut self, animations: &mut Animations) {
    let Animations {
      animates_def,
      states_def,
      transitions_def,
      triggers,
      ..
    } = animations;

    animates_def
      .iter_mut()
      .for_each(|a| self.visit_animate_mut(a));
    states_def.iter_mut().for_each(|s| self.visit_state_mut(s));
    transitions_def
      .iter_mut()
      .for_each(|t| self.visit_transition_mut(t));
    triggers.iter_mut().for_each(|t| self.visit_trigger_mut(t));
  }

  fn visit_animate_mut(&mut self, animate: &mut Animate) {
    let Animate { from, transition, used_name_info, .. } = animate;
    match &mut from.expr {
      StateExpr::State(state) => {
        self.visit_state_mut(state);
        if let Some(Id { name, .. }) = state.id.as_ref() {
          self.add_used_widget(name.clone());
        }
      }
      StateExpr::Expr(expr) => self.visit_expr_mut(expr),
    };
    match &mut transition.expr {
      TransitionExpr::Transition(t) => {
        self.visit_transition_mut(t);
        if let Some(Id { name, .. }) = t.id.as_ref() {
          self.add_used_widget(name.clone());
        }
      }
      TransitionExpr::Expr(e) => self.visit_expr_mut(e),
    }
    *used_name_info = self.take_current_used_info();
  }

  fn visit_state_mut(&mut self, state: &mut State) {
    state.fields.iter_mut().for_each(|p| {
      self.visit_member_path(&mut p.path);
      self.visit_expr_mut(&mut p.expr);
    });
    state.used_name_info = self.take_current_used_info();
  }

  fn visit_transition_mut(&mut self, transition: &mut Transition) {
    transition
      .fields
      .iter_mut()
      .for_each(|f| self.visit_simple_field_mut(f));
    transition.used_name_info = self.take_current_used_info();
  }

  fn visit_trigger_mut(&mut self, trigger: &mut Trigger) {
    let Trigger { path: trigger, expr, .. } = trigger;
    self.visit_member_path(trigger);
    match expr {
      AnimateExpr::Animate(a) => {
        self.visit_animate_mut(a);
      }
      AnimateExpr::Transition(t) => {
        self.visit_transition_mut(t);
      }
      AnimateExpr::Expr(expr) => self.visit_expr_mut(expr),
    }
    self.take_current_used_info();
  }

  fn visit_member_path(&mut self, path: &mut MemberPath) {
    self.add_used_widget(widget_from_field_name(&path.widget, &path.member));
  }

  fn visit_simple_field_mut(&mut self, f: &mut SimpleField) { self.visit_expr_mut(&mut f.expr); }
}

impl Animations {
  pub fn names(&self) -> impl Iterator<Item = &Ident> {
    self.named_objects_iter().map(|o| o.name())
  }

  // return the key-value map of the named widget define tokens.
  pub fn named_objects_def_tokens_iter(&self) -> impl Iterator<Item = (Ident, TokenStream)> + '_ {
    self.named_objects_iter().map(|o| {
      let tokens = match o {
        AnimationObject::Animate(a) => quote! { #a },
        AnimationObject::Transition(t) => quote! {#t},
        AnimationObject::State(s) => quote! {#s},
      };
      (o.name().clone(), tokens)
    })
  }

  pub fn follows_iter(&self) -> impl Iterator<Item = (Ident, Depends)> + '_ {
    self
      .named_objects_iter()
      .filter_map(|n| n.depends().map(|d| (n.name().clone(), d)))
  }

  pub fn named_objects_iter(&self) -> impl Iterator<Item = AnimationObject> + '_ {
    fn named_objects_in_animate<'a>(a: &'a Animate) -> impl Iterator<Item = AnimationObject> {
      let Animate { id, from, transition, .. } = a;
      id.as_ref()
        .map(|_| AnimationObject::Animate(a))
        .into_iter()
        .chain(
          if let FromStateField {
            expr: StateExpr::State(s @ State { id: Some(_), .. }),
            ..
          } = from
          {
            Some(AnimationObject::State(s))
          } else {
            None
          }
          .into_iter(),
        )
        .chain(
          if let TransitionField {
            expr: TransitionExpr::Transition(t @ Transition { id: Some(_), .. }),
            ..
          } = transition
          {
            Some(AnimationObject::Transition(t))
          } else {
            None
          }
          .into_iter(),
        )
    }

    self
      .animates_def
      .iter()
      .flat_map(named_objects_in_animate)
      .chain(self.states_def.iter().map(AnimationObject::State))
      .chain(self.transitions_def.iter().map(AnimationObject::Transition))
      .chain(
        self
          .triggers
          .iter()
          .filter_map(|t| match &t.expr {
            AnimateExpr::Animate(a) => {
              let iter: Box<dyn Iterator<Item = AnimationObject>> =
                Box::new(named_objects_in_animate(a));
              Some(iter)
            }
            AnimateExpr::Transition(t @ Transition { id: Some(_), .. }) => {
              let iter: Box<dyn Iterator<Item = AnimationObject>> =
                Box::new(std::iter::once(AnimationObject::Transition(t)));
              Some(iter)
            }
            _ => None,
          })
          .flatten(),
      )
  }
}

impl<'a> AnimationObject<'a> {
  fn name(&self) -> &'a Ident {
    let id = match self {
      AnimationObject::Animate(a) => a.id.as_ref(),
      AnimationObject::Transition(t) => t.id.as_ref(),
      AnimationObject::State(s) => s.id.as_ref(),
    };
    &id.unwrap().name
  }

  fn depends(&self) -> Option<Depends<'a>> {
    match self {
      AnimationObject::Animate(a) => a.depends(),
      AnimationObject::Transition(t) => t.depends(),
      AnimationObject::State(s) => s.depends(),
    }
  }
}

impl Animate {
  #[inline]
  pub fn depends(&self) -> Option<Depends> { self.used_name_info.depends(DependIn::Animate(self)) }
}

impl State {
  #[inline]
  pub fn depends(&self) -> Option<Depends> { self.used_name_info.depends(DependIn::State(self)) }
}

impl Transition {
  #[inline]
  pub fn depends(&self) -> Option<Depends> {
    self.used_name_info.depends(DependIn::Transition(self))
  }
}

impl Spanned for AnimateExpr {
  #[inline]
  fn span(&self) -> proc_macro2::Span {
    match self {
      AnimateExpr::Animate(a) => a.animate_token.span().join(a.brace_token.span).unwrap(),
      AnimateExpr::Transition(t) => t.span(),
      AnimateExpr::Expr(e) => e.span(),
    }
  }
}
