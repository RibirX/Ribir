use crate::util::struct_unwrap;
use proc_macro::{Diagnostic, Level};
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::{
  parse::Parse, parse_quote, punctuated::Punctuated, spanned::Spanned, token, DataStruct, Fields,
  Ident, Result,
};

const DECLARE: &str = "Declare";
const BUILDER: &str = "Builder";
const DECLARE_ATTR: &str = "declare";

struct DefaultValue {
  default_token: kw::default,
  _eq_token: Option<syn::token::Eq>,
  value: Option<syn::LitStr>,
}

#[derive(Default)]
struct DeclareAttr {
  rename: Option<syn::LitStr>,
  builtin: Option<kw::builtin>,
  default: Option<DefaultValue>,
  custom_convert: Option<kw::custom_convert>,
}

mod kw {
  use syn::custom_keyword;
  custom_keyword!(rename);
  custom_keyword!(builtin);
  custom_keyword!(default);
  custom_keyword!(custom_convert);
}

pub fn field_convert_method(field_name: &Ident) -> Ident {
  Ident::new(&format!("{field_name}_convert",), field_name.span())
}

pub fn field_default_method(field_name: &Ident) -> Ident {
  Ident::new(&format!("{field_name}_default",), field_name.span())
}

impl Parse for DefaultValue {
  fn parse(input: syn::parse::ParseStream) -> Result<Self> {
    Ok(Self {
      default_token: input.parse()?,
      _eq_token: input.parse()?,
      value: input.parse()?,
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
      } else if lookahead.peek(kw::custom_convert) {
        attr.custom_convert = Some(input.parse()?);
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

  let builder = Ident::new(&format!("{}{}", name, BUILDER), name.span());

  // rename fields if need
  builder_fields
    .iter_mut()
    .try_for_each::<_, syn::Result<()>>(|(f, attr)| {
      if let Some(new_name) = attr.as_ref().and_then(|attr| attr.rename.as_ref()) {
        f.ident = Some(syn::Ident::new(new_name.value().as_str(), new_name.span()));
      }
      Ok(())
    })?;

  // reverse name check.
  let reserve_ident = &crate::widget_attr_macro::RESERVE_IDENT;
  builder_fields
    .iter_mut()
    .filter_map(|(f, attr)| {
      let not_builtin = attr.as_ref().map_or(true, |attr| attr.builtin.is_none());
      not_builtin.then(|| f)
    })
    .for_each(|f| {
      let field_name = f.ident.as_ref().unwrap();
      if let Some(doc) = reserve_ident.get(field_name.to_string().as_str()) {
        let msg = format!("the identify `{}` is reserved to {}", field_name, &doc);
        // not display the attrs in the help code.
        f.attrs.clear();
        Diagnostic::spanned(vec![field_name.span().unwrap()], Level::Error, msg)
          .help(format! {
            "use `rename` meta to avoid the name conflict in `widget!` macro.\n\n\
            #[declare(rename = \"xxx\")] \n\
            {}", quote!{ #f }
          })
          .emit();
      }
    });

  // builder define
  let def_fields = builder_fields.pairs().map(|p| {
    let ((f, _), c) = p.into_tuple();
    let mut f = f.clone();
    let ty = &f.ty;
    f.ty = parse_quote!(Option<#ty>);
    syn::punctuated::Pair::new(f, c)
  });

  // implement declare trait
  let fields_ident = stt.fields.iter().map(|f| f.ident.as_ref());

  let builder_fields_ident = builder_fields
    .iter()
    .map(|(f, _)| f.ident.as_ref().unwrap());

  let init_values = builder_fields.iter().map(|(f, attr)| {
    let field_name = f.ident.as_ref().unwrap();

    let or_default = attr.as_ref().and_then(|a| a.default.as_ref()).map_or_else(
      || {
        quote_spanned! { f.span() => expect(&format!("Required field `{}::{}` not set", stringify!(#name), stringify!(#field_name)))}
      },
      |d| {
      let default_method = field_default_method(field_name);
      quote_spanned!{d.default_token.span() =>  unwrap_or_else(|| Self::#default_method(ctx))}
    });

    quote_spanned! { f.span() => self.#field_name.#or_default }
  });

  let methods = builder_fields.iter().map(|(f, attr)| {
    let field_name = f.ident.as_ref().unwrap();
    let ty = &f.ty;
    let mut method_tokens = quote! {
      #[inline]
      #vis fn #field_name(mut self, v: #ty) -> Self {
        self.#field_name = Some(v);
        self
      }
    };

    if attr
      .as_ref()
      .map_or(true, |attr| attr.custom_convert.is_none())
    {
      let convert_method = field_convert_method(field_name);
      method_tokens.extend(quote! {
        #[inline]
        #vis fn #convert_method(v: #ty) -> #ty { v }
      });
    }

    if let Some(DeclareAttr { default: Some(d), .. }) = attr.as_ref() {
      let value = d.default_value(field_name);
      let default_method = field_default_method(field_name);
      method_tokens.extend(quote! {
        #vis fn #default_method(ctx: &mut BuildCtx) -> #ty { #value }
      });
    }

    method_tokens
  });

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
        #[allow(dead_code)]
        fn build(self, ctx: &mut BuildCtx) -> Self::Target {
          #name {
            #(#fields_ident : #init_values),* }
        }
      }

      impl #g_impl #builder #g_ty #g_where {
        #(#methods)*
      }
  };

  // println!("declare gen tokens {tokens}");
  Ok(tokens)
}

impl DefaultValue {
  fn default_value(&self, field_name: &Ident) -> TokenStream {
    match &self.value {
      Some(v) => {
        let expr: syn::Expr = v.parse().unwrap();
        let field_convert = field_convert_method(field_name);
        quote! {Self::#field_convert(#expr)}
      }
      None => {
        quote! {<_>::default()}
      }
    }
  }
}
fn collect_filed_and_attrs(
  stt: &mut DataStruct,
) -> Result<Punctuated<(syn::Field, Option<DeclareAttr>), token::Comma>> {
  let mut builder_fields = Punctuated::default();
  match &mut stt.fields {
    Fields::Named(named) => {
      named
        .named
        .pairs_mut()
        .try_for_each::<_, syn::Result<()>>(|mut pair| {
          let idx = pair
            .value()
            .attrs
            .iter()
            .position(|attr| attr.path.is_ident(DECLARE_ATTR));
          let builder_attr = if let Some(idx) = idx {
            let attr = pair.value_mut().attrs.remove(idx);
            let args: DeclareAttr = attr.parse_args()?;
            Some(args)
          } else {
            None
          };

          builder_fields.push(((*pair.value()).clone(), builder_attr));
          if let Some(c) = pair.punct() {
            builder_fields.push_punct(**c);
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
