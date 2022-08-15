use crate::util::struct_unwrap;
use proc_macro::{Diagnostic, Level};
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned, ToTokens};
use syn::{
  parse::{discouraged::Speculative, Parse},
  parse_quote,
  punctuated::Punctuated,
  spanned::Spanned,
  token, DataStruct, Fields, Ident, Result,
};

const DECLARE: &str = "Declare";
const BUILDER: &str = "Builder";
const DECLARE_ATTR: &str = "declare";

struct DefaultMeta {
  _default_kw: kw::default,
  _eq_token: Option<syn::token::Eq>,
  value: Option<syn::Expr>,
}

struct BoxTraitValue {
  _box_trait_kw: kw::box_trait,
  _paren: token::Paren,
  value: syn::TraitBound,
}
enum ConvertValue {
  Into(kw::into),
  BoxTrait(BoxTraitValue),
  Custom(kw::custom),
  Stipe(kw::strip_option),
}
struct ConvertMeta {
  _convert_kw: kw::convert,
  _eq_token: Option<syn::token::Eq>,
  value: ConvertValue,
}

#[derive(Default)]
struct DeclareAttr {
  rename: Option<syn::Ident>,
  builtin: Option<kw::builtin>,
  default: Option<DefaultMeta>,
  convert: Option<ConvertMeta>,
}

struct DeclareField<'a> {
  attr: Option<DeclareAttr>,
  field: &'a syn::Field,
}
mod kw {
  use syn::custom_keyword;
  custom_keyword!(rename);
  custom_keyword!(builtin);
  custom_keyword!(default);
  custom_keyword!(convert);
  custom_keyword!(into);
  custom_keyword!(box_trait);
  custom_keyword!(custom);
  custom_keyword!(strip_option);
}

#[inline]
pub fn declare_field_name(field_name: &Ident) -> Ident {
  let name = if field_name.to_string().starts_with("_") {
    format!("set_declare{field_name}",)
  } else {
    format!("set_declare_{field_name}",)
  };
  Ident::new(&name, field_name.span())
}

impl Parse for DefaultMeta {
  fn parse(input: syn::parse::ParseStream) -> Result<Self> {
    Ok(Self {
      _default_kw: input.parse()?,
      _eq_token: input.parse()?,
      value: {
        let ahead = input.fork();
        let expr = ahead.parse::<syn::Expr>();
        if expr.is_ok() {
          input.advance_to(&ahead);
        }
        expr.ok()
      },
    })
  }
}

impl Parse for ConvertMeta {
  fn parse(input: syn::parse::ParseStream) -> Result<Self> {
    Ok(Self {
      _convert_kw: input.parse()?,
      _eq_token: input.parse()?,
      value: input.parse()?,
    })
  }
}

impl Parse for ConvertValue {
  fn parse(input: syn::parse::ParseStream) -> Result<Self> {
    let lk = input.lookahead1();
    if lk.peek(kw::into) {
      input.parse().map(ConvertValue::Into)
    } else if lk.peek(kw::custom) {
      input.parse().map(ConvertValue::Custom)
    } else if lk.peek(kw::box_trait) {
      input.parse().map(ConvertValue::BoxTrait)
    } else if lk.peek(kw::strip_option) {
      input.parse().map(ConvertValue::Stipe)
    } else {
      Err(lk.error())
    }
  }
}

impl Parse for BoxTraitValue {
  fn parse(input: syn::parse::ParseStream) -> Result<Self> {
    let content;
    Ok(Self {
      _box_trait_kw: input.parse()?,
      _paren: syn::parenthesized!(content in input),
      value: content.parse()?,
    })
  }
}

