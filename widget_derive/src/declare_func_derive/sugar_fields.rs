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
    assign_uninit_field!($self.$name, $field, $name)
  };
  ($left: expr, $right: ident, $name: ident) => {
    if $left.is_none() {
      $left = Some($right);
      Ok(())
    } else {
      Err(syn::Error::new(
        $right.span(),
        format!("field `{}` specified more than once", stringify!($name)).as_str(),
      ))
    }
  };
}
pub(crate) use assign_uninit_field;

use crate::{declare_func_derive::widget_gen::WidgetGen, error::DeclareError};

use super::{
  declare_visit_mut::DeclareCtx, kw, ribir_suffix_variable, widget_def_variable, FollowPart,
};
use super::{declare_widget::DeclareField, Follows};

#[derive(Debug)]
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
      pub const BUILTIN_LISTENERS:  [&'static str; 16] = [$(stringify!($listeners)),*];
      pub const BUILTIN_DATA_ATTRS:  [&'static str; 5] = [$(stringify!($attrs)),*];

      pub fn assign_field(&mut self, f: DeclareField) -> Result<Option<DeclareField>> {
        $(
          if f.member == stringify!($attrs) {
            assign_uninit_field!(self.$attrs, f)?;
            return Ok(None);
          }
        )*
        $(
          if f.member == stringify!($listeners) {
            assign_uninit_field!(self.$listeners, f)?;
            return Ok(None);
          }
        )*
        $(
          if f.member == stringify!($w_wrap) {
            assign_uninit_field!(self.$w_wrap, f)?;
            return Ok(None);
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

      pub fn wrap_widget_from_field_name(name: &Ident) -> Option<Ident> {
        if DECORATION_FIELDS.iter().find(|f| name == f).is_some() {
          Some(Ident::new(DECORATION, name.span()))
        } else {
          match name.to_string().as_str() {
            $(stringify!($w_wrap) => Some(name.clone()),)*
            _ => None
          }
        }
      }

      pub fn wrap_widget_from_member(mem: &Member) -> Option<Ident> {
        match mem {
          Member::Named(name) => {
            Self::wrap_widget_from_field_name(name)
          }
          Member::Unnamed(_) => None,
        }
      }
    }
  }
}

include!("./sugar_fields_struct.rs");

const DECORATION: &str = "decoration";
const DECORATION_FIELDS: [&str; 3] = ["background", "radius", "border"];
const PADDING: &str = "Padding";
const MARGIN: &str = "Margin";
const BOX_DECORATION: &str = "BoxDecoration";

impl SugarFields {
  pub fn gen_wrap_widgets_tokens<F>(&self, host: &Ident, ctx: &DeclareCtx, mut f: F)
  where
    F: FnMut(Ident, TokenStream),
  {
    if let Some(padding) = self.padding.clone() {
      let w_ty = Ident::new(PADDING, padding.member.span()).into();
      let (name, tokens) = common_def_tokens(padding, &w_ty, host, ctx);
      f(name, tokens);
    }

    if self.has_box_decoration_field() {
      let (name, tokens) = self.decoration_widget_tokens(host, ctx);
      f(name, tokens)
    }

    if let Some(margin) = self.margin.clone() {
      let w_ty = Ident::new(MARGIN, margin.member.span()).into();
      let (name, tokens) = common_def_tokens(margin, &w_ty, host, ctx);
      f(name, tokens)
    }
  }

  pub fn gen_wrap_widget_compose_tokens(&self, host: &Ident) -> TokenStream {
    let mut compose_tokens = quote! {};

    fn compose(host: &Ident, suffix: &str) -> TokenStream {
      let name = ribir_suffix_variable(host, suffix);
      let wrap_def = widget_def_variable(&name);
      let host_def = widget_def_variable(host);
      quote! {let #host_def = #wrap_def.have_child(#host_def);}
    }

    if let Some(padding) = self.padding.as_ref() {
      compose_tokens.extend(compose(host, &padding.member.to_string()));
    }

    if self.has_box_decoration_field() {
      compose_tokens.extend(compose(host, DECORATION))
    }

    if let Some(margin) = self.margin.clone() {
      compose_tokens.extend(compose(host, &margin.member.to_string()));
    }

    compose_tokens
  }

  pub fn collect_wrap_widget_follows<'a>(
    &'a self,
    host: &Ident,
    follows_info: &mut BTreeMap<Ident, Follows<'a>>,
  ) {
    let mut copy_follows = |f: Option<&'a DeclareField>| {
      if let Some(part) = f.and_then(FollowPart::from_widget_field) {
        let name = ribir_suffix_variable(host, &f.unwrap().member.to_string());
        follows_info.insert(name, Follows::from_single_part(part));
      }
    };

    copy_follows(self.margin.as_ref());
    copy_follows(self.padding.as_ref());

    let bg_follows = self
      .background
      .as_ref()
      .and_then(FollowPart::from_widget_field);
    let border_follows = self.border.as_ref().and_then(FollowPart::from_widget_field);
    let radius_follows = self.radius.as_ref().and_then(FollowPart::from_widget_field);

    let deco_follows: Follows = bg_follows
      .into_iter()
      .chain(border_follows.into_iter())
      .chain(radius_follows.into_iter())
      .collect();

    if !deco_follows.is_empty() {
      let name = ribir_suffix_variable(host, DECORATION);
      follows_info.insert(name, deco_follows);
    }
  }

  pub fn key_follow_check(&self) -> crate::error::Result<()> {
    if let Some(DeclareField { member, follows: Some(follows), .. }) = self.key.as_ref() {
      Err(DeclareError::KeyDependsOnOther {
        key: member.span().unwrap(),
        depends_on: follows.iter().map(|fo| fo.widget.span().unwrap()).collect(),
      })
    } else {
      Ok(())
    }
  }

  fn has_box_decoration_field(&self) -> bool {
    let Self { border, radius, background, .. } = self;
    border.is_some() || radius.is_some() || background.is_some()
  }

  fn decoration_widget_tokens(&self, host_id: &Ident, ctx: &DeclareCtx) -> (Ident, TokenStream) {
    let Self { border, radius, background, .. } = self;
    let fields = [border, radius, background]
      .iter()
      .filter_map(|f| (*f).clone())
      .collect::<Vec<_>>();

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
    let ty = &Ident::new(BOX_DECORATION, span).into();
    let gen = WidgetGen { ty, name, fields: &fields };
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

    (gen.name.clone(), def_and_ref_tokens)
  }
}

// generate the wrapper widget define tokens and return the wrap tokens.
fn common_def_tokens(
  mut f: DeclareField,
  ty: &Path,
  host: &Ident,

  ctx: &DeclareCtx,
) -> (Ident, TokenStream) {
  let if_guard = f.if_guard.take();
  let name = ribir_suffix_variable(host, &f.member.to_string());
  let wrap_def = widget_def_variable(&name);
  let widget_gen = WidgetGen { ty, name, fields: &vec![f] };
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

  (widget_gen.name.clone(), widget_tokens)
}
