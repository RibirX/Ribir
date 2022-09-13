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

use crate::widget_attr_macro::{Id, IdType};

use super::{
  capture_widget,
  declare_widget::{
    assign_uninit_field, check_duplicate_field, is_listener, pick_fields_by, BuiltinFieldWidgets,
    WidgetGen, FIELD_WIDGET_TYPE,
  },
  ribir_suffix_variable, ribir_variable, DeclareCtx, ObjectUsed, ScopeUsedInfo, UsedType,
};
use super::{declare_widget::DeclareField, kw};

pub struct Animations {
  animations_token: kw::animations,
  brace_token: token::Brace,
  triggers: Punctuated<Trigger, token::Comma>,
}

#[derive(Debug)]
pub struct State {
  state_token: kw::State,
  brace_token: token::Brace,
  fields: Punctuated<StateField, token::Comma>,
  expr_used: ScopeUsedInfo,
  target_used: ScopeUsedInfo,
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
  animate_token: Ident,
  _brace_token: token::Brace,
  id: Option<Id>,
  from: FromStateField,
  transition: TransitionField,
  lerp_fn: DeclareField,
  directly_used: ScopeUsedInfo,
}
mod animate_kw {
  syn::custom_keyword!(from);
  syn::custom_keyword!(transition);
  syn::custom_keyword!(animation);
  syn::custom_keyword!(lerp_fn);
}

struct AnimateParser {
  animate_token: Ident,
  _brace_token: token::Brace,
  id: Option<Id>,
  from: Option<FromStateField>,
  transition: TransitionField,
  lerp_fn: DeclareField,
}

#[derive(Debug)]
struct FromStateField {
  from_token: Ident,
  colon_token: token::Colon,
  expr: State,
}

#[derive(Debug)]
enum AnimateTransitionValue {
  Transition(Transition),
  Expr {
    expr: syn::Expr,
    used_name_info: ScopeUsedInfo,
  },
}
#[derive(Debug)]
struct TransitionField {
  transition_token: Ident,
  colon_token: Option<token::Colon>,
  value: AnimateTransitionValue,
}

struct Trigger {
  path: MemberPath,
  _colon_token: token::Colon,
  expr: AnimateExpr,
  env: Option<TokenStream>,
  eval_before_trigger: Option<TokenStream>,
}