impl Parse for DeclareAttr {
  fn parse(input: syn::parse::ParseStream) -> Result<Self> {
    let mut attr = DeclareAttr::default();
    while !input.is_empty() {
      let lookahead = input.lookahead1();

      // use input instead of lookahead to peek builtin, because need't complicate in
      // compile error.
      if input.peek(kw::builtin) {
        attr.builtin = Some(input.parse()?);
      } else if lookahead.peek(kw::rename) {
        input.parse::<kw::rename>()?;
        input.parse::<syn::Token![=]>()?;
        attr.rename = Some(input.parse()?);
      } else if lookahead.peek(kw::convert) {
        attr.convert = Some(input.parse()?);
      } else if lookahead.peek(kw::default) {
        attr.default = Some(input.parse()?);
      } else {
        return Err(lookahead.error());
      }
      if let (Some(rename), Some(builtin)) = (attr.rename.as_ref(), attr.builtin.as_ref()) {
        let mut d = Diagnostic::new(
          Level::Error,
          "`rename` and `builtin` can not be used in same time.",
        );
        d.set_spans(vec![rename.span().unwrap(), builtin.span().unwrap()]);
        d.emit();
      }
      if !input.is_empty() {
        input.parse::<syn::Token![,]>()?;
      }
    }
    Ok(attr)
  }
}

pub(crate) fn declare_derive(input: &mut syn::DeriveInput) -> syn::Result<TokenStream> {
  let syn::DeriveInput { vis, ident: name, generics, data, .. } = input;
  let (g_impl, g_ty, g_where) = generics.split_for_impl();

  let stt = struct_unwrap(data, DECLARE)?;
  let mut builder_fields = collect_filed_and_attrs(stt)?;

  // reverse name check.
  builder_fields
    .iter_mut()
    .for_each(DeclareField::check_reserve);

  let builder = Ident::new(&format!("{}{}", name, BUILDER), name.span());
  // builder define
  let def_fields = builder_fields.pairs().map(|p| {
    let (f, c) = p.into_tuple();
    let mut f = f.field.clone();
    let ty = &f.ty;
    f.ty = parse_quote!(Option<#ty>);
    syn::punctuated::Pair::new(f, c)
  });

  // implement declare trait
  let fields_ident = builder_fields.iter().map(|f| f.field.ident.as_ref());
  let builder_fields_ident = fields_ident.clone();

  let fill_default = builder_fields.iter().filter_map(|f| {
    let field_name = f.member();
    let method = f.set_method_name();
    f.attr.as_ref().and_then(|attr| {
      attr.default.as_ref().map(|df| {
        let set_default_value = if let Some(v) = df.value.as_ref() {
          quote! { self = self.#method(#v); }
        } else {
          quote! { self.#field_name = Some(<_>::default()) }
        };
        quote! {
          if self.#field_name.is_none() {
            #set_default_value
          }
        }
      })
    })
  });

  let init_values = builder_fields.iter().map(|df| {
    let field_name = df.field.ident.as_ref().unwrap();
    let method = df.set_method_name();
    quote_spanned! { field_name.span() =>
      self.#field_name.expect(&format!(
        "Required field `{}::{}` not set, use method `{}` init it",
        stringify!(#name), stringify!(#field_name), stringify!(#method)
      ))
    }
  });

  let mut builder_methods = quote! {};
  let mut methods = quote! {};
  builder_fields.iter().for_each(
    |f @ DeclareField {
       attr: _,
       field: syn::Field { vis: f_vis, ident, .. },
     }| {
      let field_name = ident.as_ref().unwrap();
      let set_method = f.set_method_name();
      let declare_set = declare_field_name(set_method);
      if let Some((value_arg, expr)) = f.setter_value_arg_and_expr() {
        builder_methods.extend(quote! {
          #[inline]
          #vis fn #set_method(mut self, #value_arg) -> Self {
            self.#field_name = Some(#expr);
            self
          }
        });
        methods.extend(quote! {
          #[inline]
          #f_vis fn #declare_set(&mut self, #value_arg) {
            self.#field_name = #expr;
          }
        });
      }
    },
  );

  let tokens = quote! {

      #vis struct #builder #g_ty #g_where {
        #(#def_fields)*
      }

      impl #g_impl Declare for #name #g_ty #g_where {
        type Builder = #builder #g_ty;

        fn builder() -> Self::Builder {
          #builder { #(#builder_fields_ident : None ),*}
        }
      }

      impl #g_impl DeclareBuilder for #builder #g_ty #g_where {
        type Target = #name #g_ty;
        #[inline]
        fn build(mut self, ctx: &mut BuildCtx) -> Self::Target {
          #(#fill_default)*
          #name {
            #(#fields_ident : #init_values),* }
        }
      }

      impl #g_impl #name #g_ty #g_where {
        #methods
      }

      impl #g_impl #builder #g_ty #g_where {
        #builder_methods
      }
  };
  Ok(tokens)
}

