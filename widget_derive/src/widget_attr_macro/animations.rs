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
  declare_widget::{assign_uninit_field, SugarFields},
  ribir_suffix_variable, ribir_variable, DeclareCtx, FollowOn, FollowPart, FollowPlace, Follows,
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

#[derive(Debug)]
pub struct State {
  state_token: kw::State,
  _brace_token: token::Brace,
  id: Option<Id>,
  fields: Punctuated<PathField, token::Comma>,
  follows: Option<Vec<FollowOn>>,
}

#[derive(Debug)]
pub struct Transition {
  transition_token: kw::Transition,
  _brace_token: token::Brace,
  id: Option<Id>,
  fields: Punctuated<SimpleField, token::Comma>,
  follows: Option<Vec<FollowOn>>,
}

#[derive(Debug)]
pub struct Animate {
  animate_token: kw::Animate,
  brace_token: token::Brace,
  id: Option<Id>,
  from: FromStateField,
  transition: TransitionField,
  follows: Option<Vec<FollowOn>>,
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
struct SimpleField {
  member: Ident,
  colon_token: Option<token::Colon>,
  expr: Expr,
}

struct SimpleStruct<KW, F> {
  name: KW,
  brace_token: token::Brace,
  id: Option<Id>,
  fields: Punctuated<F, token::Comma>,
}


fn widget_from_field_name(widget: &Ident, field: &Ident) -> Ident {
  if let Some(suffix) = SugarFields::widget_name_from_field(field) {
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
      follows: None,
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
      follows: None,
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
      follows: None,
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

    Ok(FromStateField {  expr })
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
    let transition_token:animate_kw::transition = input.parse()?;
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

impl Animations {
  pub fn to_tokens(&self, ctx_name: &Ident) -> TokenStream {
    let mut tokens = quote! {};
    self.brace_token.surround(&mut tokens, |tokens| {
      self
        .triggers
        .iter()
        .for_each(|t| t.to_tokens(ctx_name, tokens));
    });
    tokens
  }
}

impl Animate {
  fn to_tokens(&self, ctx_name: &Ident) -> TokenStream {
    let Self {
      animate_token, id, from, transition, ..
    } = self;

    let animate_span = animate_token.span();

    let mut animate_tokens = quote_spanned! { animate_span =>
      #animate_token::new(
        #from,
        &#transition,
        #ctx_name)
    };

    if let Some(Id { name, .. }) = id.as_ref() {
      animate_tokens = quote_spanned! {animate_span =>
        #[allow(unused_mut)]
        let mut #name = #animate_tokens;
      };
    }

    return animate_tokens;
  }

  fn embed_as_expr_tokens(&self, ctx_name: &Ident) -> TokenStream {
    if let Some(Id { name, .. }) = self.id.as_ref() {
      quote! { #name }
    } else {
      self.to_tokens(ctx_name)
    }
  }
}

impl ToTokens for FromStateField {
  fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
    self.expr.to_tokens(tokens);
  }
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
    let Self { state_token, id, fields, .. } = self;

    let state_span = state_token.span();
    let mut state_tokens = if fields.len() > 1 {
      let init_value = fields.iter().map(|f| &f.expr);
      // let path_members = fields.iter().map(|f| &f.path);
      let widgets = fields.iter().map(|f| widget_from_field_name(&f.path.widget, &f.path.member));
      let widgets2 = widgets.clone();
      let members = fields.iter().map(|f| &f.path.member);
      let members2 = members.clone();
      let indexes = (0..fields.len()).map(syn::Index::from);

      quote_spanned! { state_span =>{ move |_, _| {
            let state_init = (#(#init_value),*);
            let state_final = (#(#widgets2.#members2.clone()),*);
            move |p: f32| { #(#widgets .shallow(). #members = Tween::tween(&state_init.#indexes, &state_final.#indexes, p);)* }
          }
        } 
      }
    } else {
      let PathField { path, _colon_token, expr } = &fields[0];
      let MemberPath { widget, member, dot_token} = &path;
      let widget = widget_from_field_name(&widget, &member);
      quote_spanned! { state_span =>
      move |_, _|  {
          let state_init = #expr;
          let state_final = #widget #dot_token #member.clone();
          move |p: f32| { #widget.shallow().#member =  Tween::tween(&state_init, &state_final, p); }
        }
      }
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

impl Trigger {
  fn to_tokens(&self, ctx_name: &Ident, tokens: &mut TokenStream) {
    let Self {
      path: path @ MemberPath { widget, member, dot_token },
      expr,
      ..
    } = self;
    // todo: not support use attr as trigger, because we may remove attribute
    // concept in future.  else if SugarFields::BUILTIN_DATA_ATTRS.iter().
    // any(|v| member == v) {   let get_attr = Ident::new(&format!("get_{}",
    // quote! {#member}), member.span());   let member = quote_spanned!
    // {member.span() => #get_attr() };   subscribe_tokens(&widget, &member,
    // dot_token, quote! {#expr_tokens}) }

    let trigger_span = widget.span().join(expr.span()).unwrap();
    let animate = ribir_variable("animate", expr.span());

    if SugarFields::BUILTIN_LISTENERS.iter().any(|v| member == v) {
      let expr = match expr {
        AnimateExpr::Animate(a) => a.embed_as_expr_tokens(ctx_name),
        AnimateExpr::Transition(t) => quote_spanned! { t.transition_token.span() =>
          compile_error!("`Transition can not directly use for listener trigger, use `Animate` instead of.`")
        },
        AnimateExpr::Expr(e) => {
          quote! {#e}
        }
      };
      tokens.extend(quote_spanned! { trigger_span =>
        let mut #animate = #expr;
        #path (move |_|{ #animate.start();} );
      })
    } else {
      let widget = widget_from_field_name(&widget, &member);

      let expr = match expr {
        AnimateExpr::Animate(a) => a.embed_as_expr_tokens(ctx_name),
        AnimateExpr::Transition(t) => {
          let transition = if let Some(Id { name, .. }) = t.id.as_ref() {
            quote! {#name}
          } else {
            quote! {#t}
          };
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

      tokens.extend(quote_spanned! { trigger_span =>
        let mut #animate = #expr;
        #widget
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
    let mut ctx = self.borrow_capture_scope(true);
    let Animations {
      animates_def,
      states_def,
      transitions_def,
      triggers,
      ..
    } = animations;

    animates_def
      .iter_mut()
      .for_each(|a| ctx.visit_animate_mut(a));
    states_def.iter_mut().for_each(|s| ctx.visit_state_mut(s));
    transitions_def
      .iter_mut()
      .for_each(|t| ctx.visit_transition_mut(t));
    triggers.iter_mut().for_each(|t| ctx.visit_trigger_mut(t));
  }

  fn visit_animate_mut(&mut self, animate: &mut Animate) {
    let Animate { from, transition, follows, .. } = animate;
    match &mut from.expr {
      StateExpr::State(state) => {
        self.visit_state_mut(state);
        if let Some(Id { name, .. }) = state.id.as_ref() {
          self.add_follow(name.clone());
        }
      }
      StateExpr::Expr(expr) => self.visit_expr_mut(expr),
    };
    match &mut transition.expr {
      TransitionExpr::Transition(t) => {
        self.visit_transition_mut(t);
        if let Some(Id { name, .. }) = t.id.as_ref() {
          self.add_follow(name.clone());
        }
      }
      TransitionExpr::Expr(e) => self.visit_expr_mut(e),
    }
    *follows = self.take_current_follows();
  }

  fn visit_state_mut(&mut self, state: &mut State) {
    state.fields.iter_mut().for_each(|p| {
      self.visit_member_path(&mut p.path);
      self.visit_expr_mut(&mut p.expr);
    });
    state.follows = self.take_current_follows();
  }

  fn visit_transition_mut(&mut self, transition: &mut Transition) {
    transition
      .fields
      .iter_mut()
      .for_each(|f| self.visit_simple_field_mut(f));
    transition.follows = self.take_current_follows();
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
    self.take_current_follows();
  }

  fn visit_member_path(&mut self, path: &mut MemberPath) { self.add_follow(widget_from_field_name(&path.widget, &path.member)); }

  fn visit_simple_field_mut(&mut self, f: &mut SimpleField) { self.visit_expr_mut(&mut f.expr); }
}

impl Animations {
  pub fn names(&self) -> impl Iterator<Item = &Ident> {
    self.named_objects_iter().map(|o| o.name())
  }

  // return the key-value map of the named widget define tokens.
  pub fn named_objects_def_tokens_iter<'a>(
    &'a self,
    ctx_name: &'a Ident,
  ) -> impl Iterator<Item = (Ident, TokenStream)> + 'a {
    self.named_objects_iter().map(|o| {
      let tokens = match o {
        AnimationObject::Animate(a) => a.to_tokens(ctx_name),
        AnimationObject::Transition(t) => quote! {#t},
        AnimationObject::State(s) => quote! {#s},
      };
      (o.name().clone(), tokens)
    })
  }

  pub fn follows_iter(&self) -> impl Iterator<Item = (Ident, Follows)> {
    self.named_objects_iter().filter_map(|n| {
      n.as_follow_part()
        .map(|p| (n.name().clone(), Follows::from_single_part(p)))
    })
  }

  pub fn named_objects_iter(&self) -> impl Iterator<Item = AnimationObject> {
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

  pub fn as_follow_part(&self) -> Option<FollowPart<'a>> {
    match self {
      AnimationObject::Animate(a) => a.as_follow_part(),
      AnimationObject::Transition(t) => t.as_follow_part(),
      AnimationObject::State(s) => s.as_follow_part(),
    }
  }
}

impl Animate {
  fn as_follow_part(&self) -> Option<FollowPart> {
    self.follows.as_ref().map(|follows| FollowPart {
      origin: FollowPlace::Animate(self),
      follows: &*follows,
    })
  }
}

impl State {
  fn as_follow_part(&self) -> Option<FollowPart> {
    self.follows.as_ref().map(|follows| FollowPart {
      origin: FollowPlace::State(self),
      follows: &*follows,
    })
  }
}

impl Transition {
  fn as_follow_part(&self) -> Option<FollowPart> {
    self.follows.as_ref().map(|follows| FollowPart {
      origin: FollowPlace::Transition(self),
      follows: &*follows,
    })
  }
}

impl Spanned for AnimateExpr {
  fn span(&self) -> proc_macro2::Span {
    match self {
      AnimateExpr::Animate(a) => a.animate_token.span().join(a.brace_token.span).unwrap(),
      AnimateExpr::Transition(t) => t.span(),
      AnimateExpr::Expr(e) => e.span(),
    }
  }
}

impl Spanned for Animations {
  fn span(&self) -> proc_macro2::Span {
    self
      .animations_token
      .span()
      .join(self.brace_token.span)
      .unwrap()
  }
}
