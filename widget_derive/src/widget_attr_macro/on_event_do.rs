use std::collections::{BTreeMap, HashMap};

use crate::error::{DeclareError, DeclareWarning};

use super::{
  animations::{Animate, MemberPath, Transition},
  declare_widget::{
    builtin_var_name, check_duplicate_field, is_listener, BuiltinWidgetInfo, DeclareField,
    SkipNcAttr, WidgetGen, FIELD_WIDGET_TYPE,
  },
  kw,
  on_change::OnChangeDo,
  ribir_suffix_variable, ribir_variable, DeclareCtx, IdType, ObjectUsed, UsedType, CHANGE,
};

use quote::{quote, quote_spanned, ToTokens};
use syn::{
  braced,
  parse::Parse,
  parse_quote, parse_quote_spanned,
  punctuated::Punctuated,
  spanned::Spanned,
  token::{Brace, Comma},
  Ident,
};

/// todo: The next step, we should support pipe for event as a rx stream
///
/// ```
///     on sized_box {
///         wheel pipe(xxx)
///         | press pipe(): move || {}
///     }
#[derive(Debug)]
pub struct OnEventDo {
  pub on_token: kw::on,
  pub target: Ident,
  pub brace: Brace,
  pub listeners: HashMap<&'static str, BuiltinWidgetInfo, ahash::RandomState>,
  // change have itself logic to gen code.
  pub on_change_do: Option<OnChangeDo>,
}

#[derive(Debug)]
pub struct OnTransitionSyntax {
  pub skip_nc: Option<SkipNcAttr>,
  pub on_token: kw::on,
  pub observe: syn::Expr,
  pub transition: Transition,
}

/// The syntax case `on id.member Animate {...}`,
#[derive(Debug)]
pub struct OnAnimateSyntax {
  pub skip_nc: Option<SkipNcAttr>,
  pub on_token: kw::on,
  pub observe: syn::Expr,
  pub animate: Animate,
}

#[derive(Debug)]
pub struct OnAnimate {
  env_variables: Option<InitVariables>,
  pub animate: Animate,
  pub on_change_do: OnChangeDo,
}

#[derive(Debug)]

struct InitVariables {
  name1: Ident,
  name2: Ident,
  value: MemberPath,
}

impl Parse for OnEventDo {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    let content;

    let on_token = input.parse()?;
    let target = input.parse()?;
    let brace = braced!(content in input);
    let fields: Punctuated<_, Comma> = content.parse_terminated(DeclareField::parse)?;
    check_duplicate_field(&fields)?;

    let mut listeners = HashMap::default();
    let mut on_change_do = None;
    for f in fields {
      if f.member == CHANGE {
        on_change_do = Some(parse_quote_spanned! { f.span() =>
          #on_token #target { #f }
        });
      } else {
        let ty = FIELD_WIDGET_TYPE
          .get(f.member.to_string().as_str())
          .ok_or_else(|| {
            syn::Error::new(
              f.member.span(),
              &format!(
                "`{}` is not allow use in `on` group, only listeners support.",
                f.member
              ),
            )
          })?;
        let info: &mut BuiltinWidgetInfo = listeners.entry(*ty).or_default();
        info.push(f);
      }
    }

    Ok(Self {
      on_token,
      target,
      brace,
      listeners,
      on_change_do,
    })
  }
}

