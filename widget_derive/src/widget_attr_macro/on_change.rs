use super::{
  declare_widget::{try_parse_skip_nc, SkipNcAttr},
  kw,
  widget_macro::TrackExpr,
  DeclareCtx, ObjectUsed, UsedPart,
};
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned, ToTokens};
use std::collections::BTreeMap;
use syn::{
  braced,
  parse::{Parse, ParseStream},
  parse_quote_spanned,
  spanned::Spanned,
  token::{self, Brace, Colon, Paren, Semi},
  Ident,
};

use crate::{error::DeclareWarning, widget_attr_macro::capture_widget};

#[derive(Debug)]
pub struct OnChangeDo {
  pub on_token: kw::on,
  pub observe: TrackExpr,
  pub brace: Brace,
  pub skip_nc: Option<SkipNcAttr>,
  pub change_token: kw::change,
  pub colon_token: token::Colon,
  pub subscribe_do: TrackExpr,
}

/// change flow is just a syntax sugar of `OnChangeDo`
/// `a.size ~> b.size` is sugar of
/// `on a.size { change: move |_, after| b.size = after}`
#[derive(Debug)]
pub struct ChangeFlow {
  pub skip_nc: Option<SkipNcAttr>,
  pub on_token: kw::on,
  pub from: TrackExpr,
  pub flow_arrow: kw::FlowArrow,
  pub to: TrackExpr,
}

impl Parse for ChangeFlow {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    Ok(Self {
      skip_nc: try_parse_skip_nc(input)?,
      on_token: input.parse()?,
      from: input.parse()?,
      flow_arrow: input.parse()?,
      to: input.parse()?,
    })
  }
}

impl Parse for OnChangeDo {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    let content;
    Ok(Self {
      on_token: input.parse()?,
      observe: input.parse()?,
      brace: braced! { content in input },
      skip_nc: try_parse_skip_nc(&content)?,
      change_token: content.parse()?,
      colon_token: content.parse()?,
      subscribe_do: content.parse()?,
    })
  }
}

impl ChangeFlow {
  pub fn into_change_do(self) -> OnChangeDo {
    let Self {
      skip_nc,
      on_token,
      from,
      flow_arrow,
      to,
    } = self;
    // flow change is a sugar of `OnChangeDo`
    OnChangeDo {
      on_token,
      observe: from,
      brace: Brace(to.span()),
      skip_nc,
      change_token: kw::change(flow_arrow.span()),
      colon_token: Colon(flow_arrow.span()),
      subscribe_do: TrackExpr {
        expr: parse_quote_spanned! { to.span() => move |(_, after)| #to = after },
        used_name_info: to.used_name_info,
      },
    }
  }
}

impl ToTokens for OnChangeDo {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    let Self {
      observe,
      brace,
      skip_nc,
      subscribe_do,
      ..
    } = self;

    if let Some(upstream) = observe.upstream_tokens() {
      let observe_span = observe.span();
      upstream.to_tokens(tokens);
      let mut expr_value = quote! {};
      observe
        .used_name_info
        .value_expr_surround_refs(&mut expr_value, observe_span, |tokens| {
          observe.to_tokens(tokens)
        });

      let captures = observe
        .used_name_info
        .all_widgets()
        // if upstream is not none, must used some widget.
        .unwrap()
        .map(capture_widget);

      tokens.extend(quote_spanned! { observe.span() =>
        .filter(|s| s.contains(ChangeScope::DATA))
        .scan_initial({
            let v = #expr_value;
            (v.clone(), v)
          }, {
            #(#captures)*
            move |(_, after), _| { (after, #expr_value)}
        })
      });

      if skip_nc.is_some() {
        tokens.extend(quote_spanned! { skip_nc.span() =>
          .filter(move |(before, after)| before != after)
        });
      }
      quote_spanned! {brace.span => .subscribe}.to_tokens(tokens);
      Paren(brace.span).surround(tokens, |tokens| {
        let mut subscribe_tokens = quote! {};
        subscribe_do
          .used_name_info
          .refs_surround(&mut subscribe_tokens, |tokens| {
            subscribe_do.to_tokens(tokens);
          });
        if let Some(all) = subscribe_do.used_name_info.all_widgets() {
          Brace(brace.span).surround(tokens, |tokens| {
            // we convert a `expression` into move closure.
            for c in all {
              capture_widget(c).to_tokens(tokens);
            }
            subscribe_tokens.to_tokens(tokens);
          });
        } else {
          subscribe_tokens.to_tokens(tokens);
        }
      });
      Semi(brace.span).to_tokens(tokens);
    }
  }
}

impl OnChangeDo {
  pub fn warning(&self) -> Option<DeclareWarning> {
    let expr = &self.observe;
    expr
      .used_name_info
      .directly_used_widgets()
      .is_none()
      .then(|| DeclareWarning::ObserveIsConst(expr.span().unwrap()))
  }
}

impl OnChangeDo {
  pub fn analyze_observe_depends<'a>(&'a self, follows: &mut BTreeMap<Ident, ObjectUsed<'a>>) {
    if let Some(widgets) = self.subscribe_do.used_name_info.all_widgets() {
      if let Some(part) = self.as_depend_part() {
        widgets.for_each(|name| {
          if let Some(w_follows) = follows.get_mut(name) {
            *w_follows = w_follows
              .iter()
              .cloned()
              .chain(std::iter::once(part.clone()))
              .collect();
          } else {
            follows.insert(name.clone(), ObjectUsed::from_single_part(part.clone()));
          }
        })
      }
    }
  }

  pub fn as_depend_part(&self) -> Option<UsedPart> {
    self
      .observe
      .used_name_info
      .used_part(None, self.skip_nc.is_some())
  }
}

impl DeclareCtx {
  pub fn visit_on_change_do(&mut self, on_change_do: &mut OnChangeDo) {
    self.visit_track_expr(&mut on_change_do.observe);
    self.visit_track_expr(&mut on_change_do.subscribe_do);
  }
}
