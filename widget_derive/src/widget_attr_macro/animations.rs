use std::collections::BTreeMap;

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
  declare_widget::{assign_uninit_field, check_duplicate_field, pick_fields_by, WidgetGen},
  ribir_variable,
  track::SimpleField,
  widget_macro::TrackExpr,
  DeclareCtx, ObjectUsed, ScopeUsedInfo, UsedType,
};
use super::{declare_widget::DeclareField, kw};

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
  pub id: Option<Id>,
  fields: Punctuated<DeclareField, token::Comma>,
}

#[derive(Debug)]
pub struct Animate {
  animate_token: Ident,
  _brace_token: token::Brace,
  pub id: Option<Id>,
  pub from: Option<FromStateField>,
  transition: TransitionField,
  lerp_fn: DeclareField,
}
mod animate_kw {
  syn::custom_keyword!(from);
  syn::custom_keyword!(transition);
  syn::custom_keyword!(animation);
  syn::custom_keyword!(lerp_fn);
}
#[derive(Debug)]
pub struct FromStateField {
  from_token: Ident,
  colon_token: token::Colon,
  expr: State,
}

#[derive(Debug)]
enum AnimateTransitionValue {
  Transition(Transition),
  Expr(TrackExpr),
}
#[derive(Debug)]
struct TransitionField {
  transition_token: Ident,
  colon_token: Option<token::Colon>,
  value: AnimateTransitionValue,
}

#[derive(Debug)]
pub struct MemberPath {
  pub widget: Ident,
  pub dot_token: token::Dot,
  pub member: Ident,
}

#[derive(Debug)]
struct StateField {
  path: MemberPath,
  _colon_token: Option<token::Colon>,
  value: Expr,
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
      AnimateTransitionValue::Expr(input.parse()?)
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

impl Animate {
  pub fn collect_named_defs(&self, ctx: &mut DeclareCtx) {
    if let Some(Id { name, .. }) = self.id.as_ref() {
      let mut tokens = quote! {};
      self.gen_tokens(&mut tokens, ctx);
      ctx.named_obj_defs.insert(name.clone(), tokens);
      if let AnimateTransitionValue::Transition(t) = &self.transition.value {
        t.collect_named_defs(ctx);
      }
    }
  }