enum AnimateExpr {
  /// a.on_click: Animate { ... }
  Animate(Animate),
  Expr {
    expr: syn::Expr,
    used_name_info: ScopeUsedInfo,
  },
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
  _colon_token: Option<token::Colon>,
  value: Expr,
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
    let content;
    Ok(Animations {
      animations_token: input.parse()?,
      brace_token: braced!(content in input),
      triggers: content.parse_terminated(Trigger::parse)?,
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
    let captures = self.target_objs().map(capture_widget);
    quote! { #(#captures)*}
  }

  fn target_objs(&self) -> impl Iterator<Item = &Ident> {
    self
      .target_used
      .all_widgets()
      .expect("State target widget muse not be empty.")
  }

  fn maybe_tuple_value(&self, value_by_field: impl Fn(&StateField) -> TokenStream) -> TokenStream {
    let value_tokens = self.fields.iter().map(|s| value_by_field(s));
    if self.fields.len() > 1 {
      quote! { (#(#value_tokens), *)}
    } else {
      quote! { #(#value_tokens), *}
    }
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
      target_used: <_>::default(),
    };
    if state.fields.is_empty() {
      Err(syn::Error::new(state.span(), "`State` must not be empty."))
    } else {
      Ok(state)
    }
  }
}

impl Parse for AnimateParser {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    let animate_token = input.parse::<Ident>()?;
    let content;
    let brace_token = braced!(content in input);
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
        content.parse::<token::Comma>()?;
      }
      if content.is_empty() {
        break;
      }
    }

    let Fields { id, from, transition, lerp_fn } = fields;
    let transition =
      transition.ok_or_else(|| Error::new(animate_token.span(), "miss `transition` field."))?;

    let lerp_fn = lerp_fn.unwrap_or_else(|| {
      parse_quote! {
       lerp_fn: |from, to, rate| Lerp::lerp(from, to, rate)
      }
    });
    Ok(AnimateParser {
      animate_token,
      _brace_token: brace_token,
      id,
      from,
      transition,
      lerp_fn,
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
      from_token: input.parse()?,
      colon_token: input.parse()?,
      expr: input.parse()?,
    })
  }
}

impl Parse for TransitionField {
  fn parse(input: ParseStream) -> Result<Self> {
    let transition_token: animate_kw::transition = input.parse()?;
    let transition_token = parse_quote! {#transition_token};
    let _colon_token: Option<token::Colon> = input.parse()?;
    let value = if _colon_token.is_some() {
      input.parse()?
    } else {
      parse_quote! {#transition_token}
    };
    Ok(TransitionField {
      transition_token,
      colon_token: _colon_token,
      value,
    })
  }
}

impl Parse for AnimateTransitionValue {
  fn parse(input: ParseStream) -> Result<Self> {
    let expr = if input.peek(kw::Transition) {
      AnimateTransitionValue::Transition(input.parse()?)
    } else {
      AnimateTransitionValue::Expr {
        expr: input.parse()?,
        used_name_info: <_>::default(),
      }
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
    let path = input.parse()?;
    let _colon_token: Option<_> = input.parse()?;
    let value = if _colon_token.is_some() {
      input.parse()?
    } else {
      parse_quote!(#path)
    };

    Ok(Self { path, _colon_token, value })
  }
}

impl Parse for Trigger {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    let path: MemberPath = input.parse()?;
    let _colon_token = input.parse()?;
    let mut eval_before_trigger = None;
    let mut env = None;
    let expr = if input.peek(kw::Animate) {
      let a = input.parse::<AnimateParser>()?;
      AnimateExpr::Animate(a.into_animate(&path, &mut env, &mut eval_before_trigger)?)
    } else if input.peek(kw::Transition) {
      let transition: Transition = input.parse()?;
      let animate: AnimateParser = parse_quote! {
        Animate { transition: #transition }
      };
      AnimateExpr::Animate(animate.into_animate(&path, &mut env, &mut eval_before_trigger)?)
    } else {
      AnimateExpr::Expr {
        expr: input.parse()?,
        used_name_info: <_>::default(),
      }
    };

    Ok(Trigger {
      path,
      _colon_token,
      expr,
      eval_before_trigger,
      env,
    })
  }
}

impl Animations {
  pub fn gen_tokens(&mut self, ctx: &mut DeclareCtx, tokens: &mut TokenStream) {
    self
      .triggers
      .iter_mut()
      .for_each(|t| t.gen_tokens(tokens, ctx));
  }
}

impl Animate {
  fn gen_tokens(&self, tokens: &mut TokenStream, ctx: &DeclareCtx) {
    let Self {
      animate_token,
      from,
      transition,
      lerp_fn,
      ..
    } = self;

    let from_field = parse_quote! { #from };
    let transition_field = transition.to_declare_field(ctx);

    let name = self.variable_name();
    let ty = parse_quote! {#animate_token<_, _, _, _, _, _>};
    let fields = [&from_field, &transition_field, lerp_fn];
    let gen = WidgetGen::new(&ty, &name, fields.into_iter(), true);
    let animate_def = gen.gen_widget_tokens(ctx);
    animate_def.to_tokens(tokens);
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
  fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
    self.from_token.to_tokens(tokens);
    self.colon_token.to_tokens(tokens);
    self.expr.to_tokens(tokens);
  }
}

impl TransitionField {
  fn to_declare_field(&self, ctx: &DeclareCtx) -> DeclareField {
    let TransitionField { transition_token, colon_token, value } = self;

    match value {
      AnimateTransitionValue::Transition(t) => {
        // named object is already define before
        if let Some(Id { name, .. }) = t.id.as_ref() {
          parse_quote! { #transition_token #colon_token #name.clone() }
        } else {
          let mut transition_tokens = quote! {};
          t.gen_tokens(&mut transition_tokens, ctx);
          t.variable_name().to_tokens(&mut transition_tokens);
          parse_quote! { #transition_token #colon_token { #transition_tokens } }
        }
      }
      AnimateTransitionValue::Expr { expr, used_name_info } => DeclareField {
        skip_nc: None,
        member: transition_token.clone(),
        colon_token: colon_token.clone(),
        expr: expr.clone(),
        used_name_info: used_name_info.clone(),
      },
    }
  }
}

impl Spanned for TransitionField {
  fn span(&self) -> Span {
    match &self.value {
      AnimateTransitionValue::Transition(t) => t.span(),
      AnimateTransitionValue::Expr { expr, .. } => expr.span(),
    }
  }
}

impl ToTokens for State {
  fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
    let Self {
      state_token, expr_used, brace_token, ..
    } = self;
    let state_span = state_token.span.join(brace_token.span).unwrap();

    let init_value = self.maybe_tuple_value(|StateField { value: expr, .. }| quote! {#expr});

    let init_refs = expr_used
      .directly_used_widgets()
      .into_iter()
      .flat_map(|names| {
        names.map(|name| quote_spanned! { name.span() =>  let #name = #name.raw_ref(); })
      });
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
    let target_refs = self
      .target_objs()
      .map(|name| quote_spanned! { name.span() =>  let #name = #name.raw_ref(); });

    let target_mut_refs = self
      .target_objs()
      .map(|name| quote_spanned! { name.span() =>  let mut #name = #name.shallow_ref(); });
    let target_value = self.maybe_tuple_value(|field| {
      let value = field.path.to_real_widget_tokens();
      quote! { #value.clone()}
    });
    let target_assign = self.maybe_tuple_value(|field| field.path.to_real_widget_tokens());

    let v = ribir_variable("v", state_span);
    tokens.extend(quote_spanned! { state_span =>
      AnimateState::new(
        #init_fn,
        {
          #target_captures
          move || {
            #(#target_refs)*
            #target_value
          }
        },
        {
          #target_captures
          move |#v| {
            #(#target_mut_refs)*
            #target_assign = #v;
          }
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
    let gen = WidgetGen::new(&ty, &name, fields.iter(), false);
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
  pub fn gen_tokens(&mut self, tokens: &mut TokenStream, ctx: &mut DeclareCtx) {
    // define animation
    let Self { expr, env, eval_before_trigger, .. } = self;
    env.to_tokens(tokens);

    let animate_name = expr.variable_name();
    let trigger = match expr {
      AnimateExpr::Animate(a) => {
        if a.id.is_none() {
          a.gen_tokens(tokens, ctx);
        }
        quote! {{
          let #animate_name = #animate_name.clone_stateful();
          move |change| {
            #eval_before_trigger
            #animate_name.run()
          }
        }}
      }
      AnimateExpr::Expr { expr, used_name_info } => {
        let mut run_fn = quote! { move |change| {
          #eval_before_trigger
          (#expr).run()
        }};
        if let Some(captures) = used_name_info.all_widgets() {
          let captures = captures.map(capture_widget);
          run_fn = quote! {{
            #(#captures)*
            #run_fn
          }}
        }
        run_fn
      }
    };

    self.subscribe_to_trigger_animate(trigger, tokens, ctx);
  }

  fn subscribe_to_trigger_animate(
    &self,
    run_fn: TokenStream,
    tokens: &mut TokenStream,
    ctx: &DeclareCtx,
  ) {
    if let Some(listener) = self.listener_trigger_ty() {
      self.path.on_real_widget_name(|name| {
        let ty = Ident::new(listener, self.path.span()).into();
        let member = &self.path.member;
        let fields = [parse_quote! {#member: #run_fn}];
        let gen = WidgetGen::new(&ty, name, fields.iter(), false);

        if ctx
          .named_objects
          .get(name)
          .map_or(false, |id_ty| id_ty.contains(IdType::DECLARE))
        {
          let listener = gen.gen_widget_tokens(ctx);
          tokens.extend(quote! {
            let #name: SingleChildWidget<_, _> = {
              let tmp = #name;
              #listener
              #name.have_child(tmp)
            };
          });
        } else {
          tokens.extend(gen.gen_widget_tokens(ctx));
        }
      });
    } else {
      self.path.on_real_widget_name(|name| {
        let MemberPath { dot_token, member, .. } = &self.path;
        tokens.extend(quote_spanned! { self.span() =>
          #name.clone_stateful()
            .state_change(|w| w #dot_token #member #dot_token clone())
            .filter(StateChange::not_same)
            .subscribe(#run_fn);
        })
      });
    }
  }

  fn listener_trigger_ty(&self) -> Option<&str> {
    FIELD_WIDGET_TYPE
      .get(self.path.member.to_string().as_str())
      .filter(|name| name.ends_with("Listener"))
      .cloned()
  }
}

impl MemberPath {
  fn to_real_widget_tokens(&self) -> TokenStream {
    let mut tokens = quote! {};
    self.on_real_widget_name(|w| w.to_tokens(&mut tokens));
    self.dot_token.to_tokens(&mut tokens);
    self.member.to_tokens(&mut tokens);
    tokens
  }

  fn as_state(&self) -> Result<MemberPathAsState> {
    if self.is_listener().is_some() {
      return Err(syn::Error::new(
        self.span(),
        "A listener trigger, can not use as an implicit `State`.",
      ));
    }
    let init = ribir_variable("init_state", self.member.span());
    let init_2 = ribir_suffix_variable(&init, "2");

    let MemberPath { dot_token, member, .. } = self;
    let mut init_env = quote! {};
    self.on_real_widget_name(|name| {
      init_env = quote_spanned! { self.span() =>
        let #init = std::rc::Rc::new(std::cell::RefCell::new(
          #name #dot_token raw_ref() #dot_token #member #dot_token clone()
        ));
        let #init_2 = #init.clone();
      };
    });

    let state = quote! {
      State { #self: #init.borrow().clone()}
    };
    let animate_trigger_eval = quote! {
      *#init_2.borrow_mut() = change.before.clone();
    };
    Ok(MemberPathAsState {
      init_env,
      state,
      animate_trigger_eval,
    })
  }

  fn is_listener(&self) -> Option<&str> {
    FIELD_WIDGET_TYPE
      .get(self.member.to_string().as_str())
      .filter(|ty_name| is_listener(ty_name))
      .cloned()
  }
}

struct MemberPathAsState {
  init_env: TokenStream,
  state: TokenStream,
  animate_trigger_eval: TokenStream,
}

impl ToTokens for MemberPath {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    let Self { widget, dot_token, member } = self;
    widget.to_tokens(tokens);
    dot_token.to_tokens(tokens);
    member.to_tokens(tokens);
  }
}

impl ToTokens for SimpleField {
  fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
    let Self { member, expr, .. } = self;
    tokens.extend(quote! { .#member(#expr) });
  }
}

pub enum AnimationObject<'a> {
  Animate(&'a Animate),
  Transition(&'a Transition),
}

impl DeclareCtx {
  pub fn visit_animations_mut(&mut self, animations: &mut Animations) {
    let Animations { triggers, .. } = animations;
    triggers.iter_mut().for_each(|t| self.visit_trigger_mut(t));
  }

  fn visit_animate_mut(&mut self, animate: &mut Animate) {
    let Animate { from, transition, lerp_fn, .. } = animate;
    self.visit_state_mut(&mut from.expr);
    match &mut transition.value {
      AnimateTransitionValue::Transition(t) => {
        self.visit_transition_mut(t);
      }
      AnimateTransitionValue::Expr { expr, used_name_info } => {
        self.visit_expr_mut(expr);
        *used_name_info = self.take_current_used_info();
      }
    }
    self.visit_declare_field_mut(lerp_fn);
    if let AnimateTransitionValue::Transition(Transition { id: Some(Id { name, .. }), .. }) =
      &transition.value
    {
      self.add_used_widget(name.clone(), UsedType::USED);
    }
    animate.directly_used = self.take_current_used_info();
  }

  fn visit_state_mut(&mut self, state: &mut State) {
    state
      .fields
      .iter_mut()
      .for_each(|f| self.visit_expr_mut(&mut f.value));

    state.expr_used = self.take_current_used_info();

    state
      .fields
      .iter_mut()
      .for_each(|p| self.visit_member_path(&mut p.path));
    state.target_used = self.take_current_used_info();
  }

  fn visit_transition_mut(&mut self, transition: &mut Transition) {
    transition
      .fields
      .iter_mut()
      .for_each(|f| self.visit_declare_field_mut(f));
  }

  fn visit_trigger_mut(&mut self, trigger: &mut Trigger) {
    match &mut trigger.expr {
      AnimateExpr::Animate(a) => {
        self.visit_animate_mut(a);
        // animate declare in trigger will used by trigger.
        if let Some(id) = a.id.as_ref() {
          self.add_used_widget(id.name.clone(), UsedType::USED)
        }
      }
      AnimateExpr::Expr { expr, used_name_info } => {
        self.visit_expr_mut(expr);
        *used_name_info = self.take_current_used_info();
      }
    }
    self.visit_member_path(&mut trigger.path);
    self.take_current_used_info();
  }

  fn visit_member_path(&mut self, path: &mut MemberPath) {
    let MemberPath { widget, member, .. } = path;
    if let Some(builtin) = self.find_builtin_access(widget, member) {
      // listener trigger is not be used and follow change, but gen be animate self,
      // but need to tell builtin compose.
      if path.is_listener().is_some() {
        self.animate_listener_triggers.insert(builtin);
      } else {
        self.add_used_widget(builtin, UsedType::USED);
      }
    } else {
      self.add_used_widget(widget.clone(), UsedType::USED);
    }
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
            value: AnimateTransitionValue::Transition(t @ Transition { id: Some(_), .. }),
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
      .triggers
      .iter()
      .filter_map(|t| match &t.expr {
        AnimateExpr::Animate(a) => Some(named_objects_in_animate(a)),
        _ => None,
      })
      .flatten()
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
  fn used_part(&self) -> Option<ObjectUsed> {
    let FromStateField { from_token, expr, .. } = &self.from;
    let mut used_objs = [
      self
        .directly_used
        .used_part(Some(&self.animate_token), false),
      expr.expr_used.used_part(Some(from_token), false),
      expr.target_used.used_part(Some(from_token), false),
    ]
    .into_iter()
    .filter_map(|o| o)
    .collect::<Vec<_>>();

    let TransitionField { transition_token, value, .. } = &self.transition;
    match value {
      AnimateTransitionValue::Transition(t) => {
        if t.id.is_none() {
          used_objs.extend(t.fields.iter().filter_map(|f| f.used_part()));
        }
      }
      AnimateTransitionValue::Expr { used_name_info, .. } => {
        if let Some(o) = used_name_info.used_part(Some(&transition_token), false) {
          used_objs.push(o);
        }
      }
    };

    (!used_objs.is_empty()).then(|| ObjectUsed(used_objs.into_boxed_slice()))
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
      AnimateExpr::Expr { expr, .. } => expr.span(),
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

impl Spanned for Trigger {
  fn span(&self) -> Span { self.path.span().join(self.expr.span()).unwrap() }
}

impl AnimateParser {
  fn into_animate(
    self,
    trigger_path: &MemberPath,
    env: &mut Option<TokenStream>,
    eval_before_trigger: &mut Option<TokenStream>,
  ) -> Result<Animate> {
    let Self {
      animate_token,
      _brace_token,
      id,
      from,
      transition,
      lerp_fn,
    } = self;

    let from = match from {
      Some(f) => f,
      None => {
        let MemberPathAsState {
          init_env,
          state,
          animate_trigger_eval,
        } = trigger_path.as_state()?;
        *env = Some(init_env);
        *eval_before_trigger = Some(animate_trigger_eval);
        parse_quote!(from: #state)
      }
    };

    Ok(Animate {
      animate_token,
      _brace_token,
      id,
      from,
      transition,
      lerp_fn,
      directly_used: <_>::default(),
    })
  }
}