impl OnEventDo {
  pub fn gen_tokens(&self, tokens: &mut proc_macro2::TokenStream, ctx: &mut DeclareCtx) {
    let OnEventDo { target, listeners, on_change_do, .. } = self;
    listeners.iter().for_each(|(ty, fields)| {
      let name = builtin_var_name(target, ty);
      let ty = Ident::new(ty, fields.0[0].span()).into();
      let gen = WidgetGen::new(&ty, &name, fields.0.iter(), false);

      if ctx
        .named_objects
        .get(&name)
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

    if let Some(on_change) = on_change_do {
      on_change.to_tokens(tokens)
    }
  }

  pub fn analyze_observe_depends<'a>(&'a self, depends: &mut BTreeMap<Ident, ObjectUsed<'a>>) {
    self.listeners.iter().for_each(|(ty, info)| {
      let used: ObjectUsed = info.0.iter().filter_map(|f| f.used_part()).collect();
      if !used.is_empty() {
        let name = builtin_var_name(&self.target, ty);
        depends.insert(name, used);
      }
    });
  }

  pub fn error_check(&self, ctx: &mut DeclareCtx) {
    if ctx.named_objects.get(&self.target) != Some(&IdType::DECLARE) {
      ctx
        .errors
        .push(DeclareError::OnInvalidTarget(self.target.clone()));
    }
    self
      .listeners
      .iter()
      .filter(|(ty_name, _)| !is_listener(ty_name))
      .for_each(|(_, info)| {
        info.0.iter().for_each(|f| {
          ctx
            .errors
            .push(DeclareError::OnInvalidField(f.member.clone()))
        })
      });
  }

  pub fn warning(&self) -> Option<DeclareWarning> {
    self.on_change_do.as_ref().and_then(|c| c.warning())
  }
}

impl DeclareCtx {
  pub fn visit_on_event_do_mut(&mut self, on_event_do: &mut OnEventDo) {
    let OnEventDo {
      target,
      listeners: events,
      on_change_do,
      ..
    } = on_event_do;
    for (ty, builtin_widget) in events {
      let builtin_name = builtin_var_name(target, ty);
      // widget be used by on target.
      self.add_used_widget(builtin_name, UsedType::USED);
      self.take_current_used_info();

      self.visit_builtin_widget_info_mut(builtin_widget);
    }
    if let Some(on_change_do) = on_change_do {
      self.visit_on_change_do_mut(on_change_do)
    }
  }
  pub fn visit_on_animate_mut(&mut self, on_animate: &mut OnAnimate) {
    let OnAnimate { env_variables, animate, on_change_do } = on_animate;
    if let Some(vars) = env_variables.as_mut() {
      self.visit_vars_mut(vars);
    }
    self.visit_animate_mut(animate);
    if animate.id.is_none() {
      // animate not declare an id but also captured be the `on_change_do`.
      let animate_name = animate.variable_name();
      self.add_named_obj(animate_name.clone(), IdType::DECLARE);
      self.visit_on_change_do_mut(on_change_do);
      self.named_objects.remove(&animate_name);
    } else {
      self.visit_on_change_do_mut(on_change_do);
    }
  }

  fn visit_vars_mut(&mut self, vars: &mut InitVariables) {
    self.visit_member_path_mut(&mut vars.value);
    self.take_current_used_info();
  }
}

impl OnAnimate {
  pub fn gen_tokens(&self, tokens: &mut proc_macro2::TokenStream, ctx: &mut DeclareCtx) {
    let Self { env_variables, animate, on_change_do } = self;
    if let Some(InitVariables { name1, name2, value }) = env_variables {
      let MemberPath { widget, dot_token, member } = value;
      tokens.extend(quote_spanned! { value.span() =>
        let #name1 = std::rc::Rc::new(std::cell::RefCell::new(
          #widget #dot_token raw_ref() #dot_token #member #dot_token clone()
        ));
        let #name2 = #name1.clone();
      });
    }
    if animate.id.is_none() {
      animate.gen_tokens(tokens, ctx);
    }
    on_change_do.to_tokens(tokens);
  }
}
impl OnAnimateSyntax {
  pub fn into_on_animate(self) -> OnAnimate {
    let span = self.span();
    let Self {
      skip_nc,
      on_token,
      observe,
      mut animate,
    } = self;
    let animate_name = animate.variable_name();
    if animate.from.is_some() {
      // todo: only subscribe change
      OnAnimate {
        env_variables: None,
        animate,
        on_change_do: parse_quote_spanned! { span =>
          #on_token #observe.clone() { #skip_nc change: move |_| #animate_name.run() }
        },
      }
    } else {
      let path: MemberPath = parse_quote! { #observe };
      let init = ribir_variable("init_state", path.member.span());
      let init_2 = ribir_suffix_variable(&init, "2");

      animate.from = Some(parse_quote_spanned! { path.span() =>
        from: State { #path: #init.borrow().clone()}
      });

      let on_change_do = parse_quote_spanned! { span =>
        on #observe.clone() { #skip_nc change: move |(before, _)| {
          *#init_2.borrow_mut() = before.clone();
          #animate_name.run()
        }}
      };
      OnAnimate {
        env_variables: Some(InitVariables {
          name1: init,
          name2: init_2,
          value: path,
        }),
        animate,
        on_change_do,
      }
    }
  }
}

impl Spanned for OnAnimateSyntax {
  fn span(&self) -> proc_macro2::Span {
    let Self { skip_nc, on_token, animate, .. } = self;
    if let Some(skip) = skip_nc {
      skip.span().join(animate.span()).unwrap()
    } else {
      on_token.span().join(animate.span()).unwrap()
    }
  }
}

impl OnTransitionSyntax {
  pub fn into_on_animate(self) -> OnAnimate {
    let Self {
      skip_nc,
      on_token,
      observe,
      transition,
    } = self;
    OnAnimateSyntax {
      on_token,
      skip_nc,
      observe,
      animate: parse_quote_spanned! { transition.span() =>
        Animate { transition: #transition }
      },
    }
    .into_on_animate()
  }
}
