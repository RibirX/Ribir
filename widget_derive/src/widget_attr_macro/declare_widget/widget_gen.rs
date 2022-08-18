use crate::{
  declare_derive::declare_field_name,
  widget_attr_macro::{
    capture_widget, ribir_variable,
    widget_macro::{is_const_expr_keyword, EXPR_FIELD},
    widget_state_ref, DeclareCtx, ScopeUsedInfo, UsedType, BUILD_CTX,
  },
};
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned, ToTokens};
use syn::{spanned::Spanned, Ident, Path};

use super::{upstream_tokens, DeclareField};

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

  pub fn gen_widget_tokens(&self, ctx: &DeclareCtx) -> TokenStream {
    if is_const_expr_keyword(self.ty) {
      self.const_expr_widget_tokens()
    } else {
      self.normal_widget_token(ctx)
    }
  }

  fn normal_widget_token(&self, ctx: &DeclareCtx) -> TokenStream {
    let Self { fields, ty, name, .. } = self;

    let stateful = self.is_stateful(ctx).then(|| quote! { .into_stateful()});

    let build_ctx = ribir_variable(BUILD_CTX, self.ty.span());
    let fields_tokens = self.fields.clone().map(|f| f.field_tokens());
    let mut build_widget = quote! {
      <#ty as Declare>::builder()#(#fields_tokens)*.build(#build_ctx)#stateful
    };
    let used_info = self.whole_used_info();
    if let Some(refs) = used_info.directly_used_widgets() {
      let refs = refs.map(widget_state_ref);
      build_widget = quote_spanned! { ty.span() =>
        let #name = {
          #(#refs)*
          #build_widget
        };
      };
    } else {
      build_widget = quote_spanned! { ty.span() => let #name = #build_widget; };
    }
    let fields_follow = fields.clone().filter_map(|f| self.field_follow_tokens(f));

    quote! {
      #build_widget
      #(#fields_follow)*
    }
  }

  fn const_expr_widget_tokens(&self) -> TokenStream {
    let Self { ty, name, fields, .. } = self;
    let expr_field = fields.clone().last().unwrap();
    assert_eq!(expr_field.member, EXPR_FIELD);

    let value_tokens = expr_field.value_tokens();
    quote_spanned! { ty.span() => let #name = #value_tokens; }
  }

  fn field_follow_tokens(&self, f: &DeclareField) -> Option<TokenStream> {
    let DeclareField { member, used_name_info, skip_nc, .. } = f;

    let name = &self.name;
    let expr_tokens = f.value_tokens();
    let directly_used = used_name_info.directly_used_widgets()?;

    if f.value_is_an_id().is_some() {
      return None;
    }

    let declare_set = declare_field_name(member);
    let mut assign = if skip_nc.is_some() {
      let old = ribir_variable("old", expr_tokens.span());
      quote! {{
         let diff = {
          let mut #name = #name.raw_ref();
          let #old = #name.#member.clone();
          #name.#declare_set(#expr_tokens);
          #name.#member != #old
        };
        if diff {
          // if value really changed, trigger state change
          #name.state_ref();
        }
      }}
    } else {
      quote! { #name.state_ref().#declare_set(#expr_tokens) }
    };
    if let Some(refs) = f.used_name_info.refs_tokens() {
      assign = quote! {{ #(#refs)* #assign }};
    }

    let upstream = upstream_tokens(directly_used, quote! {change_stream});
    let capture_widgets = used_name_info
      .all_widgets()
      .into_iter()
      .flatten()
      .chain(std::iter::once(<&Ident>::clone(name)))
      .map(capture_widget);

    Some(quote_spanned! { f.span() => {
      #(#capture_widgets)*
      #upstream.subscribe(move |_| #assign );
    }})
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
      .any(move |f| f.used_name_info.directly_used_widgets().is_some())
  }

  fn whole_used_info(&self) -> ScopeUsedInfo {
    self
      .fields
      .clone()
      .fold(ScopeUsedInfo::default(), |mut acc, f| {
        acc.merge(&f.used_name_info);
        acc
      })
  }
}

impl DeclareField {
  fn value_tokens(&self) -> TokenStream {
    if let Some(name) = self.value_is_an_id() {
      quote_spanned! { name.span() => #name.clone() }
    } else {
      self.expr.to_token_stream()
    }
  }

  pub(crate) fn field_tokens(&self) -> TokenStream {
    let member = &self.member;
    let value = self.value_tokens();
    quote! {.#member(#value)}
  }

  fn value_is_an_id(&self) -> Option<&Ident> {
    if let syn::Expr::Path(path) = &self.expr {
      let name = path.path.get_ident()?;
      let used_info = self.used_name_info.get(name)?;
      assert_eq!(used_info.used_type, UsedType::USED);
      assert_eq!(self.used_name_info.len(), 1);
      Some(name)
    } else {
      None
    }
  }
}
