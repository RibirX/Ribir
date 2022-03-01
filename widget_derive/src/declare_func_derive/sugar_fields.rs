use std::collections::BTreeMap;

use lazy_static::lazy_static;
use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};

use syn::{
  parse::{Parse, ParseStream},
  parse_quote,
  spanned::Spanned,
  Ident, Member, Path, Result, Token,
};

macro_rules! assign_uninit_field {
  ($self: ident.$name: ident, $field: ident) => {
    if $self.$name.is_none() {
      $self.$name = Some($field);
      Ok(None)
    } else {
      Err(syn::Error::new(
        $field.span(),
        format!("field `{}` specified more than once", stringify!($name)).as_str(),
      ))
    }
  };
}
pub(crate) use assign_uninit_field;

use crate::{
  declare_func_derive::{widget_gen::WidgetGen, FieldFollows},
  error::DeclareError,
};

use super::{
  declare_visit_mut::DeclareCtx, ribir_suffix_variable, widget_def_variable, WidgetFollowPart,
};
use super::{DeclareField, WidgetFollows};

pub struct Id {
  pub id_token: kw::id,
  pub colon_token: Token![:],
  pub name: Ident,
}

impl Parse for Id {
  fn parse(input: ParseStream) -> Result<Self> {
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

    Ok(parse_quote! {#field})
  }
}

macro_rules! fields_sugar_def {
  (
    #attributes
    $(
      #[doc=$attr_doc: literal]
      $attrs:ident: $a_ty: literal,
    )*

    #listeners

    $(
      #[doc=$listener_doc: literal]
      $listeners:ident: $l_ty: literal,
    )*

    #widget_wrap
    $(
      #[doc=$w_wrap_doc: literal]
      $w_wrap:ident : $w_ty: literal
    ),*
  ) => {
    #[derive(Default)]
    pub struct SugarFields {
      $($attrs : Option<DeclareField>,)*
      $($listeners: Option<DeclareField>,)*
      $($w_wrap: Option<DeclareField>),*
    }


    lazy_static! {
      pub static ref RESERVE_IDENT: std::collections::HashMap<&'static str, &'static str>
        = std::collections::HashMap::from([
          ("id", "give an identify to the widget, so that other widgets in same macro can use the `id` to access it."),
          $((stringify!($attrs), $attr_doc),)*
          $((stringify!($listeners), $listener_doc), )*
          $((stringify!($w_wrap), $w_wrap_doc)),*
      ]);
    }

    impl SugarFields {

      pub fn assign_field(&mut self, f: DeclareField) -> Result<Option<DeclareField>> {
        $(
          if f.member == stringify!($attrs) {
            return assign_uninit_field!(self.$attrs, f)
          }
        )*
        $(
          if f.member == stringify!($listeners) {
            return assign_uninit_field!(self.$listeners, f)
          }
        )*
        $(
          if f.member == stringify!($w_wrap) {
            return assign_uninit_field!(self.$w_wrap, f)
          }
        )*
        Ok(Some(f))
      }

      pub fn visit_sugar_field_mut(&mut self, ctx: &mut DeclareCtx) {
        $(
          if let Some(f) = self.$attrs.as_mut() {
            ctx.visit_declare_field_mut(f);
          }
        )*
        $(
          if let Some(f) = self.$listeners.as_mut() {
            ctx.visit_declare_field_mut(f);
          }
        )*
        $(
          if let Some(f) = self.$w_wrap.as_mut() {
            ctx.visit_declare_field_mut(f);
          }
        )*
      }

      pub fn listeners_iter(&self) -> impl Iterator<Item = &DeclareField> {
        vec![$(self.$listeners.as_ref(),)*]
        .into_iter()
        .filter_map(|v| v)
      }

      pub fn normal_attr_iter(&self) -> impl Iterator<Item = &DeclareField> {
        vec![$(self.$attrs.as_ref(),)*]
        .into_iter()
        .filter_map(|v| v)
      }

      pub fn widget_wrap_field_iter(&self) -> impl Iterator<Item = &DeclareField> {
        vec![$(self.$w_wrap.as_ref(),)*]
        .into_iter()
        .filter_map(|v| v)
      }

      pub fn as_widget_wrap_name_field(mem: &Member) -> Option<&Ident> {
        match mem {
          Member::Named(name) => {
            match name.to_string().as_str() {
              $(stringify!($w_wrap) => Some(name),)*
              _ => None
            }
          }
          Member::Unnamed(_) => None,
        }
      }
    }
  }
}
pub mod kw {
  syn::custom_keyword!(id);
  syn::custom_keyword!(data_flow);
  syn::custom_keyword!(skip_nc);
}

include!("./sugar_fields_struct.rs");

const DECORATION: &str = "decoration";

pub struct WrapWidgetTokens {
  pub name: Ident,
  pub def_and_ref_tokens: TokenStream,
  pub compose_tokens: TokenStream,
}

impl SugarFields {
  // generate tokens of the wrap widgets define and return token stream to compose
  // these widgets and its host widget which should call after all children
  // composed.
  pub fn gen_wrap_widgets_tokens(
    &self,
    host_id: &Ident,
    ctx_name: &Ident,
    ctx: &DeclareCtx,
  ) -> Vec<WrapWidgetTokens> {
    let mut tokens = vec![];

    if let Some(padding) = self.padding.clone() {
      let w_ty = Ident::new("Padding", padding.member.span()).into();
      tokens.push(common_def_tokens(padding, &w_ty, host_id, ctx_name, ctx));
    }

    if let Some(d) = self.decoration_widget_tokens(host_id, ctx_name, ctx) {
      tokens.push(d);
    }

    if let Some(margin) = self.margin.clone() {
      let w_ty = Ident::new("Margin", margin.member.span()).into();
      tokens.push(common_def_tokens(margin, &w_ty, host_id, ctx_name, ctx));
    }

    tokens
  }

  pub fn collect_wrap_widget_follows<'a>(
    &'a self,
    host_name: &Ident,
    follows_info: &mut BTreeMap<Ident, WidgetFollows<'a>>,
  ) {
    let mut copy_follows = |f: Option<&'a DeclareField>| {
      if let Some(follows) = f.and_then(FieldFollows::clone_from) {
        let name = ribir_suffix_variable(host_name, &follows.field.member.to_string());
        let part = WidgetFollowPart::Field(follows);
        follows_info.insert(name, WidgetFollows::from_single_part(part));
      }
    };

    copy_follows(self.margin.as_ref());
    copy_follows(self.padding.as_ref());

    let bg_follows = self.background.as_ref().and_then(FieldFollows::clone_from);
    let border_follows = self.border.as_ref().and_then(FieldFollows::clone_from);
    let radius_follows = self.radius.as_ref().and_then(FieldFollows::clone_from);

    let deco_follows: WidgetFollows = bg_follows
      .into_iter()
      .chain(border_follows.into_iter())
      .chain(radius_follows.into_iter())
      .map(WidgetFollowPart::Field)
      .collect();

    if !deco_follows.is_empty() {
      let name = ribir_suffix_variable(host_name, DECORATION);
      follows_info.insert(name, deco_follows);
    }
  }

