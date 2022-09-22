use crate::{
  declare_derive::declare_field_name,
  widget_attr_macro::{
    ctx_ident, kw, on_change::OnChangeDo, widget_macro::TrackExpr, DeclareCtx, ScopeUsedInfo,
    UsedType,
  },
};
use proc_macro2::TokenStream;
use quote::{quote_spanned, ToTokens};
use syn::{
  parse_quote_spanned,
  spanned::Spanned,
  token::{Brace, Colon, Semi},
  Ident, Path,
};

use super::DeclareField;

pub struct WidgetGen<'a, F> {
  ty: &'a Path,
  name: &'a Ident,
  fields: F,
  force_stateful: bool,
}

impl<'a, F: Iterator<Item = &'a DeclareField> + Clone> WidgetGen<'a, F> {
  pub fn new(ty: &'a Path, name: &'a Ident, fields: F, force_stateful: bool) -> Self {
    Self { ty, name, fields, force_stateful }
  }

  pub fn gen_widget_tokens(self, ctx: &DeclareCtx) -> TokenStream {
    let Self { fields, ty, name, .. } = &self;
    let used_info = self.whole_used_info();
    let span = ty.span();
    let mut tokens = quote_spanned! { span => let #name = };
    used_info.value_expr_surround_refs(&mut tokens, span, |tokens| {
      tokens.extend(quote_spanned! { span => <#ty as Declare>::builder() });
      self.fields.clone().for_each(|f| {
        let DeclareField { member, expr, .. } = f;
        tokens.extend(quote_spanned! {expr.span() => .#member(#expr)})
      });
      let build_ctx = ctx_ident(self.ty.span());
      tokens.extend(quote_spanned! { span => .build(#build_ctx) });
      if self.is_stateful(ctx) {
        tokens.extend(quote_spanned! { span => .into_stateful() });
      }
    });

    Semi(span).to_tokens(&mut tokens);

    for f in fields.clone() {
      let DeclareField { skip_nc, member, expr, .. } = f;
      if f.expr.upstream_tokens().is_some() {
        let expr_span = expr.span();
        let declare_set = declare_field_name(member);
        let mut used_name_info = ScopeUsedInfo::default();
        used_name_info.add_used((*name).clone(), UsedType::MOVE_CAPTURE);
        let on_change_flow = OnChangeDo {
          on_token: kw::on(expr_span),
          observe: expr.clone(),
          brace: Brace(expr_span),
          skip_nc: skip_nc.clone(),
          change_token: kw::change(expr_span),
          colon_token: Colon(expr_span),
          subscribe_do: TrackExpr {
            expr: parse_quote_spanned! { member.span() =>
              move |(_, after)| {
                 #name.state_ref().#declare_set(after)
              }
            },
            used_name_info,
          },
        };
        on_change_flow.to_tokens(&mut tokens)
      }
    }
    tokens
  }

  pub(crate) fn is_stateful(&self, ctx: &DeclareCtx) -> bool {
    self.force_stateful
    // widget is followed by others.
    || ctx.is_used(self.name)
    // or its fields follow others
    ||  self.used_other_objs()
  }

  fn used_other_objs(&self) -> bool {
    self
      .fields
      .clone()
      .any(move |f| f.expr.used_name_info.directly_used_widgets().is_some())
  }

  fn whole_used_info(&self) -> ScopeUsedInfo {
    self
      .fields
      .clone()
      .fold(ScopeUsedInfo::default(), |mut acc, f| {
        acc.merge(&f.expr.used_name_info);
        acc
      })
  }
}
