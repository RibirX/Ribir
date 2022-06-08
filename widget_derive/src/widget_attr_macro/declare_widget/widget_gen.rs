use crate::{
  declare_derive::{field_convert_method, field_default_method},
  widget_attr_macro::{
    capture_widget, ribir_variable, skip_nc_assign,
    widget_macro::{is_expr_keyword, EXPR_FIELD},
    widget_state_ref, DeclareCtx, BUILD_CTX,
  },
};
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::{spanned::Spanned, Ident, Path};

use super::{upstream_by_used_widgets, DeclareField};

pub struct WidgetGen<'a> {
  pub ty: &'a Path,
  pub name: Ident,
  pub fields: &'a [DeclareField],
}

impl<'a> WidgetGen<'a> {
  pub fn gen_widget_tokens(&self, ctx: &DeclareCtx) -> TokenStream {
    if is_expr_keyword(&self.ty) {
      self.expr_widget_token()
    } else {
      self.normal_widget_token(ctx)
    }
  }

  fn normal_widget_token(&self, ctx: &DeclareCtx) -> TokenStream {
    let Self { fields, ty, name, .. } = self;

    let stateful = self.is_stateful(ctx).then(|| quote! { .into_stateful()});

    let build_ctx = ribir_variable(BUILD_CTX, self.ty.span());
    let fields_tokens = self.fields.iter().map(|f| f.field_tokens(ty));
    let build_widget = quote_spanned! { ty.span() =>
      let #name = <#ty as Declare>::builder()#(#fields_tokens)*.build(#build_ctx)#stateful;
    };
    let fields_follow = fields.iter().filter_map(|f| self.field_follow_tokens(f));

    quote! {

      #build_widget
      #(#fields_follow)*
    }
  }

  fn expr_widget_token(&self) -> TokenStream {
    let Self { ty, name, fields } = self;
    let expr_field = fields.last().unwrap();
    assert_eq!(expr_field.member, EXPR_FIELD);

    let DeclareField {
      member: expr_mem,
      expr,
      used_name_info,
      ..
    } = expr_field;
    let build_ctx = ribir_variable(BUILD_CTX, ty.span());
    if let Some(follows) = used_name_info.used_names.as_ref() {
      let follow_names = follows.iter().map(|f| &f.widget);
      let refs = follow_names.clone().map(widget_state_ref);
      let upstream = upstream_by_used_widgets(follow_names);
      let captures = used_name_info.use_or_capture_name().map(capture_widget);
      let field_converter = field_convert_method(expr_mem);
      quote_spanned! { ty.span() =>
        let #name = #ty::<_>::builder()
          .upstream(Some(#upstream.box_it()))
          .#expr_mem({
            #(#captures)*
            <ExprWidget::<_> as Declare>::Builder::#field_converter(
              move |cb: &mut dyn FnMut(Widget)| {
                #(#refs)*
                ChildConsumer::<_>::consume(#expr, cb)
              }
            )
          })
          .build(#build_ctx);
      }
    } else {
      quote_spanned! { ty.span() => let #name = #expr; }
    }
  }

  fn field_follow_tokens(&self, f: &DeclareField) -> Option<TokenStream> {
    let DeclareField { member, used_name_info, skip_nc, .. } = f;

    let name = &self.name;
    let expr_tokens = f.value_tokens(self.ty);

    used_name_info.used_names.is_some().then(|| {
      let assign = skip_nc_assign(skip_nc.is_some(), &quote! { #name.#member}, &expr_tokens);

      let upstream = upstream_by_used_widgets(used_name_info.used_widgets());
      let capture_widgets = used_name_info
        .used_widgets()
        .chain(std::iter::once(name))
        .map(capture_widget);

      quote_spanned! { f.span() => {
        #(#capture_widgets)*
        #upstream.subscribe(move |_| {
          let mut #name = #name.state_ref();
          #assign
        });
      }}
    })
  }

  fn is_stateful(&self, ctx: &DeclareCtx) -> bool {
    // widget is followed by others.
    ctx.is_used(&self.name)
      // or its fields follow others
      ||  self
      .fields
      .iter()
      .any(|f| f.used_name_info.used_names.is_some())
  }
}

impl DeclareField {
  fn value_tokens(&self, widget_ty: &Path) -> TokenStream {
    let Self { member, expr, .. } = self;
    let field_converter = field_convert_method(member);
    let span = expr.span();
    let mut expr =
      quote_spanned! { span => <#widget_ty as Declare>::Builder::#field_converter(#expr) };

    if self.used_name_info.used_names.is_some() {
      let refs = self.used_name_info.used_widgets().map(widget_state_ref);
      expr = quote_spanned! { span => #(#refs)* #expr };
    }

    if let Some(if_guard) = self.if_guard.as_ref() {
      let default_method = field_default_method(member);
      let build_ctx = ribir_variable(BUILD_CTX, if_guard.span());
      quote_spanned!(span => #if_guard {
        #expr
      } else {
        <#widget_ty as Declare>::Builder::#default_method(#build_ctx)
      })
    } else if self.used_name_info.used_names.is_some() {
      quote_spanned! { span => { #expr }}
    } else {
      expr
    }
  }

  pub(crate) fn field_tokens(&self, widget_ty: &Path) -> TokenStream {
    let member = &self.member;
    let value = self.value_tokens(widget_ty);
    quote! {.#member(#value)}
  }
}
