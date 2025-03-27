use proc_macro2::TokenStream;
use quote::{ToTokens, quote, quote_spanned};
use syn::{
  Attribute, Fields, Ident, Result, Visibility,
  parse::{Parse, discouraged::Speculative},
  spanned::Spanned,
};
const DECLARE_ATTR: &str = "declare";

pub struct Declarer<'a> {
  pub name: Ident,
  pub fields: Vec<DeclareField<'a>>,
  pub original: &'a syn::ItemStruct,
}

pub(crate) fn simple_declarer_attr(
  stt: &mut syn::ItemStruct, stateless: bool,
) -> Result<TokenStream> {
  let declarer = Declarer::new(stt)?;
  let Declarer { name, original, .. } = &declarer;
  let syn::ItemStruct { vis, generics, ident: host, .. } = original;

  let finish_obj = declarer.finish_obj(finish_values(&declarer));
  let set_methods = declarer_set_methods(&declarer.fields, vis);
  let (g_impl, g_ty, g_where) = generics.split_for_impl();
  let builder_members = declarer.builder_members();
  let builder_members_2 = declarer.builder_members();
  let builder_tys = declarer.builder_tys();

  let mut tokens = quote! {
    #vis struct #name #generics #g_where {
      _marker: std::marker::PhantomData<#host #g_ty>,
      #(#builder_members : Option<#builder_tys>),*
    }

    impl #g_impl Declare for #host #g_ty #g_where {
      type Builder = #name #g_ty;

      fn declarer() -> Self::Builder {
        #name {
          _marker: std::marker::PhantomData,
          #(#builder_members_2 : None ),*
        }
      }
    }

    impl #g_impl #name #g_ty #g_where {
      #(#set_methods)*
    }
  };

  if stateless || original.fields.is_empty() {
    tokens.extend(quote! {
      impl #g_impl ObjDeclarer for #name #g_ty #g_where {
        type Target = #host #g_ty;

        #[track_caller]
        fn finish(mut self) -> Self::Target {
          #finish_obj
        }
      }
    });
  } else {
    tokens.extend(quote! {
      impl #g_impl ObjDeclarer for #name #g_ty #g_where {
        type Target = State<#host #g_ty>;

        #[track_caller]
        fn finish(mut self) -> Self::Target {
          State::value(#finish_obj)
        }
      }
    });
  }

  original.to_tokens(&mut tokens);
  Ok(tokens)
}

fn finish_values<'a>(declarer: &'a Declarer) -> impl Iterator<Item = TokenStream> + 'a {
  let host = declarer.host();
  declarer.fields.iter().map(move |f| {
    let f_name = f.member();

    if f.is_not_skip() {
      if let Some(df) = f.default_value() {
        quote! { self.#f_name.take().unwrap_or_else(|| #df) }
      } else {
        let err = format!("Required field `{host}::{f_name}` not set");
        quote_spanned! { f_name.span() => self.#f_name.take().expect(#err) }
      }
    } else {
      // skip field must have default value.
      f.default_value().unwrap()
    }
  })
}

impl<'a> Declarer<'a> {
  pub fn new(item_stt: &'a mut syn::ItemStruct) -> Result<Self> {
    let host = &item_stt.ident;
    let name = Ident::new(&format!("{host}Declarer"), host.span());
    // Safety: During field collection, we only maintain a reference to `stt_fields`
    // and extract the `build` attribute from each field. No additional ownership
    // or mutation is performed, ensuring safe access.
    let (original, item_stt) = unsafe {
      let ptr = item_stt as *mut syn::ItemStruct;
      (&*ptr, &mut *ptr)
    };
    let fields = match &mut item_stt.fields {
      Fields::Named(named) => collect_fields(named.named.iter_mut()),
      Fields::Unnamed(unnamed) => collect_fields(unnamed.unnamed.iter_mut()),
      Fields::Unit => vec![],
    };

    Ok(Declarer { name, fields, original })
  }

  pub fn builder_members(&self) -> impl Iterator<Item = &Ident> {
    self.no_skip_fields().map(|f| f.member())
  }

  pub fn builder_tys(&self) -> impl Iterator<Item = &syn::Type> {
    self.no_skip_fields().map(|f| &f.field.ty)
  }

  pub fn all_members(&self) -> impl Iterator<Item = &Ident> {
    self.fields.iter().map(|f| f.member())
  }

  pub fn finish_obj(&self, values: impl Iterator<Item = TokenStream>) -> TokenStream {
    let host = self.host();

    match &self.original.fields {
      Fields::Named(_) => {
        let members = self.all_members();
        quote!(#host { #(#members: #values),* })
      }
      Fields::Unnamed(_) => quote!(#host(#(#values),*)),
      Fields::Unit => quote!(#host),
    }
  }

  fn no_skip_fields(&self) -> impl Iterator<Item = &DeclareField> {
    self.fields.iter().filter(|f| f.is_not_skip())
  }

  pub fn host(&self) -> &Ident { &self.original.ident }
}

fn collect_fields<'a>(fields: impl Iterator<Item = &'a mut syn::Field>) -> Vec<DeclareField<'a>> {
  fields
    .enumerate()
    .map(|(idx, f)| {
      if f.ident.is_none() {
        f.ident = Some(Ident::new(&format!("v_{idx}"), f.span()))
      }
      DeclareField { attr: take_build_attr(f), field: f }
    })
    .collect()
}

fn take_build_attr(field: &mut syn::Field) -> Option<DeclareAttr> {
  let idx = field
    .attrs
    .iter()
    .position(|attr| matches!(&attr.meta, syn::Meta::List(l) if l.path.is_ident(DECLARE_ATTR)));

  field.attrs.remove(idx?).parse_args().ok()
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
      .is_none_or(|attr| attr.skip.is_none())
  }

  pub fn is_strict(&self) -> bool {
    self
      .attr
      .as_ref()
      .is_some_and(|attr| attr.strict.is_some())
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
      .is_none_or(|attr| attr.custom.is_none() && attr.skip.is_none())
  }

  pub fn doc_attr(&self) -> Option<&Attribute> {
    self
      .field
      .attrs
      .iter()
      .find(|attr| matches!(&attr.meta, syn::Meta::NameValue(nv) if nv.path.is_ident("doc")))
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

pub fn declarer_set_methods<'a>(
  fields: &'a [DeclareField], vis: &'a Visibility,
) -> impl Iterator<Item = TokenStream> + 'a {
  fields
    .iter()
    .filter(|f| f.need_set_method())
    .map(move |f| {
      let field_name = f.field.ident.as_ref().unwrap();
      let doc_attr = f.doc_attr();
      let ty = &f.field.ty;
      let set_method = f.set_method_name();
      if f.is_strict() {
        quote! {
          #[inline]
          #doc_attr
          #vis fn #set_method(&mut self, v: #ty) -> &mut Self {
            self.#field_name = Some(v);
            self
          }
        }
      } else {
        quote! {
          #[inline]
          #doc_attr
          #vis fn #set_method(&mut self, v: impl Into<#ty>) -> &mut Self
          {
            self.#field_name = Some(v.into());
            self
          }
        }
      }
    })
}
