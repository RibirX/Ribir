use std::collections::BTreeMap;

use lazy_static::lazy_static;
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};

use syn::{
  parse::{Parse, ParseStream},
  parse_quote,
  spanned::Spanned,
  token::{Brace, Comma},
  Ident, Member, Result, Token,
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

use crate::{declare_func_derive::FieldFollows, error::DeclareError};

use super::IfGuard;
use super::{declare_visit_mut::DeclareCtx, WidgetFollowPart};
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
    $(#[$m:meta])*
    $v: vis struct $name:ident {
      #[attributes]

      $(
        #[doc=$attr_doc: literal]
        $attrs:ident : $a_ty:ty,
      )*

      #[listeners]

      $(
        #[doc=$listener_doc: literal]
        $listeners:ident : $l_ty:ty,
      )*

      #[widget_wrap]
      $(
        #[doc=$w_wrap_doc: literal]
        $w_wrap:ident : $w_ty:ty
      ),*
    }
  ) => {
    $(#[$m])*
    $v struct $name {
      $($attrs : $a_ty,)*
      $($listeners: $l_ty,)*
      $($w_wrap: $w_ty),*
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

    impl $name {

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

// todo: auto generate the sugar fields list document by the below code.
fields_sugar_def! {
  #[derive(Default)]
  pub struct SugarFields {
    #[attributes]
    #[doc="assign a key to the widget use for track it when tree rebuild."]
    key: Option<DeclareField>,
    #[doc="assign cursor to the widget."]
    cursor: Option<DeclareField>,
    #[doc="assign theme to the widget."]
    theme: Option<DeclareField>,
    #[doc="Indicates whether the widget should automatically get focus when the window loads."]
    auto_focus: Option<DeclareField>,
    #[doc="indicates that `widget` can be focused, and where it participates in \
          sequential keyboard navigation (usually with the Tab key, hence the name."]
    tab_index: Option<DeclareField>,

    #[listeners]

    #[doc="specify the event handler for the pointer down event."]
    on_pointer_down: Option<DeclareField>,
    #[doc="specify the event handler for the pointer up event."]
    on_pointer_up: Option<DeclareField>,
    #[doc="specify the event handler for the pointer move event."]
    on_pointer_move: Option<DeclareField>,
    #[doc="specify the event handler for the pointer tap event."]
    on_tap: Option<DeclareField>,
    #[doc="specify the event handler for processing the specified times tap."]
    on_tap_times: Option<DeclareField>,
    #[doc="specify the event handler to process pointer cancel event."]
    on_pointer_cancel: Option<DeclareField>,
    #[doc="specify the event handler when pointer enter this widget."]
    on_pointer_enter: Option<DeclareField>,
    #[doc="specify the event handler when pointer leave this widget."]
    on_pointer_leave: Option<DeclareField>,
    #[doc="specify the event handler to process focus event."]
    on_focus: Option<DeclareField>,
    #[doc="specify the event handler to process blur event."]
    on_blur: Option<DeclareField>,
    #[doc="specify the event handler to process focusin event."]
    on_focus_in: Option<DeclareField>,
    #[doc="specify the event handler to process focusout event."]
    on_focus_out: Option<DeclareField>,
    #[doc="specify the event handler when keyboard press down."]
    on_key_down: Option<DeclareField>,
    #[doc="specify the event handler when a key is released."]
    on_key_up: Option<DeclareField>,
    #[doc="specify the event handler when received a unicode character."]
    on_char: Option<DeclareField>,
    #[doc="specify the event handler when user moving a mouse wheel or similar input device."]
    on_wheel: Option<DeclareField>,

    #[widget_wrap]
    // padding should always before margin, it widget have margin & padding both
    // margin should wrap padding.
    #[doc="set the padding area on all four sides of the widget."]
    padding: Option<DeclareField>,
    #[doc="expand space around widget wrapped."]
    margin: Option<DeclareField>,
    #[doc="specify the background of the widget box."]
    background: Option<DeclareField>,
    #[doc="specify the border of the widget which draw above the background"]
    border: Option<DeclareField>,
    #[doc= "specify how rounded the corners have of the widget."]
    radius: Option<DeclareField>
  }
}

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
    def_name: &Ident,
    ref_name: &Ident,
    ctx: &DeclareCtx,
  ) -> Vec<WrapWidgetTokens> {
    let mut tokens = vec![];

    if let Some(padding @ DeclareField { expr, member, .. }) = self.padding.as_ref() {
      let lit = if padding.colon_token.is_some() {
        quote! { Padding { #member: #expr } }
      } else {
        quote! { Padding { #member } }
      };
      tokens.push(common_def_tokens(padding, def_name, ref_name, lit, ctx));
    }

    if let Some(d) = self.decoration_widget_tokens(def_name, ref_name, ctx) {
      tokens.push(d);
    }

    if let Some(margin @ DeclareField { expr, member, .. }) = self.margin.as_ref() {
      let lit = if margin.colon_token.is_some() {
        quote! { Margin { #member: #expr } }
      } else {
        quote! { Margin { #member } }
      };
      tokens.push(common_def_tokens(margin, def_name, ref_name, lit, ctx));
    }

    tokens
  }

  pub fn wrap_widget_follows<'a>(
    &'a self,
    ref_name: &Ident,
    ctx: &DeclareCtx,
    follows_info: &mut BTreeMap<Ident, WidgetFollows<'a>>,
  ) {
    let mut copy_follows = |f: Option<&'a DeclareField>| {
      if let Some(follows) = f.and_then(FieldFollows::clone_from) {
        let name = ctx.no_conflict_name_with_suffix(ref_name, &follows.field.member);
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
      let suffix = Ident::new(DECORATION, proc_macro2::Span::call_site());
      let name = ctx.no_conflict_name_with_suffix(ref_name, &suffix);
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
    def_name: &Ident,
    ref_name: &Ident,
    ctx: &DeclareCtx,
  ) -> Option<WrapWidgetTokens> {
    let Self { border, radius, background: bg, .. } = self;

    if border.is_none() && bg.is_none() && radius.is_none() {
      return None;
    }

    let suffix = Ident::new(DECORATION, proc_macro2::Span::call_site());
    let wrap_def_name = ctx.no_conflict_name_with_suffix(def_name, &suffix);
    let wrap_ref_name = ctx.no_conflict_name_with_suffix(ref_name, &suffix);
    let mut value_before = quote! {};
    let mut follow_after = quote! {};
    let mut decoration = quote! { BoxDecoration };

    // decoration can emit if all user declared field have if guard and its
    // condition result is false.
    let mut decoration_cond = quote! {};

    fn value_converter(value: &syn::Expr) -> TokenStream {
      quote! { Some(#value.into())}
    }

    let mut def_and_ref_tokens = quote! {};

    Brace::default().surround(&mut decoration, |decoration| {
      let comma = Comma::default();
      let mut gen_decoration_field_tokens = |f: &DeclareField| -> Option<Ident> {
        if ctx.be_followed(ref_name) {
          let bg_ref = ctx.no_conflict_name_with_suffix(ref_name, &f.member);
          follow_after.extend(quote! { let #bg_ref = #wrap_def_name.ref_cell();});
        }
        let cond = f.gen_tokens(
          &wrap_def_name,
          &wrap_ref_name,
          &mut value_before,
          decoration,
          &mut follow_after,
          Some(value_converter),
          ctx,
        );
        comma.to_tokens(decoration);
        cond
      };

      let bg_cond = bg.as_ref().and_then(&mut gen_decoration_field_tokens);
      let border_cond = border.as_ref().and_then(&mut gen_decoration_field_tokens);
      let radius_cond = radius.as_ref().and_then(&mut gen_decoration_field_tokens);

      if bg.is_none() || border.is_none() || radius.is_none() {
        decoration.extend(quote! { ..<_>::default() })
      }

      if bg_cond.is_some() && border_cond.is_some() && radius_cond.is_some() {
        decoration_cond = quote! { #bg_cond || #border_cond || #radius_cond };
      }
    });

    let stateful = (!follow_after.is_empty()).then(|| quote! { .into_stateful() });

    def_and_ref_tokens.extend(value_before);
    if decoration_cond.is_empty() {
      def_and_ref_tokens.extend(quote! {
        let #wrap_def_name = #decoration #stateful;
      });
      def_and_ref_tokens.extend(follow_after);
    } else {
      def_and_ref_tokens.extend(quote! {
        let #wrap_def_name = #decoration_cond.then(||{
          let #wrap_def_name = #decoration #stateful;
          #follow_after
          #wrap_def_name
        });
      });
    }

    Some(WrapWidgetTokens {
      compose_tokens: quote! { let #def_name = (#wrap_def_name, #def_name).compose(); },
      name: wrap_def_name,
      def_and_ref_tokens,
    })
  }
}

// generate the wrapper widget define tokens and return the wrap tokens.
fn common_def_tokens(
  f @ DeclareField { if_guard, member, .. }: &DeclareField,
  def_name: &Ident,
  ref_name: &Ident,
  widget_lit: TokenStream,
  ctx: &DeclareCtx,
) -> WrapWidgetTokens {
  fn wrap_if_guard(name: &Ident, if_guard: &IfGuard, to_wrap: TokenStream) -> TokenStream {
    quote! {
      let #name = #if_guard {
        #to_wrap
        Some(#name)
      } else {
        None
      };
    }
  }

  // wrap widget should be stateful if it's depended by other or itself need
  // follow other change.
  fn is_stateful(name: &Ident, f: &DeclareField, ctx: &DeclareCtx) -> bool {
    f.follows.is_some() || ctx.be_followed(name)
  }

  let wrap_name = ctx.no_conflict_name_with_suffix(def_name, &member);
  let wrap_ref = ctx.no_conflict_name_with_suffix(ref_name, &member);
  let stateful = is_stateful(&wrap_ref, f, ctx).then(|| quote! { .into_stateful() });

  let field_follow = f.follow_tokens(&wrap_ref, &wrap_name, None, ctx);
  let widget_tokens = quote! {
    let #wrap_name = #widget_lit #stateful;
    #field_follow
  };

  let mut def_and_ref_tokens = quote! {};
  if let Some(if_guard) = if_guard {
    def_and_ref_tokens.extend(wrap_if_guard(&wrap_name, if_guard, widget_tokens));
  } else {
    def_and_ref_tokens.extend(widget_tokens);
    // widget have `if guard` syntax, can not be depended.

    if ctx.be_followed(&wrap_ref) {
      def_and_ref_tokens.extend(quote! { let #wrap_ref =  #wrap_name.ref_cell(); });
    }
  }

  WrapWidgetTokens {
    compose_tokens: quote! { let #def_name = (#wrap_name, #def_name).compose(); },
    name: wrap_name,
    def_and_ref_tokens,
  }
}