  pub fn key_follow_check(&self) -> crate::error::Result<()> {
    if let Some(DeclareField { member, follows: Some(follows), .. }) = self.key.as_ref() {
      Err(DeclareError::KeyDependsOnOther {
        key: member.span(),
        depends_on: follows.names().map(|k| k.span()).collect(),
      })
    } else {
      Ok(())
    }
  }

  fn decoration_widget_tokens(
    &self,
    host_id: &Ident,
    ctx_name: &Ident,
    ctx: &DeclareCtx,
  ) -> Option<WrapWidgetTokens> {
    let Self { border, radius, background, .. } = self;
    let mut fields = vec![];
    if let Some(border) = border {
      fields.push(border.clone())
    }
    if let Some(background) = background {
      fields.push(background.clone())
    }
    if let Some(radius) = radius {
      fields.push(radius.clone())
    }

    (!fields.is_empty()).then(|| {
      let name = ribir_suffix_variable(host_id, DECORATION);
      let span = fields
        .iter()
        .fold(None, |span: Option<Span>, f| {
          if let Some(span) = span {
            span.join(f.member.span())
          } else {
            Some(f.member.span())
          }
        })
        .unwrap();
      let ty = &Ident::new("BoxDecoration", span).into();
      let gen = WidgetGen { ty, name, fields: &fields, ctx_name };
      let host_name = widget_def_variable(host_id);
      let wrap_name = widget_def_variable(&gen.name);
      let mut def_and_ref_tokens = gen.gen_widget_tokens(ctx, false);

      // If all fields have if guard and condition are false, `BoxDecoration` can
      // emit.
      if fields.iter().all(|f| f.if_guard.is_some()) {
        def_and_ref_tokens = quote! {
          let #wrap_name = #wrap_name.is_empty().then(||{
            #def_and_ref_tokens
            #wrap_name
          });
        };
      }

      WrapWidgetTokens {
        compose_tokens: quote! { let #host_name = (#wrap_name, #host_name).compose(); },
        name: widget_def_variable(&gen.name),
        def_and_ref_tokens,
      }
    })

    // fixme:
    // 1. others follow decoration
    // if ctx.be_followed(ref_name) {
    //   let state_ref = ctx.no_conflict_name_with_suffix(ref_name, &f.member);
    //   follow_after.extend(quote! { let #state_ref = unsafe
    // {#wrap_def_name.state_ref()};}); }
  }
}

// generate the wrapper widget define tokens and return the wrap tokens.
fn common_def_tokens(
  mut f: DeclareField,
  ty: &Path,
  host_id: &Ident,
  ctx_name: &Ident,
  ctx: &DeclareCtx,
) -> WrapWidgetTokens {
  let if_guard = f.if_guard.take();
  let name = ribir_suffix_variable(host_id, &f.member.to_string());
  let host_def = widget_def_variable(host_id);
  let wrap_def = widget_def_variable(&name);
  let widget_gen = WidgetGen { ty, name, fields: &vec![f], ctx_name };
  let mut widget_tokens = widget_gen.gen_widget_tokens(ctx, false);

  if let Some(if_guard) = if_guard {
    widget_tokens = quote! {
      let #wrap_def = #if_guard {
        #widget_tokens
        Some(#wrap_def)
      } else {
        None
      };
    };
  }

  WrapWidgetTokens {
    compose_tokens: quote! { let #host_def = (#wrap_def, #host_def).compose(); },
    name: wrap_def,
    def_and_ref_tokens: widget_tokens,
  }
}
