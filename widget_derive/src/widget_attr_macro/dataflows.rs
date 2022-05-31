use super::{
  declare_widget::{try_parse_skip_nc, upstream_by_used_widgets, SkipNcAttr},
  kw, skip_nc_assign,
  widget_macro::UsedNameInfo,
  DeclareCtx, DependIn, DependPart, Depends, MergeDepends,
};
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use std::collections::BTreeMap;
use syn::{
  braced,
  parse::{Parse, ParseStream},
  punctuated::Punctuated,
  token,
  visit_mut::VisitMut,
  Expr, Ident,
};

use crate::{
  error::DeclareError,
  widget_attr_macro::{capture_widget, widget_state_ref},
};

mod ct {
  syn::custom_punctuation!(RightArrow, ~>);
}

pub struct Dataflows {
  _dataflows_token: kw::dataflows,
  brace_token: token::Brace,
  // todo: use ',' replace ';'?
  flows: Punctuated<Dataflow, token::Semi>,
}

#[derive(Debug)]
pub struct Dataflow {
  pub skip_nc: Option<SkipNcAttr>,
  from: DataFlowExpr,
  _arrow_token: ct::RightArrow,
  to: DataFlowExpr,
}

impl Parse for Dataflows {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    let content;
    Ok(Self {
      _dataflows_token: input.parse()?,
      brace_token: braced!(content in input),
      flows: Punctuated::parse_terminated(&content)?,
    })
  }
}

impl ToTokens for Dataflows {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    self.brace_token.surround(tokens, |tokens| {
      self.flows.iter().for_each(|t| t.to_tokens(tokens));
    });
  }
}

impl ToTokens for Dataflow {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    let Self { from, to, .. } = self;
    if from.used_name_info.used_names.is_none() {
      DeclareError::DataFlowNoDepends(syn::spanned::Spanned::span(&from.expr).unwrap()).error_emit()
    }

    let upstream = upstream_by_used_widgets(from.used_name_info.used_widgets());
    let from_used_name = &from.used_name_info;
    let to_used_name = &to.used_name_info;
    let state_refs = from_used_name
      .used_names
      .iter()
      .chain(to_used_name.used_names.iter())
      .unique_widget()
      .into_iter()
      .flat_map(|widgets| widgets.into_iter().map(widget_state_ref));

    let captures = from_used_name
      .used_names
      .iter()
      .chain(from_used_name.captures.iter())
      .chain(to_used_name.used_names.iter())
      .chain(to_used_name.captures.iter())
      .unique_widget()
      .into_iter()
      .flat_map(|widgets| widgets.into_iter().map(capture_widget));

    let subscribe_do = skip_nc_assign(self.skip_nc.is_some(), &to.expr, &from.expr);
    tokens.extend(quote! {
      #(#captures)*
      #upstream.subscribe(move |_| { #(#state_refs)* #subscribe_do });
    });
  }
}

#[derive(Debug)]
pub struct DataFlowExpr {
  expr: Expr,
  used_name_info: UsedNameInfo,
}

impl Dataflows {
  pub fn analyze_data_flow_follows<'a>(&'a self, follows: &mut BTreeMap<Ident, Depends<'a>>) {
    self.flows.iter().for_each(|df| {
      if let Some(to) = df.to.used_name_info.used_names.as_ref() {
        let part = df.as_depend_part();
        to.iter().for_each(|fo| {
          let name = &fo.widget;
          if let Some(w_follows) = follows.get_mut(name) {
            *w_follows = w_follows
              .iter()
              .cloned()
              .chain(Some(part.clone()).into_iter())
              .collect();
          } else {
            follows.insert(name.clone(), Depends::from_single_part(part.clone()));
          }
        })
      }
    });
  }
}

impl Parse for Dataflow {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    Ok(Self {
      skip_nc: try_parse_skip_nc(input)?,
      from: DataFlowExpr {
        expr: input.parse()?,
        used_name_info: <_>::default(),
      },
      _arrow_token: input.parse()?,
      to: DataFlowExpr {
        expr: input.parse()?,
        used_name_info: <_>::default(),
      },
    })
  }
}

impl Dataflow {
  pub fn as_depend_part(&self) -> DependPart {
    let place_info = self
      .from
      .used_name_info
      .used_names
      .as_ref()
      .expect("data flow must depends on some widget");

    DependPart {
      origin: DependIn::DataFlow(self),
      place_info,
    }
  }
}
impl DeclareCtx {
  pub fn visit_dataflows_mut(&mut self, dfs: &mut Dataflows) {
    dfs
      .flows
      .iter_mut()
      .for_each(|df| self.visit_dataflow_mut(df))
  }

  fn visit_dataflow_mut(&mut self, df: &mut Dataflow) {
    self.visit_expr_mut(&mut df.from.expr);
    df.from.used_name_info = self.take_current_used_info();
    self.visit_expr_mut(&mut df.to.expr);
    df.to.used_name_info = self.take_current_used_info();
  }
}
