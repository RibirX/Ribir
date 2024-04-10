use proc_macro2::TokenStream;
use quote::{quote, quote_spanned, ToTokens};
use syn::{
  parse::{discouraged::Speculative, Parse},
  spanned::Spanned,
  Fields, Ident, Result, Visibility,
};
const DECLARE_ATTR: &str = "declare";

pub struct Declarer<'a> {
  pub name: Ident,
  pub fields: Vec<DeclareField<'a>>,
}

pub(crate) fn simple_declarer_attr(stt: &mut syn::ItemStruct) -> Result<TokenStream> {
  if stt.fields.is_empty() {
    return empty_impl(stt);
  }
  let syn::ItemStruct { vis, generics, ident, fields, .. } = stt;
  let declarer = Declarer::new(ident, fields)?;

  let name = &declarer.name;
  let init_pairs = init_pairs(&declarer.fields, ident);
  let set_methods = declarer_set_methods(&declarer.fields, vis);
  let (g_impl, g_ty, g_where) = generics.split_for_impl();
  let (builder_f_names, builder_f_tys) = declarer.declare_names_tys();

  let mut tokens = quote! {
    #vis struct #name #generics #g_where {
      #(#builder_f_names : Option<#builder_f_tys>),*
    }

    impl #g_impl Declare for #ident #g_ty #g_where {
      type Builder = #name #g_ty;

      fn declarer() -> Self::Builder {
        #name { #(#builder_f_names : None ),*}
      }
    }

    impl #g_impl ObjDeclarer for #name #g_ty #g_where {
      type Target = State<#ident #g_ty>;

      #[inline]
      fn finish(mut self, ctx!(): &BuildCtx) -> Self::Target {
        State::value(#ident {#(#init_pairs),*})
      }
    }

    impl #g_impl #name #g_ty #g_where {
      #(#set_methods)*
    }
  };

  stt.to_tokens(&mut tokens);
  Ok(tokens)
}