fn collect_filed_and_attrs(stt: &mut DataStruct) -> Result<Punctuated<DeclareField, token::Comma>> {
  let mut builder_fields = Punctuated::default();
  match &mut stt.fields {
    Fields::Named(named) => {
      named
        .named
        .pairs_mut()
        .try_for_each::<_, syn::Result<()>>(|pair| {
          let (field, comma) = pair.into_tuple();
          let idx = field
            .attrs
            .iter()
            .position(|attr| attr.path.is_ident(DECLARE_ATTR));
          let builder_attr = if let Some(idx) = idx {
            let attr = field.attrs.remove(idx);
            let args: DeclareAttr = attr.parse_args()?;
            Some(args)
          } else {
            None
          };

          builder_fields.push(DeclareField { attr: builder_attr, field });
          if let Some(c) = comma {
            builder_fields.push_punct(c.clone());
          }

          Ok(())
        })?;
    }
    Fields::Unit => <_>::default(),
    Fields::Unnamed(unnamed) => {
      let err = syn::Error::new(
        unnamed.span(),
        format!("`{}` not be supported to derive for tuple struct", DECLARE),
      );
      return Err(err);
    }
  };
  Ok(builder_fields)
}

impl<'a> DeclareField<'a> {
  fn member(&self) -> &Ident { self.field.ident.as_ref().unwrap() }

  fn set_method_name(&self) -> &Ident {
    self
      .attr
      .as_ref()
      .and_then(|attr| attr.rename.as_ref())
      .or(self.field.ident.as_ref())
      .unwrap()
  }

  fn setter_value_arg_and_expr(&self) -> Option<(TokenStream, TokenStream)> {
    let ty = &self.field.ty;
    let cv = self
      .attr
      .as_ref()
      .and_then(|attr| attr.convert.as_ref())
      .map(|meta| &meta.value);
    match cv {
      Some(ConvertValue::Into(_)) => Some((
        quote! { v: impl std::convert::Into<#ty> },
        quote! { v.into() },
      )),
      Some(ConvertValue::Stipe(_)) => Some((
        quote! { v: impl std::convert::Into<DeclareStripOption<#ty>> },
        quote! { v.into().into_option_value() },
      )),
      Some(ConvertValue::BoxTrait(BoxTraitValue { ref value, .. })) => {
        Some((quote! { v: impl #value + 'static }, quote! { Box::new(v)}))
      }
      // custom
      Some(ConvertValue::Custom(_)) => None,
      None => Some((quote! { v: #ty}, quote! {v})),
    }
  }

  fn check_reserve(&mut self) {
    // reverse name check.
    let reserve_ident = &crate::widget_attr_macro::RESERVE_IDENT;

    let not_builtin = self
      .attr
      .as_ref()
      .map_or(true, |attr| attr.builtin.is_none());

    if not_builtin {
      let method_name = self.set_method_name();
      if let Some(r) = reserve_ident.get(method_name.to_string().as_str()) {
        let msg = format!("the identify `{}` is reserved to {}", method_name, &r);
        let mut field = self.field.clone();
        // not display the attrs in the help code.

        field.attrs.clear();
        Diagnostic::spanned(vec![method_name.span().unwrap()], Level::Error, msg)
          .help(format! {
            "use `rename` meta to avoid the name conflict in `widget!` macro.\n\n\
            #[declare(rename = xxx)] \n\
            {}", field.into_token_stream()
          })
          .emit();
      }
    }
  }
}
