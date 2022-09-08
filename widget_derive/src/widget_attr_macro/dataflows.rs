use super::{
  declare_widget::{try_parse_skip_nc, upstream_tokens, SkipNcAttr},
  expr_refs_wrap, kw, skip_nc_assign, DeclareCtx, ObjectUsed, ScopeUsedInfo, UsedPart,
};
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use std::collections::{BTreeMap, HashSet};
use syn::{
  braced,
  parse::{Parse, ParseStream},
  punctuated::Punctuated,
  token,
  visit_mut::VisitMut,
  Expr, Ident,
};

use crate::{error::DeclareError, widget_attr_macro::capture_widget};

mod ct {
  syn::custom_punctuation!(RightArrow, ~>);
}

pub struct Dataflows {
  _dataflows_token: kw::dataflows,
  _brace_token: token::Brace,
  flows: Punctuated<Dataflow, token::Comma>,
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
      _brace_token: braced!(content in input),
      flows: Punctuated::parse_terminated(&content)?,
    })
  }
}

impl ToTokens for Dataflows {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    self.flows.iter().for_each(|t| t.to_tokens(tokens));
  }
}

impl ToTokens for Dataflow {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    let Self { from, to, .. } = self;
    let directly_used = match from.used_name_info.directly_used_widgets() {
      None => {
        DeclareError::DataFlowNoDepends(syn::spanned::Spanned::span(&from.expr).unwrap())
          .error_emit();
        return;
      }
      Some(d) => d,
    };

    let upstream = upstream_tokens(directly_used, quote!(change_stream));
    let from_used_name = &from.used_name_info;
    let to_used_name = &to.used_name_info;
    let state_refs: HashSet<&Ident, ahash::RandomState> = from_used_name
      .refs_widgets()
      .into_iter()
      .chain(to_used_name.refs_widgets().into_iter())
      .flatten()
      .collect();
    let mut subscribe_do = skip_nc_assign(self.skip_nc.is_some(), &to.expr, &from.expr);
    subscribe_do = expr_refs_wrap(state_refs.iter().cloned(), subscribe_do);

    let captures: HashSet<&Ident, ahash::RandomState> = from_used_name
      .all_widgets()
      .into_iter()
      .chain(to_used_name.all_widgets())
      .flatten()
      .collect();
    let capture_tokens = captures.into_iter().into_iter().map(capture_widget);

    tokens.extend(quote! {{
      #(#capture_tokens)*
      #upstream.subscribe(move |_| #subscribe_do );
    }});
  }
}

#[derive(Debug)]
pub struct DataFlowExpr {
  expr: Expr,
  used_name_info: ScopeUsedInfo,
}

impl Dataflows {
  pub fn analyze_data_flow_follows<'a>(&'a self, follows: &mut BTreeMap<Ident, ObjectUsed<'a>>) {
    self.flows.iter().for_each(|df| {
      if let Some(widgets) = df.to.used_name_info.all_widgets() {
        let part = df.as_depend_part();
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
  pub fn as_depend_part(&self) -> UsedPart {
    self
      .from
      .used_name_info
      .used_part(None, self.skip_nc.is_some())
      .expect("data flow must depends on some widget")
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
