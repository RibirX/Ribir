use crate::{
  declare_derive::field_convert_method,
  declare_func_derive::{skip_nc_assign, upstream_observable, widget_def_variable},
};
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::{spanned::Spanned, Ident, Path};

use super::{ribir_suffix_variable, DeclareCtx, DeclareField};

pub struct WidgetGen<'a> {
  pub ty: &'a Path,
  pub name: Ident,
  pub fields: &'a Vec<DeclareField>,
  pub ctx_name: &'a Ident,
}

impl<'a> WidgetGen<'a> {
  pub fn gen_widget_tokens(&self, ctx: &DeclareCtx, force_stateful: bool) -> TokenStream {
    let Self { fields, ty, .. } = self;

    let stateful = self.is_stateful(ctx).then(|| quote! { .into_stateful()});
    let def_name = widget_def_variable(&self.name);
    let ref_name = &self.name;

    let mut value_before = quote! {};
    let mut build_widget = quote! {
      let mut #def_name = <#ty as Declare>::builder();
    };
    let mut follow_after = quote! {};

    // todo: split fields by if it has `if-guard` and generate chain or not.
    fields.iter().for_each(|f| {
      self.gen_field_tokens(f, &mut value_before, &mut build_widget, &mut follow_after);
    });

    let ctx_name = self.ctx_name;
    build_widget.extend(quote! { let mut #def_name = #def_name.build(#ctx_name)#stateful;});

    let state_ref = if force_stateful || self.is_stateful(ctx) {
      Some(quote! { let mut #ref_name = unsafe { #def_name.state_ref() }; })
    } else if ctx.be_reference(&ref_name) {
      Some(quote! { let #ref_name = &mut #def_name; })
    } else {
      None
    };

    quote! {
      #value_before
      #build_widget
      #state_ref
      #follow_after
    }
  }

  /// Generate field tokens with three part, the first is a tuple of field value
  /// and the follow condition, the second part is the field value declare in
  /// struct literal, the last part is expression to follow the other widgets
  /// change.
  ///
  /// The return value is the name of the follow condition;
  fn gen_field_tokens(
    &self,
    f: &DeclareField,
    value_before: &mut TokenStream,
    widget_def: &mut TokenStream,
    follow_after: &mut TokenStream,
  ) -> Option<Ident> {
    let DeclareField { if_guard, member, expr, .. } = f;
    let field_follow = self.field_follow_tokens(f);
    let def_name = widget_def_variable(&self.name);

    match (if_guard, field_follow.as_ref()) {
      (Some(guard), Some(_)) => {
        // we need to calculate `if guard` value before define widget to avoid twice
        // calculate it
        let guard_cond = Ident::new(&member.to_string(), guard.span());
        let guard_cond = ribir_suffix_variable(&guard_cond, "guard");
        value_before.extend(quote! {
            let #guard_cond = #if_guard { true } else { false };
        });
        widget_def.extend(quote! { if #guard_cond { #def_name.#member(#expr); }});
        follow_after.extend(quote! {if #guard_cond { #field_follow } });
        Some(guard_cond)
      }
      _ => {
        widget_def.extend(quote! {#def_name.#member(#expr);});
        follow_after.extend(field_follow);
        None
      }
    }
  }

  fn field_follow_tokens(&self, f: &DeclareField) -> Option<TokenStream> {
    let DeclareField {
      member, follows: depends_on, skip_nc, ..
    } = f;

    let ref_name = &self.name;
    let expr_tokens = f.value_tokens(self.ty);

    depends_on.as_ref().map(|follows| {
      let assign = skip_nc_assign(
        skip_nc.is_some(),
        &quote! { #ref_name.#member},
        &expr_tokens,
      );
      let upstream = upstream_observable(follows);

      quote! {
          #upstream.subscribe( move |_|{ #assign } );
      }
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
  fn value_tokens(&self, widget_ty: &Path) -> TokenStream {
    let Self { member, expr, .. } = self;
    let field_converter = field_convert_method(member);
    quote_spanned! { expr.span() => <#widget_ty as Declare>::Builder::#field_converter(#expr) }
  }
}
