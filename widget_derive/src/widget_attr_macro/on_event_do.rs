use std::collections::{BTreeMap, HashMap};

use crate::error::{DeclareError, DeclareWarning};

use super::{
  declare_widget::{
    builtin_var_name, check_duplicate_field, is_listener, BuiltinWidgetInfo, DeclareField,
    WidgetGen, FIELD_WIDGET_TYPE,
  },
  kw,
  on_change::OnChangeDo,
  DeclareCtx, IdType, ObjectUsed, UsedType, CHANGE,
};

use quote::{quote, ToTokens};
use syn::{
  braced,
  parse::Parse,
  parse_quote_spanned,
  punctuated::Punctuated,
  spanned::Spanned,
  token::{Brace, Comma},
  Ident,
};

#[derive(Debug)]
pub struct OnEventDo {
  pub on_token: kw::on,
  pub target: Ident,
  pub brace: Brace,
  pub listeners: HashMap<&'static str, BuiltinWidgetInfo, ahash::RandomState>,
  // change have itself logic to gen code.
  pub on_change_do: Option<OnChangeDo>,
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
  pub fn visit_on_event_do(&mut self, on_event_do: &mut OnEventDo) {
    let OnEventDo { target, listeners: events, .. } = on_event_do;
    for (ty, builtin_widget) in events {
      let builtin_name = builtin_var_name(target, ty);
      // widget be used by on target.
      self.add_used_widget(builtin_name, UsedType::USED);
      self.take_current_used_info();

      self.visit_builtin_widget_info_mut(builtin_widget);
    }
  }
}
