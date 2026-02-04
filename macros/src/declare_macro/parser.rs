use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{Ident, Result, parse::Parse};

mod kw {
  use syn::custom_keyword;
  custom_keyword!(default);
  custom_keyword!(custom);
  custom_keyword!(skip);
  custom_keyword!(strict);
  custom_keyword!(setter);
  custom_keyword!(validate);
  custom_keyword!(simple);
  custom_keyword!(stateless);
  custom_keyword!(event);
  custom_keyword!(eager);
}

pub(crate) struct ValidateMeta {
  pub(crate) validate_kw: kw::validate,
  pub(crate) method_name: Option<Ident>,
}

impl Parse for ValidateMeta {
  fn parse(input: syn::parse::ParseStream) -> Result<Self> {
    let validate_kw = input.parse()?;
    let method_name = if input.peek(syn::Token![=]) {
      let _: syn::Token![=] = input.parse()?;
      Some(input.parse()?)
    } else {
      None
    };
    Ok(Self { validate_kw, method_name })
  }
}

/// Event metadata supporting three syntaxes:
/// - `event = Type` (use Into trait)
/// - `event = Type.field` or `event = Type.a.b` (field path)
/// - `event = Type.method()` or `event = Type.a.method()` (method call with
///   optional chain)
pub(crate) struct EventMeta {
  /// The event type (e.g., `SliderChanged`)
  pub(crate) event_type: syn::Type,
  /// The conversion expression chain after the type (e.g., `.to` or
  /// `.extract()`) None means use Into trait directly
  pub(crate) convert_chain: Option<TokenStream>,
}

#[allow(dead_code)]
pub(crate) struct SetterMeta {
  pub(crate) setter_kw: kw::setter,
  pub(crate) eq_token: syn::Token![=],
  pub(crate) method_name: Ident,
  pub(crate) ty: Option<syn::Type>,
}

pub(crate) struct DefaultMeta {
  pub(crate) _default_kw: kw::default,
  pub(crate) _eq_token: Option<syn::token::Eq>,
  pub(crate) value: Option<syn::Expr>,
}

#[derive(Default)]
pub(crate) struct DeclareAttr {
  pub(crate) default: Option<DefaultMeta>,
  pub(crate) custom: Option<kw::custom>,
  // field with `skip` attr, will not generate setter method and use default to init value.
  pub(crate) skip: Option<kw::skip>,
  pub(crate) strict: Option<kw::strict>,
  // Setter binding: `setter = method_name` or `setter = method_name(Type)`
  pub(crate) setter: Option<SetterMeta>,
  pub(crate) validate: Option<ValidateMeta>,
  pub(crate) simple: Option<kw::simple>,
  pub(crate) stateless: Option<kw::stateless>,
  // Event binding: `event = Type` or `event = Type.field` or `event = Type.method()`
  pub(crate) event: Option<EventMeta>,
  pub(crate) eager: Option<kw::eager>,
}

impl DeclareAttr {
  pub(crate) fn check_conflicts(&self) -> Result<()> {
    if let (Some(custom), Some(skip)) = (self.custom.as_ref(), self.skip.as_ref()) {
      let mut err = syn::Error::new_spanned(
        custom,
        "A field marked as `skip` cannot implement a `custom` set method.",
      );
      err.combine(syn::Error::new_spanned(
        skip,
        "A field marked as `custom` cannot also be marked as `skip`.",
      ));
      return Err(err);
    }

    self.check_event_conflicts()?;
    Ok(())
  }

  fn check_event_conflicts(&self) -> Result<()> {
    if self.event.is_none() {
      return Ok(());
    }

    if let Some(simple) = &self.simple {
      return Err(syn::Error::new_spanned(
        simple,
        "`simple` attribute cannot be used with `event` attribute.",
      ));
    }
    if let Some(stateless) = &self.stateless {
      return Err(syn::Error::new_spanned(
        stateless,
        "`stateless` attribute cannot be used with `event` attribute.",
      ));
    }
    Ok(())
  }
}

impl Parse for DeclareAttr {
  fn parse(input: syn::parse::ParseStream) -> Result<Self> {
    let mut attr = DeclareAttr::default();

    while !input.is_empty() {
      let lookahead = input.lookahead1();

      if lookahead.peek(kw::custom) {
        attr.custom = Some(input.parse()?);
      } else if lookahead.peek(kw::default) {
        attr.default = Some(input.parse()?);
      } else if lookahead.peek(kw::skip) {
        attr.skip = Some(input.parse()?);
      } else if lookahead.peek(kw::strict) {
        attr.strict = Some(input.parse()?);
      } else if lookahead.peek(kw::setter) {
        attr.setter = Some(input.parse()?);
      } else if lookahead.peek(kw::validate) {
        attr.validate = Some(input.parse()?);
      } else if lookahead.peek(kw::simple) {
        attr.simple = Some(input.parse()?);
      } else if lookahead.peek(kw::stateless) {
        attr.stateless = Some(input.parse()?);
      } else if lookahead.peek(kw::eager) {
        attr.eager = Some(input.parse()?);
      } else if lookahead.peek(kw::event) {
        let _: kw::event = input.parse()?;
        let _: syn::Token![=] = input.parse()?;
        attr.event = Some(input.parse()?);
      } else {
        return Err(lookahead.error());
      }

      attr.check_conflicts()?;

      if !input.is_empty() {
        input.parse::<syn::Token![,]>()?;
      }
    }

    Ok(attr)
  }
}

impl Parse for SetterMeta {
  fn parse(input: syn::parse::ParseStream) -> Result<Self> {
    let kw: kw::setter = input.parse()?;
    let eq: syn::Token![=] = input.parse()?;
    let method: Ident = input.parse()?;
    let ty = if input.peek(syn::token::Paren) {
      let content;
      syn::parenthesized!(content in input);
      Some(content.parse()?)
    } else {
      None
    };
    Ok(Self { setter_kw: kw, eq_token: eq, method_name: method, ty })
  }
}

impl Parse for EventMeta {
  fn parse(input: syn::parse::ParseStream) -> Result<Self> {
    // Parse the event type first
    let event_type: syn::Type = input.parse()?;

    // Check if there's a dot-chain following
    let convert_chain = if input.peek(syn::Token![.]) {
      let mut tokens = TokenStream::new();

      // Parse the entire chain: .field.field2.method()
      while input.peek(syn::Token![.]) {
        let dot: syn::Token![.] = input.parse()?;
        dot.to_tokens(&mut tokens);

        let ident: Ident = input.parse()?;
        ident.to_tokens(&mut tokens);

        // Check for method call parentheses
        if input.peek(syn::token::Paren) {
          let content;
          let paren = syn::parenthesized!(content in input);
          let inner: TokenStream = content.parse()?;
          paren.surround(&mut tokens, |t| inner.to_tokens(t));
        }
      }

      Some(tokens)
    } else {
      None
    };

    Ok(Self { event_type, convert_chain })
  }
}

impl Parse for DefaultMeta {
  fn parse(input: syn::parse::ParseStream) -> Result<Self> {
    let _default_kw: kw::default = input.parse()?;
    let _eq_token: Option<syn::token::Eq> = input.parse()?;
    let value: Option<syn::Expr> =
      if _eq_token.is_some() && !input.is_empty() { Some(input.parse()?) } else { None };
    Ok(Self { _default_kw, _eq_token, value })
  }
}
