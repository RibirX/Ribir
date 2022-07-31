use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned, ToTokens};
use syn::{
  braced,
  parse::{Parse, ParseStream},
  parse_quote, parse_quote_spanned,
  punctuated::Punctuated,
  spanned::Spanned,
  token,
  visit_mut::VisitMut,
  Error, Expr, Ident, Result,
};

use crate::widget_attr_macro::Id;

use super::{
  capture_widget,
  declare_widget::{
    assign_uninit_field, check_duplicate_field, pick_fields_by, BuiltinFieldWidgets, WidgetGen,
    FIELD_WIDGET_TYPE,
  },
  ribir_suffix_variable, ribir_variable, DeclareCtx, ObjectUsed, ScopeUsedInfo, UsedType,
  BUILD_CTX,
};
use super::{declare_widget::DeclareField, kw};

pub struct Animations {
  animations_token: kw::animations,
  brace_token: token::Brace,
  animates_def: Vec<Animate>,
  transitions_def: Vec<Transition>,
  triggers: Punctuated<Trigger, token::Comma>,
}

#[derive(Debug)]
pub struct State {
  state_token: kw::State,
  brace_token: token::Brace,
  fields: Punctuated<StateField, token::Comma>,
  expr_used: ScopeUsedInfo,
}

#[derive(Debug)]
pub struct Transition {
  transition_token: kw::Transition,
  brace_token: token::Brace,
  id: Option<Id>,
  fields: Punctuated<DeclareField, token::Comma>,
}

#[derive(Debug)]
pub struct Animate {
  animate_token: kw::Animate,
  _brace_token: token::Brace,
  id: Option<Id>,
  from: FromStateField,
  //todo: as a declare field can follow ?
  transition: TransitionField,
  used_name_info: ScopeUsedInfo,
}
mod animate_kw {
  syn::custom_keyword!(from);
  syn::custom_keyword!(transition);
  syn::custom_keyword!(animation);
}

#[derive(Debug)]
struct FromStateField {
  _from_token: animate_kw::from,
  _colon_token: token::Colon,
  expr: State,
}

#[derive(Debug)]
enum TransitionExpr {
  Transition(Transition),
  Expr(syn::Expr),
}
#[derive(Debug)]
struct TransitionField {
  _transition_token: animate_kw::transition,
  _colon_token: Option<token::Colon>,
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
struct StateField {
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

impl Parse for Transition {
  fn parse(input: ParseStream) -> Result<Self> {
    let content;
    let mut res = Self {
      transition_token: input.parse()?,
      brace_token: braced!( content in input),
      id: None,
      fields: content.parse_terminated(DeclareField::parse)?,
    };

    check_duplicate_field(&res.fields)?;
    pick_fields_by(&mut res.fields, |p| {
      if p.value().is_id_field() {
        res.id = Some(Id::from_field_pair(p)?);
        Ok(None)
      } else {
        Ok(Some(p))
      }
    })?;

    Ok(res)
  }
}

impl Parse for Animations {
  fn parse(input: ParseStream) -> Result<Self> {
    let animations_token = input.parse()?;
    let content;
    let brace_token = braced!(content in input);

    let mut animates_def: Vec<Animate> = vec![];
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
      transitions_def,
      triggers,
    })
  }
}

impl Spanned for Animations {
  fn span(&self) -> proc_macro2::Span {
    self
      .animations_token
      .span
      .join(self.brace_token.span)
      .unwrap()
  }
}

impl MemberPath {
  fn on_real_widget_name(&self, mut cb: impl FnMut(&Ident)) {
    let Self { widget, member, .. } = self;
    if let Some(suffix) = BuiltinFieldWidgets::as_builtin_widget(member) {
      cb(&ribir_suffix_variable(widget, suffix))
    } else {
      cb(widget)
    }
  }
}

impl State {
  // return the capture tokens of the widgets the state want to modify.
  fn capture_target_tokens(&self) -> TokenStream {
    let mut tokens = quote! {};
    self.fields.iter().for_each(|f| {
      f.path
        .on_real_widget_name(|w| capture_widget(w).to_tokens(&mut tokens))
    });
    tokens
  }
}

impl Parse for State {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    let content;
    let state = Self {
      state_token: input.parse()?,
      brace_token: braced!(content in input),
      fields: Punctuated::parse_terminated(&content)?,
      expr_used: <_>::default(),
    };
    if state.fields.is_empty() {
      Err(syn::Error::new(state.span(), "`State` must not be empty."))
    } else {
      Ok(state)
    }
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
      } else {
        return Err(lk.error());
      }
      if !content.is_empty() {
        content.parse::<token::Comma>()?;
      }
      if content.is_empty() {
        break;
      }
    }

    let Fields { id, from, transition } = fields;
    let from = from.ok_or_else(|| Error::new(animate_token.span(), "miss `from` field."))?;
    let transition =
      transition.ok_or_else(|| Error::new(animate_token.span(), "miss `transition` field."))?;

    Ok(Animate {
      animate_token,
      _brace_token: brace_token,
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
    Ok(FromStateField {
      _from_token: input.parse()?,
      _colon_token: input.parse()?,
      expr: input.parse()?,
    })
  }
}

