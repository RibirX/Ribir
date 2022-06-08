use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, quote_spanned, ToTokens};
use syn::{
  parse::{Parse, ParseStream},
  spanned::Spanned,
  token, Ident,
};

use crate::{
  error::{FollowInfo, Result},
  widget_attr_macro::declare_widget::DeclareField,
};
mod declare_ctx;
pub use declare_ctx::*;
mod name_used_info;

pub use name_used_info::*;
mod variable_names;
pub use self::{declare_widget::DeclareWidget, widget_macro::WidgetMacro};
pub use variable_names::*;
mod animations;
mod dataflows;
mod declare_widget;
pub use declare_widget::RESERVE_IDENT;

mod track;
mod widget_macro;
pub mod kw {
  syn::custom_keyword!(widget);
  syn::custom_keyword!(dataflows);
  syn::custom_keyword!(animations);
  syn::custom_keyword!(track);
  syn::custom_keyword!(ExprWidget);
  syn::custom_keyword!(id);
  syn::custom_keyword!(skip_nc);
  syn::custom_keyword!(Animate);
  syn::custom_keyword!(State);
  syn::custom_keyword!(Transition);
}

fn skip_nc_assign<L, R>(skip_nc: bool, left: &L, right: &R) -> TokenStream2
where
  L: ToTokens,
  R: ToTokens,
{
  if skip_nc {
    let v = ribir_variable("v", left.span());
    quote! {
      let #v = #right;
      if #v != #left {
        #left = #v;
      }
    }
  } else {
    quote! { #left = #right; }
  }
}

#[derive(Debug)]
pub struct Id {
  pub id_token: kw::id,
  pub colon_token: token::Colon,
  pub name: Ident,
}

impl Parse for Id {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    Ok(Self {
      id_token: input.parse()?,
      colon_token: input.parse()?,
      name: input.parse()?,
    })
  }
}

impl ToTokens for Id {
  fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
    self.id_token.to_tokens(tokens);
    self.colon_token.to_tokens(tokens);
    self.name.to_tokens(tokens);
  }
}

impl Id {
  pub fn from_declare_field(field: DeclareField) -> syn::Result<Id> {
    if field.skip_nc.is_some() {
      return Err(syn::Error::new(
        field.skip_nc.span(),
        "Attribute `#[skip_nc]` is not supported in `id`",
      ));
    }
    if field.if_guard.is_some() {
      return Err(syn::Error::new(
        field.if_guard.span(),
        "if guard is not supported in `id`",
      ));
    }

    Ok(syn::parse_quote! {#field})
  }
}

fn widget_state_ref(widget: &Ident) -> TokenStream2 {
  quote_spanned!(widget.span() =>
    #[allow(unused_mut)]
    let mut #widget = #widget.state_ref();
  )
}

fn capture_widget(widget: &Ident) -> TokenStream2 {
  quote_spanned!(widget.span() => let #widget = #widget.clone();)
}
