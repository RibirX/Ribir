use crate::{
  declare_derive::field_convert_method,
  widget_attr_macro::{
    field_guard_variable, ribir_variable, skip_nc_assign, widget_def_variable, DeclareCtx,
    BUILD_CTX,
  },
};
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::{spanned::Spanned, Ident};

use super::{upstream_observable, DeclareField};

pub struct WidgetGen<'a> {
  pub ty: &'a syn::Type,
  pub name: Ident,
  pub fields: &'a [DeclareField],
}

impl<'a> WidgetGen<'a> {
  pub fn gen_widget_tokens(&self, ctx: &DeclareCtx) -> TokenStream {
    let Self { fields, ty, .. } = self;

    let stateful = self.is_stateful(ctx).then(|| quote! { .into_stateful()});
    let def_name = widget_def_variable(&self.name);
    let ref_name = &self.name;

    let (fields_without_guard, fields_with_guard): (Vec<_>, Vec<_>) =
      fields.iter().partition(|f| f.if_guard.is_none());

    let guard_calc = fields_with_guard.iter().map(|f| {
      let guard = f.if_guard.as_ref().unwrap();
      let guard_cond = field_guard_variable(&f.member, guard.span());
      quote! { let #guard_cond = #guard { true } else { false }; }
    });

    let build_ctx = ribir_variable(BUILD_CTX, self.ty.span());
    let build_widget = {
      let mut_token = (!fields_with_guard.is_empty()).then(|| quote! {mut});
      let without_guard_tokens = fields_without_guard
        .iter()
        .map(|f| f.build_tokens_without_guard());

      if fields_with_guard.is_empty() {
        quote_spanned! { ty.span() =>
          let #mut_token #def_name = <#ty as Declare>::builder()
            #(#without_guard_tokens)*.build(#build_ctx).into_widget()#stateful;
        }
      } else {
        let with_guard_tokens = fields_with_guard
          .iter()
          .map(|f| f.build_tokens_with_guard(&def_name));

        quote_spanned! { ty.span() =>
          let #mut_token #def_name = <#ty as Declare>::builder()#(#without_guard_tokens)*;
          #(#with_guard_tokens)*
          let #def_name = #def_name.build(#build_ctx).into_widget()#stateful;
        }
      }
    };

    let fields_follow = fields.iter().filter_map(|f| self.field_follow_tokens(f));

    let state_ref = if self.is_stateful(ctx) {
      Some(quote! { let mut #ref_name = unsafe { #def_name.state_ref() }; })
    } else if ctx.be_reference(ref_name) {
      Some(quote! { let #ref_name = &#def_name; })
    } else {
      None
    };

    quote! {
      #(#guard_calc)*
      #build_widget
      #state_ref
      #(#fields_follow)*
    }
  }

  fn field_follow_tokens(&self, f: &DeclareField) -> Option<TokenStream> {
    let DeclareField {
      member, follows, skip_nc, if_guard, ..
    } = f;

    let ref_name = &self.name;
    let expr_tokens = f.value_tokens(self.ty);

    follows.as_ref().map(|follows| {
      let assign = skip_nc_assign(
        skip_nc.is_some(),
        &quote! { #ref_name.#member},
        &expr_tokens,
      );
      let upstream = upstream_observable(follows);
      let mut tokens = quote! { #upstream.subscribe( move |_|{ #assign } );};
      if let Some(if_guard) = if_guard {
        let guard_cond = field_guard_variable(member, if_guard.span());
        tokens = quote! { if #guard_cond { #tokens } }
      }
      tokens
    })
  }

  fn is_stateful(&self, ctx: &DeclareCtx) -> bool {
    // widget is followed by others.
    ctx.be_followed(&self.name)
      // or its fields follow others
      ||  self
      .fields
      .iter()
      .any(|f| f.follows.is_some())
  }
}

impl DeclareField {
  fn value_tokens(&self, widget_ty: &syn::Type) -> TokenStream {
    let Self { member, expr, .. } = self;
    let field_converter = field_convert_method(member);
    quote_spanned! { expr.span() => <#widget_ty as Declare>::Builder::#field_converter(#expr) }
  }

  fn build_tokens_without_guard(&self) -> TokenStream {
    assert!(self.if_guard.is_none());
    let Self { member, expr, .. } = self;
    quote! {.#member(#expr)}
  }

  fn build_tokens_with_guard(&self, builder: &Ident) -> TokenStream {
    let if_guard = self.if_guard.as_ref().unwrap();
    let Self { member, expr, .. } = self;
    let guard_cond = field_guard_variable(member, if_guard.span());
    quote! {
      if #guard_cond {
        #builder = #builder.#member(#expr);
      }
    }
  }
}