  pub fn gen_tokens(&self, tokens: &mut TokenStream, ctx: &mut DeclareCtx) {
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

  pub fn collect_name(&self, ctx: &mut DeclareCtx) {
    let Animate { id, transition, .. } = self;
    ctx.id_collect(id);
    if let TransitionField {
      value: AnimateTransitionValue::Transition(t),
      ..
    } = transition
    {
      ctx.id_collect(&t.id)
    }
  }
}

impl Animate {
  pub fn variable_name(&self) -> Ident {
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
  fn to_declare_field(&self, ctx: &mut DeclareCtx) -> DeclareField {
    let TransitionField { transition_token, colon_token, value } = self;

    match value {
      AnimateTransitionValue::Transition(t) => {
        // named object is already define before
        if let Some(Id { name, .. }) = t.id.as_ref() {
          let mut f: DeclareField =
            parse_quote! { #transition_token #colon_token #name.clone_stateful() };
          f.expr.used_name_info.add_used(name.clone(), UsedType::USED);
          f
        } else {
          let mut transition_tokens = quote! {};
          t.gen_tokens(&mut transition_tokens, ctx);
          t.variable_name().to_tokens(&mut transition_tokens);
          parse_quote! { #transition_token #colon_token { #transition_tokens } }
        }
      }
      AnimateTransitionValue::Expr(expr) => DeclareField {
        skip_nc: None,
        member: transition_token.clone(),
        colon_token: colon_token.clone(),
        expr: expr.clone(),
      },
    }
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
      let path = &field.path;
      quote! { #path.clone()}
    });
    let target_assign = self.maybe_tuple_value(|field| {
      let path = &field.path;
      quote! { #path }
    });

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
  pub fn gen_tokens(&self, tokens: &mut proc_macro2::TokenStream, ctx: &mut DeclareCtx) {
    let Self { transition_token, fields, .. } = self;
    let name = self.variable_name();

    let ty = parse_quote_spanned! { transition_token.span => #transition_token <_>};
    let gen = WidgetGen::new(&ty, &name, fields.iter(), false);
    let transition_tokens = gen.gen_widget_tokens(ctx);

    tokens.extend(transition_tokens)
  }

  pub fn collect_named_defs(&self, ctx: &mut DeclareCtx) {
    if let Some(Id { name, .. }) = self.id.as_ref() {
      let mut tokens = quote! {};
      self.gen_tokens(&mut tokens, ctx);
      ctx.named_obj_defs.insert(name.clone(), tokens);
    }
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

impl DeclareCtx {
  pub fn visit_animate_mut(&mut self, animate: &mut Animate) {
    let Animate { from, transition, lerp_fn, .. } = animate;
    if let Some(from) = from.as_mut() {
      self.visit_state_mut(&mut from.expr);
    }
    match &mut transition.value {
      AnimateTransitionValue::Transition(t) => {
        self.visit_transition_mut(t);
      }
      AnimateTransitionValue::Expr(expr) => self.visit_track_expr(expr),
    }
    self.visit_declare_field_mut(lerp_fn);
  }

  pub fn visit_state_mut(&mut self, state: &mut State) {
    state
      .fields
      .iter_mut()
      .for_each(|f| self.visit_expr_mut(&mut f.value));

    state.expr_used = self.take_current_used_info();

    state
      .fields
      .iter_mut()
      .for_each(|p| self.visit_member_path_mut(&mut p.path));
    state.target_used = self.take_current_used_info();
  }

  pub fn visit_transition_mut(&mut self, transition: &mut Transition) {
    transition
      .fields
      .iter_mut()
      .for_each(|f| self.visit_declare_field_mut(f));
  }

  pub fn visit_member_path_mut(&mut self, path: &mut MemberPath) {
    let MemberPath { widget, member, .. } = path;
    if let Some(builtin) = self.find_builtin_access(widget, member) {
      *widget = parse_quote! { #builtin };
      self.add_used_widget(builtin, UsedType::USED);
    } else {
      self.add_used_widget(widget.clone(), UsedType::USED);
    }
  }
}

impl Animate {
  pub fn analyze_observe_depends<'a>(&'a self, depends: &mut BTreeMap<Ident, ObjectUsed<'a>>) {
    if let Some(Id { name, .. }) = self.id.as_ref() {
      let mut used_objs = vec![];
      if let Some(FromStateField { from_token, expr, .. }) = &self.from {
        if let Some(p) = expr.expr_used.used_part(Some(from_token), false) {
          used_objs.push(p);
        }
        if let Some(p) = expr.target_used.used_part(Some(from_token), false) {
          used_objs.push(p);
        }
      }

      let TransitionField { transition_token, value, .. } = &self.transition;
      match value {
        AnimateTransitionValue::Transition(t) => {
          t.analyze_observe_depends(depends);
          used_objs.extend(t.fields.iter().filter_map(|f| f.used_part()));
        }
        AnimateTransitionValue::Expr(expr) => {
          if let Some(o) = expr
            .used_name_info
            .used_part(Some(&transition_token), false)
          {
            used_objs.push(o);
          }
        }
      };

      if !used_objs.is_empty() {
        depends.insert(name.clone(), ObjectUsed(used_objs.into_boxed_slice()));
      }
    }
  }
}

impl Transition {
  pub fn analyze_observe_depends<'a>(&'a self, depends: &mut BTreeMap<Ident, ObjectUsed<'a>>) {
    if let Some(Id { name, .. }) = self.id.as_ref() {
      let used = ObjectUsed::from_iter(self.fields.iter().filter_map(|f| f.used_part()));
      if !used.is_empty() {
        depends.insert(name.clone(), used);
      }
    }
  }

  pub fn variable_name(&self) -> Ident {
    if let Some(Id { ref name, .. }) = self.id {
      name.clone()
    } else {
      ribir_variable("transition", self.span())
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