fn empty_impl(stt: &syn::ItemStruct) -> Result<TokenStream> {
  let syn::ItemStruct { ident: name, fields, .. } = stt;
  let construct = match fields {
    Fields::Named(_) => quote!(#name {}),
    Fields::Unnamed(_) => quote!(#name()),
    Fields::Unit => quote!(#name),
  };
  let tokens = quote! {
    #stt

    impl Declare for #name  {
      type Builder = #name;
      fn declarer() -> Self::Builder { #construct }
    }

    impl ObjDeclarer for #name {
      type Target = #name;
      #[inline]
      fn finish(self, _: &BuildCtx) -> Self::Target { self }
    }
  };
  Ok(tokens)
}

impl<'a> Declarer<'a> {
  pub fn new(host: &'a Ident, stt_fields: &'a mut Fields) -> Result<Self> {
    let name = Ident::new(&format!("{host}Declarer"), host.span());
    let mut fields = vec![];
    match stt_fields {
      Fields::Named(named) => {
        for f in named.named.iter_mut() {
          let idx = f.attrs.iter().position(
            |attr| matches!(&attr.meta, syn::Meta::List(l) if l.path.is_ident(DECLARE_ATTR)),
          );
          let builder_attr = if let Some(idx) = idx {
            let attr = f.attrs.remove(idx);
            let args: DeclareAttr = attr.parse_args()?;
            Some(args)
          } else {
            None
          };
          fields.push(DeclareField { attr: builder_attr, field: f });
        }
      }
      Fields::Unit => {}
      Fields::Unnamed(unnamed) => {
        let err = syn::Error::new(unnamed.span(), "not support to derive for tuple struct");
        return Err(err);
      }
    }
    Ok(Declarer { name, fields })
  }

  pub fn declare_names_tys(&self) -> (Vec<&Ident>, Vec<&syn::Type>) {
    self
      .fields
      .iter()
      .filter(|f| f.is_not_skip())
      .map(|f| (f.member(), &f.field.ty))
      .unzip()
  }
}
mod kw {
  use syn::custom_keyword;
  custom_keyword!(rename);
  custom_keyword!(default);
  custom_keyword!(custom);
  custom_keyword!(skip);
  custom_keyword!(strict);
}

pub(crate) struct DefaultMeta {
  _default_kw: kw::default,
  _eq_token: Option<syn::token::Eq>,
  pub(crate) value: Option<syn::Expr>,
}

#[derive(Default)]
pub(crate) struct DeclareAttr {
  pub(crate) rename: Option<syn::Ident>,
  pub(crate) default: Option<DefaultMeta>,
  pub(crate) custom: Option<kw::custom>,
  // field with `skip` attr, will not generate setter method and use default to init value.
  pub(crate) skip: Option<kw::skip>,
  pub(crate) strict: Option<kw::strict>,
}

pub struct DeclareField<'a> {
  pub(crate) attr: Option<DeclareAttr>,
  pub(crate) field: &'a syn::Field,
}

impl<'a> DeclareField<'a> {
  pub fn member(&self) -> &Ident { self.field.ident.as_ref().unwrap() }

  pub fn is_not_skip(&self) -> bool {
    self
      .attr
      .as_ref()
      .map_or(true, |attr| attr.skip.is_none())
  }

  pub fn is_strict(&self) -> bool {
    self
      .attr
      .as_ref()
      .map_or(false, |attr| attr.strict.is_some())
  }

  pub fn default_value(&self) -> Option<TokenStream> {
    let attr = self.attr.as_ref()?;
    if let Some(DefaultMeta { value: Some(ref value), .. }) = attr.default.as_ref() {
      Some(quote! { From::from(#value) })
    } else if attr.default.is_some() || attr.skip.is_some() {
      Some(quote! { <_>::default() })
    } else {
      None
    }
  }

  pub fn set_method_name(&self) -> &Ident {
    self
      .attr
      .as_ref()
      .and_then(|attr| attr.rename.as_ref())
      .or(self.field.ident.as_ref())
      .unwrap()
  }

  pub fn need_set_method(&self) -> bool {
    self
      .attr
      .as_ref()
      .map_or(true, |attr| attr.custom.is_none() && attr.skip.is_none())
  }
}

impl Parse for DeclareAttr {
  fn parse(input: syn::parse::ParseStream) -> Result<Self> {
    let mut attr = DeclareAttr::default();
    while !input.is_empty() {
      let lookahead = input.lookahead1();

      // use input instead of lookahead to peek builtin, because need't complicate in
      // compile error.
      if lookahead.peek(kw::rename) {
        input.parse::<kw::rename>()?;
        input.parse::<syn::Token![=]>()?;
        attr.rename = Some(input.parse()?);
      } else if lookahead.peek(kw::custom) {
        attr.custom = Some(input.parse()?);
      } else if lookahead.peek(kw::default) {
        attr.default = Some(input.parse()?);
      } else if lookahead.peek(kw::skip) {
        attr.skip = Some(input.parse()?);
      } else if lookahead.peek(kw::strict) {
        attr.strict = Some(input.parse()?);
      } else {
        return Err(lookahead.error());
      }
      if let (Some(custom), Some(skip)) = (attr.custom.as_ref(), attr.skip.as_ref()) {
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

      if !input.is_empty() {
        input.parse::<syn::Token![,]>()?;
      }
    }
    Ok(attr)
  }
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

fn declarer_set_methods<'a>(
  fields: &'a [DeclareField], vis: &'a Visibility,
) -> impl Iterator<Item = TokenStream> + 'a {
  fields
    .iter()
    .filter(|f| f.need_set_method())
    .map(move |f| {
      let field_name = f.field.ident.as_ref().unwrap();
      let ty = &f.field.ty;
      let set_method = f.set_method_name();
      if f.is_strict() {
        quote! {
          #[inline]
          #vis fn #set_method(mut self, v: #ty) -> Self {
            self.#field_name = Some(v);
            self
          }
        }
      } else {
        quote! {
          #[inline]
          #vis fn #set_method(mut self, v: impl Into<#ty>) -> Self
          {
            self.#field_name = Some(v.into());
            self
          }
        }
      }
    })
}

fn init_pairs<'a>(
  fields: &'a [DeclareField], stt_name: &'a Ident,
) -> impl Iterator<Item = TokenStream> + 'a {
  fields.iter().map(move |f| {
    let f_name = f.member();

    if f.is_not_skip() {
      if let Some(df) = f.default_value() {
        quote! { #f_name: self.#f_name.take().unwrap_or_else(|| #df) }
      } else {
        let err = format!("Required field `{stt_name}::{f_name}` not set");
        quote_spanned! { f_name.span() => #f_name: self.#f_name.expect(#err) }
      }
    } else {
      // skip field must have default value.
      let df = f.default_value().unwrap();
      quote! { #f_name: #df }
    }
  })
}
