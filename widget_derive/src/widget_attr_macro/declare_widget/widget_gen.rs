use crate::{
  declare_derive::declare_field_name,
  widget_attr_macro::{
    capture_widget, ctx_ident, ribir_suffix_variable, ribir_variable, DeclareCtx, ScopeUsedInfo,
    UsedType,
  },
};
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned, ToTokens};
use syn::{spanned::Spanned, Ident, Path};

use super::{upstream_tokens, DeclareField, WidgetExtend};

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
    let stateful = (self.is_stateful(ctx)).then(|| quote! { .into_stateful()});
    self.gen_widget(self.name, stateful)
  }

  pub fn gen_extended_tokens(&self, ctx: &DeclareCtx, extends: &WidgetExtend) -> TokenStream {
    let used_widgets = extends.expr_used.directly_used_widgets();
    let is_stateful = self.is_stateful(ctx) || used_widgets.map_or(false, |_| true);
    if is_stateful {
      self.gen_stateful_extended(&extends)
    } else {
      self.gen_stateless_extended(&extends)
    }
  }

  fn gen_stateless_extended(&self, extends: &WidgetExtend) -> TokenStream {
    self.gen_widget(self.name, Some(extends.tokens()))
  }

  fn gen_stateful_extended(&self, extends: &WidgetExtend) -> TokenStream {
    let Self { ty, name, .. } = self;
    let inner_name = ribir_suffix_variable(name, "inner");
    let inner_widget = self.gen_widget(&inner_name, Some(quote! { .into_stateful()}));
    let extend_tokens = extends.tokens();
    let declare_widget = {
      let tokens = extends.expr_used.expr_refs_wrap(quote_spanned! {
        ty.span() => #inner_name.raw_ref().clone()#extend_tokens.into_stateful()
      });
      quote_spanned! { ty.span() => let #name = #tokens; }
    };

    let rebuild_tokens = {
      let extend_tokens = extend_tokens.clone();
      extends.expr_used.expr_refs_wrap(
        quote! { *(#name.state_ref()) = #inner_name.state_ref().clone()#extend_tokens; },
      )
    };
    let used_widgets = extends
      .expr_used
      .directly_used_widgets()
      .into_iter()
      .flatten()
      .chain(std::iter::once(&inner_name));

    let capture_widgets = used_widgets
      .clone()
      .chain(std::iter::once(*name))
      .map(capture_widget);

    let follow = used_widgets.clone().map(move |widget| {
      let upstream = upstream_tokens(std::iter::once(widget), quote! {clone().change_stream});
      let rebuild_tokens = rebuild_tokens.clone();
      let capture_widgets = capture_widgets.clone();
      quote_spanned! { ty.span() => {
          {
            #(#capture_widgets)*
            #upstream.subscribe(move |_| #rebuild_tokens );
          }
        }
      }
    });

    quote! {
      #inner_widget
      #declare_widget
      #(#follow)*
    }
  }

  fn gen_widget(&self, name: &Ident, extends: Option<TokenStream>) -> TokenStream {
    let Self { fields, ty, .. } = self;

    let build_ctx = ctx_ident(self.ty.span());
    let fields_tokens = self.fields.clone().map(|f| f.field_tokens());
    let mut build_widget = quote! {
      <#ty as Declare>::builder()#(#fields_tokens)*.build(#build_ctx)#extends
    };
    let used_info = self.whole_used_info();
    build_widget = used_info.expr_refs_wrap(build_widget);

    build_widget = quote_spanned! { ty.span() => let #name = #build_widget; };
    let fields_follow = fields.clone().filter_map(|f| self.field_follow_tokens(f));

    quote! {
      #build_widget
      #(#fields_follow)*
    }
  }

  fn field_follow_tokens(&self, f: &DeclareField) -> Option<TokenStream> {
    let DeclareField { member, used_name_info, skip_nc, .. } = f;

    let name = &self.name;
    let expr_tokens = f.used_name_info.expr_refs_wrap(f.value_tokens());
    let directly_used = used_name_info.directly_used_widgets()?;

    if f.value_is_an_id().is_some() {
      return None;
    }

    let declare_set = declare_field_name(member);
    let assign = if skip_nc.is_some() {
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
      quote_spanned! { name.span() => #name.clone_stateful() }
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
