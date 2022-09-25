use proc_macro2::TokenStream as TokenStream2;
use quote::{quote_spanned, ToTokens};
use syn::{
  parse::{Parse, ParseStream},
  spanned::Spanned,
  token::{self, Comma},
  Ident,
};

use crate::widget_attr_macro::declare_widget::DeclareField;
mod declare_ctx;
pub use declare_ctx::*;
mod name_used_info;

pub use name_used_info::*;
mod variable_names;
pub use self::{declare_widget::DeclareWidget, widget_macro::WidgetMacro};
pub use variable_names::*;
pub mod animations;
mod declare_widget;
mod on_change;
mod on_event_do;
pub use declare_widget::RESERVE_IDENT;

mod track;
mod widget_macro;
pub mod kw {
  syn::custom_keyword!(widget);
  syn::custom_keyword!(animations);
  syn::custom_keyword!(track);
  syn::custom_keyword!(ExprWidget);
  syn::custom_keyword!(id);
  syn::custom_keyword!(skip_nc);
  syn::custom_keyword!(Animate);
  syn::custom_keyword!(State);
  syn::custom_keyword!(Transition);
  syn::custom_punctuation!(FlowArrow, ~>);
  syn::custom_keyword!(on);
  syn::custom_keyword!(change);
}
pub const CHANGE: &str = "change";

#[derive(Debug)]
pub struct Id {
  pub id_token: kw::id,
  pub colon_token: token::Colon,
  pub name: Ident,
  pub tail_comma: Option<token::Comma>,
}

impl Parse for Id {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    Ok(Self {
      id_token: input.parse()?,
      colon_token: input.parse()?,
      name: input.parse()?,
      tail_comma: input.parse()?,
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
  pub fn from_field_pair(p: syn::punctuated::Pair<DeclareField, Comma>) -> syn::Result<Id> {
    let field = p.value();
    if field.skip_nc.is_some() {
      return Err(syn::Error::new(
        field.skip_nc.span(),
        "Attribute `#[skip_nc]` is not supported in `id`",
      ));
    }

    Ok(syn::parse_quote! {#p})
  }
}

fn capture_widget(widget: &Ident) -> TokenStream2 {
  quote_spanned!(widget.span() => let #widget = #widget.clone_stateful();)
}