impl Parse for TransitionField {
  fn parse(input: ParseStream) -> Result<Self> {
    let _transition_token: animate_kw::transition = input.parse()?;
    let _colon_token: Option<token::Colon> = input.parse()?;
    let expr = if _colon_token.is_some() {
      input.parse()?
    } else {
      parse_quote! {#_transition_token}
    };
    Ok(TransitionField {
      _transition_token,
      _colon_token,
      expr,
    })
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

impl Parse for StateField {
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
  pub fn gen_tokens(&self, ctx: &DeclareCtx, tokens: &mut TokenStream) {
    self.brace_token.surround(tokens, |tokens| {
      self.triggers.iter().for_each(|t| t.gen_tokens(tokens, ctx));
    });
  }
}

impl Animate {
  fn gen_tokens(&self, tokens: &mut TokenStream, ctx: &DeclareCtx) {
    let Self { animate_token, from, transition, .. } = self;

    let animate_span = animate_token.span();
    let build_ctx = ribir_variable(BUILD_CTX, animate_span);
    let mut transition_token = quote! {};
    let name = self.variable_name();
    transition.gen_tokens(&mut transition_token, ctx);
    tokens.extend(quote_spanned! { animate_span =>
      let #name = #build_ctx.animate_store().register(
        Box::new(
          <#animate_token<_, _, _, _, _> as Declare>::builder()
            .from(#from)
            .transition(#transition_token)
            .build(#build_ctx)
          )
      );
    });
  }
}

impl AnimateExpr {
  fn variable_name(&self) -> Ident {
    if let AnimateExpr::Animate(a) = &self {
      a.variable_name()
    } else {
      Animate::anonymous_name(self.span())
    }
  }
}

impl Animate {
  fn variable_name(&self) -> Ident {
    self
      .id
      .as_ref()
      .map_or_else(|| Self::anonymous_name(self.span()), |id| id.name.clone())
  }

  fn anonymous_name(span: Span) -> Ident { ribir_variable("animate", span) }
}

impl ToTokens for FromStateField {
  fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) { self.expr.to_tokens(tokens); }
}

impl TransitionField {
  fn gen_tokens(&self, tokens: &mut proc_macro2::TokenStream, ctx: &DeclareCtx) {
    match &self.expr {
      TransitionExpr::Transition(t) => {
        // named object is already define before
        if let Some(Id { name, .. }) = t.id.as_ref() {
          name.to_tokens(tokens);
        } else {
          token::Brace::default().surround(tokens, |tokens| {
            t.gen_tokens(tokens, ctx);
            t.variable_name().to_tokens(tokens);
          });
        }
      }
      TransitionExpr::Expr(e) => e.to_tokens(tokens),
    }
  }
}

impl Spanned for TransitionField {
  fn span(&self) -> Span {
    match &self.expr {
      TransitionExpr::Transition(t) => t.span(),
      TransitionExpr::Expr(e) => e.span(),
    }
  }
}

impl ToTokens for State {
  fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
    let Self {
      state_token,
      fields,
      expr_used,
      brace_token,
    } = self;
    let state_span = state_token.span.join(brace_token.span).unwrap();

    let init_expr = fields.iter().map(|f| &f.expr);

    let init_value = if fields.len() > 1 {
      quote! { (#(#init_expr), *)}
    } else {
      quote! { #(#init_expr), *}
    };
    let init_refs = expr_used.refs_tokens().into_iter().flatten();
    let mut init_fn = quote! {
      move || {
        #(#init_refs)*;
        #init_value
      }
    };
    // because wrap by move closure, so all widgets should as capture widgets.
    if let Some(captures) = expr_used.all_widgets() {
      let capture_objs = captures.map(capture_widget);
      init_fn = quote! {{
        #(#capture_objs)*
        #init_fn
      }};
    };

    let target_captures = self.capture_target_tokens();
    let target_value = fields.iter().map(
      |StateField {
         path: MemberPath { widget, dot_token, member, .. },
         ..
       }| {
        quote! { #widget #dot_token state_ref() #dot_token #member #dot_token clone() }
      },
    );
    let mut shallow_access = fields.iter().map(
      |StateField {
         path: MemberPath { widget, dot_token, member, .. },
         ..
       }| {
        quote! { #widget #dot_token shallow_ref() #dot_token #member}
      },
    );
    let update_fn = if fields.len() > 1 {
      let indexes = (0..fields.len()).map(syn::Index::from);
      quote! { move |val| { #(#shallow_access = val.#indexes;)*} }
    } else {
      let state = shallow_access.next();
      quote! { move |val| { #state = val; } }
    };
    tokens.extend(quote_spanned! { state_span =>
      AnimateState::new(
        #init_fn,
        {
          #target_captures
          move || { #(#target_value)*}
        },
        {
          #target_captures
          #update_fn
        }
      )
    });
  }
}

impl Transition {
  fn gen_tokens(&self, tokens: &mut proc_macro2::TokenStream, ctx: &DeclareCtx) {
    let Self { transition_token, fields, .. } = self;
    let name = self.variable_name();

    let ty = parse_quote_spanned! { transition_token.span => #transition_token <_>};
    let gen = WidgetGen::new(&ty, &name, fields.iter());
    let transition_tokens = gen.gen_widget_tokens(ctx);

    tokens.extend(transition_tokens)
  }
}

impl ToTokens for Transition {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    self.transition_token.to_tokens(tokens);
    self.brace_token.surround(tokens, |tokens| {
      self.id.to_tokens(tokens);
      self.fields.to_tokens(tokens);
    });
  }
}

impl Trigger {
  pub fn gen_tokens(&self, tokens: &mut TokenStream, ctx: &DeclareCtx) {
    // define animation
    let animate_name = self.expr.variable_name();
    match &self.expr {
      AnimateExpr::Animate(a) => {
        if a.id.is_none() {
          a.gen_tokens(tokens, ctx);
        }
        self.animate_subscribe_tokens(tokens);
      }
      AnimateExpr::Transition(transition) => {
        if self.is_listener_trigger() {
          tokens.extend(quote_spanned! { transition.span() =>
            compile_error!("`Transition can not directly use for listener trigger, use `Animate` instead of.`")
          })
        } else {
          self.shorthand_syntax_to_tokens(tokens, ctx);
        }
      }
      AnimateExpr::Expr(e) => {
        tokens.extend(quote_spanned! { e.span() => let #animate_name = #e;});
        self.animate_subscribe_tokens(tokens);
      }
    }
  }

  fn animate_subscribe_tokens(&self, tokens: &mut TokenStream) {
    let animate_name = self.expr.variable_name();
    if self.is_listener_trigger() {
      tokens.extend(quote_spanned! { self.span() =>
        // todo: widget wrap with listener to trigger animate
        move |_|{ #animate_name.start();}
      })
    } else {
      let MemberPath { widget, dot_token, member } = &self.path;
      tokens.extend(quote_spanned! { self.span() =>
        #widget.clone()
          .state_change(|w| &w #dot_token #member)
          .subscribe(move |change| {
            if change.before != change.after {
              #animate_name.start();
            }
          });
      })
    }
  }

  /// When description a animation for state change, a simple syntax can
  /// directly use, eg. `id.background: Transition { ... }.
  ///
  /// This struct helper to generate code for that case.
  fn shorthand_syntax_to_tokens(&self, tokens: &mut TokenStream, ctx: &DeclareCtx) {
    let init = ribir_variable("init_state", self.path.member.span());
    let path = &self.path;
    let MemberPath { widget, dot_token, member } = path;
    let transition = match &self.expr {
      AnimateExpr::Transition(t) => t,
      _ => panic!("Caller should guarantee be `AnimateExpr::Transition`!"),
    };

    let animate: Animate = parse_quote! {
      Animate {
        from: State {
          #path: #init.borrow().clone()
        },
        transition: #transition
      }
    };
    tokens.extend(quote! {
      let #init = std::rc::Rc::new(std::cell::RefCell::new(#path.clone()));
    });
    animate.gen_tokens(tokens, ctx);
    let animate_name = Animate::anonymous_name(transition.span());

    tokens.extend(quote_spanned! { path.span() =>
      #widget.clone()
        .state_change(|w| &w #dot_token #member)
        .subscribe(move |change| {
          if change.before != change.after {
            #init.borrow_mut().set(change.before.clone());
            #animate_name.start();
          }
        });
    });
  }
  fn is_listener_trigger(&self) -> bool {
    let ty_name = FIELD_WIDGET_TYPE.get(self.path.member.to_string().as_str());
    ty_name.map_or(false, |ty| ty.ends_with("Listener"))
  }
}

impl ToTokens for MemberPath {
  fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
    self.on_real_widget_name(|w| w.to_tokens(tokens));
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
}

impl DeclareCtx {
  pub fn visit_animations_mut(&mut self, animations: &mut Animations) {
    let Animations {
      animates_def,
      transitions_def,
      triggers,
      ..
    } = animations;

    animates_def
      .iter_mut()
      .for_each(|a| self.visit_animate_mut(a));
    transitions_def
      .iter_mut()
      .for_each(|t| self.visit_transition_mut(t));
    triggers.iter_mut().for_each(|t| self.visit_trigger_mut(t));
  }

  fn visit_animate_mut(&mut self, animate: &mut Animate) {
    let Animate { from, transition, used_name_info, .. } = animate;
    self.visit_state_mut(&mut from.expr);
    match &mut transition.expr {
      TransitionExpr::Transition(t) => {
        self.visit_transition_mut(t);
        if let Some(Id { name, .. }) = t.id.as_ref() {
          self.add_used_widget(name.clone(), UsedType::USED);
        }
      }
      TransitionExpr::Expr(e) => self.visit_expr_mut(e),
    }
    *used_name_info = self.take_current_used_info();
  }

  fn visit_state_mut(&mut self, state: &mut State) {
    state
      .fields
      .iter_mut()
      .for_each(|p| self.visit_expr_mut(&mut p.expr));

    state.expr_used = self.clone_current_used_info();

    state
      .fields
      .iter_mut()
      .for_each(|p| self.visit_member_path(&mut p.path));

    // All used in state widget should mark as capture.
    self.current_used_info.iter_mut().for_each(|(_, info)| {
      info.used_type = UsedType::MOVE_CAPTURE;
    });
  }

  fn visit_transition_mut(&mut self, transition: &mut Transition) {
    transition
      .fields
      .iter_mut()
      .for_each(|f| self.visit_declare_field_mut(f));
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
    path.on_real_widget_name(|w| {
      self.add_used_widget(w.clone(), UsedType::USED);
    })
  }
}

impl Animations {
  pub fn names(&self) -> impl Iterator<Item = &Ident> {
    self.named_objects_iter().map(|o| o.name())
  }

  // return the key-value map of the named widget define tokens.
  pub fn named_objects_def_tokens_iter<'a>(
    &'a self,
    ctx: &'a DeclareCtx,
  ) -> impl Iterator<Item = (Ident, TokenStream)> + 'a {
    self.named_objects_iter().map(|o| {
      let mut tokens = quote! {};
      match o {
        AnimationObject::Animate(a) => a.gen_tokens(&mut tokens, ctx),
        AnimationObject::Transition(t) => t.gen_tokens(&mut tokens, ctx),
      };
      (o.name().clone(), tokens)
    })
  }

  pub fn dependencies(&self) -> impl Iterator<Item = (Ident, ObjectUsed)> + '_ {
    self
      .named_objects_iter()
      .filter_map(move |n| n.used_part().map(|d| (n.name().clone(), d)))
  }

  pub fn named_objects_iter(&self) -> impl Iterator<Item = AnimationObject> + '_ {
    fn named_objects_in_animate<'a>(a: &'a Animate) -> impl Iterator<Item = AnimationObject> {
      let Animate { id, transition, .. } = a;
      id.as_ref()
        .map(|_| AnimationObject::Animate(a))
        .into_iter()
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
    };
    &id.expect("Try to get name from an anonymous object.").name
  }

  fn used_part(&self) -> Option<ObjectUsed<'a>> {
    match self {
      AnimationObject::Animate(a) => a.used_part(),
      AnimationObject::Transition(t) => t.used_part(),
    }
  }
}

impl Animate {
  pub fn used_part(&self) -> Option<ObjectUsed> {
    self
      .used_name_info
      .used_part(None, false)
      .map(ObjectUsed::from_single_part)
  }
}

impl Transition {
  pub fn used_part(&self) -> Option<ObjectUsed> {
    let used = ObjectUsed::from_iter(self.fields.iter().filter_map(|f| f.used_part()));
    (!used.is_empty()).then(|| used)
  }

  pub fn variable_name(&self) -> Ident {
    if let Some(Id { ref name, .. }) = self.id {
      name.clone()
    } else {
      ribir_variable("transition", self.span())
    }
  }
}

impl Spanned for AnimateExpr {
  #[inline]
  fn span(&self) -> proc_macro2::Span {
    match self {
      AnimateExpr::Animate(a) => a.span(),
      AnimateExpr::Transition(t) => t.span(),
      AnimateExpr::Expr(e) => e.span(),
    }
  }
}

impl Spanned for Animate {
  #[inline]
  fn span(&self) -> proc_macro2::Span {
    self
      .animate_token
      .span
      .join(self._brace_token.span)
      .unwrap()
  }
}

impl Spanned for Trigger {
  fn span(&self) -> Span { self.path.span().join(self.expr.span()).unwrap() }
}
