use heck::ToSnakeCase;
use proc_macro2::TokenStream;
use quote::{ToTokens, quote, quote_spanned};
use syn::{
  Attribute, Fields, Ident, Result, Visibility,
  parse::{Parse, discouraged::Speculative},
  spanned::Spanned,
};

use crate::util::{declare_init_method, doc_attr};

const DECLARE_ATTR: &str = "declare";

pub(crate) fn declare_macro(stt: &mut syn::ItemStruct, is_attr: bool) -> Result<TokenStream> {
  let declarer = Declarer::new(stt)?;

  let mut tokens = gen_declare_struct(&declarer);
  tokens.extend(gen_declare_trait_impl(&declarer));
  tokens.extend(gen_set_methods(&declarer));
  tokens.extend(gen_obj_declarer_impl(&declarer));

  if !declarer.simple {
    tokens.extend(deref_fat_obj(&declarer));
    tokens.extend(widget_macro_to_tokens(declarer.host(), &declarer.original.vis));
  }

  if is_attr || declarer.simple {
    declarer.original.to_tokens(&mut tokens);
  }

  Ok(tokens)
}

fn gen_declare_struct(declarer: &Declarer) -> TokenStream {
  let Declarer { name, original, .. } = declarer;
  let syn::ItemStruct { vis, generics, .. } = original;
  let (_, g_ty, g_where) = generics.split_for_impl();
  let host = &original.ident;

  let fat_field = if !declarer.simple {
    quote! { fat_ಠ_ಠ: FatObj<()>, }
  } else {
    quote! {}
  };

  let builder_fields = declarer.no_skip_fields().map(|f| {
    let member = f.member();
    let ty = declarer.builder_field_ty(f);
    quote! {
      #[allow(clippy::type_complexity)]
      #member: Option<#ty>
    }
  });

  quote! {
    #vis struct #name #generics #g_where {
      #fat_field
      _marker: std::marker::PhantomData<#host #g_ty>,
      #(#builder_fields,)*
    }
  }
}

fn gen_declare_trait_impl(declarer: &Declarer) -> TokenStream {
  let Declarer { name, original, .. } = declarer;
  let (g_impl, g_ty, g_where) = original.generics.split_for_impl();
  let host = &original.ident;

  let fat_init = if !declarer.simple {
    quote! { fat_ಠ_ಠ: FatObj::new(()), }
  } else {
    quote! {}
  };

  let field_inits = declarer.no_skip_fields().map(|f| {
    let member = f.member();
    quote! { #member: None }
  });

  quote! {
    impl #g_impl Declare for #host #g_ty #g_where {
      type Builder = #name #g_ty;

      fn declarer() -> Self::Builder {
        #name {
          #fat_init
          _marker: std::marker::PhantomData,
          #(#field_inits,)*
        }
      }
    }
  }
}

fn gen_set_methods(declarer: &Declarer) -> TokenStream {
  let Declarer { name, original, .. } = declarer;
  let (g_impl, g_ty, g_where) = original.generics.split_for_impl();
  let vis = &original.vis;

  let methods = declarer
    .fields
    .iter()
    .filter(|f| f.need_set_method())
    .map(|f| {
      let field_name = f.member();
      let doc = f.doc_attr();
      let ty = &f.field.ty;
      let set_method = f.set_method_name();

      if declarer.simple || declarer.stateless {
        if f.is_strict() {
          quote! {
            #[inline] #doc
            #vis fn #set_method(&mut self, v: #ty) -> &mut Self {
              self.#field_name = Some(v);
              self
            }
          }
        } else {
          quote! {
            #[inline] #doc
            #vis fn #set_method(&mut self, v: impl Into<#ty>) -> &mut Self {
              self.#field_name = Some(v.into());
              self
            }
          }
        }
      } else {
        // Full & Stateful
        if f.is_strict() {
          quote! {
            #[inline] #doc
            #vis fn #set_method(&mut self, v: #ty) -> &mut Self {
              self.#field_name = Some(PipeValue::Value(v));
              self
            }
          }
        } else {
          quote! {
            #[inline] #[allow(clippy::type_complexity)] #doc
            #vis fn #set_method<_K: ?Sized>(
              &mut self,
              v: impl RInto<PipeValue<#ty>, _K>
            ) -> &mut Self {
              self.#field_name = Some(v.r_into());
              self
            }
          }
        }
      }
    });

  quote! {
    impl #g_impl #name #g_ty #g_where {
      #(#methods)*
    }
  }
}

fn gen_obj_declarer_impl(declarer: &Declarer) -> TokenStream {
  let Declarer { name, original, stateless, simple, .. } = declarer;
  let (g_impl, g_ty, g_where) = original.generics.split_for_impl();
  let host = &original.ident;

  let (target, finish_content) = if *simple {
    let target = if *stateless || original.fields.is_empty() {
      quote! { #host #g_ty }
    } else {
      quote! { Stateful<#host #g_ty> }
    };

    let finish_obj = declarer.build_widget(finish_values_simple(declarer));
    let finish_content = if *stateless || original.fields.is_empty() {
      quote! { #finish_obj }
    } else {
      quote! { Stateful::new(#finish_obj) }
    };
    (target, finish_content)
  } else {
    // Full
    let target = if *stateless || original.fields.is_empty() {
      quote! { FatObj<#host #g_ty> }
    } else {
      quote! { FatObj<Stateful<#host #g_ty>> }
    };

    let finish_content = if *stateless || original.fields.is_empty() {
      let finish_obj = declarer.build_widget(finish_values_simple(declarer));
      quote! { self.fat_ಠ_ಠ.map(|_| #finish_obj) }
    } else {
      gen_full_stateful_finish(declarer)
    };
    (target, finish_content)
  };

  quote! {
    impl #g_impl ObjDeclarer for #name #g_ty #g_where {
      type Target = #target;

      #[track_caller]
      fn finish(mut self) -> Self::Target {
        #finish_content
      }
    }
  }
}

fn gen_full_stateful_finish(declarer: &Declarer) -> TokenStream {
  let field_names: Vec<_> = declarer.all_members().collect();
  let field_values = field_values_full(declarer);
  let finish_obj = declarer.build_widget(field_names.iter().map(|m| quote! {#m.0}));

  let (field_tys, setter_logic): (Vec<_>, Vec<_>) = declarer
    .fields
    .iter()
    .map(|f| {
      let ty = &f.field.ty;
      let logic = if let Some(setter) = f.setter_name() {
        if let Some(st) = f.setter_ty() {
          quote! {
            let v: #st = v.into();
            this_ಠ_ಠ.write().#setter(v);
          }
        } else {
          quote! { this_ಠ_ಠ.write().#setter(v); }
        }
      } else {
        let member = f.member();
        quote! { this_ಠ_ಠ.write().#member = v; }
      };
      (ty, logic)
    })
    .unzip();

  quote! {
    #(#field_values)*
    let _obj_ಠ_ಠ = #finish_obj;
    let this_ಠ_ಠ = Stateful::new(_obj_ಠ_ಠ);

    let mut fat_ಠ_ಠ = self.fat_ಠ_ಠ;
    #(
      if let Some(o) = #field_names.1 {
        let this_ಠ_ಠ = this_ಠ_ಠ.clone_writer();
        let u = o.subscribe(move |v: #field_tys| { #setter_logic });
        fat_ಠ_ಠ.on_disposed(move |_| u.unsubscribe());
      }
    )*

    fat_ಠ_ಠ.map(move |_| this_ಠ_ಠ)
  }
}

pub struct Declarer<'a> {
  pub name: Ident,
  pub fields: Vec<DeclareField<'a>>,
  pub original: &'a syn::ItemStruct,
  pub validate: bool,
  pub simple: bool,
  pub stateless: bool,
}

impl<'a> Declarer<'a> {
  pub fn new(item_stt: &'a mut syn::ItemStruct) -> Result<Self> {
    let host = &item_stt.ident;
    let name = Ident::new(&format!("{host}Declarer"), host.span());
    let mut validate = false;
    let mut simple = false;
    let mut stateless = false;
    item_stt.attrs.retain(|attr| {
      if attr.path().is_ident(DECLARE_ATTR)
        && let Ok(attr) = attr.parse_args::<DeclareAttr>()
      {
        if attr.validate.is_some() {
          validate = true;
        }
        if attr.simple.is_some() {
          simple = true;
        }
        if attr.stateless.is_some() {
          stateless = true;
        }
        return false;
      }
      true
    });

    let (original, item_stt) = unsafe {
      let ptr = item_stt as *mut syn::ItemStruct;
      (&*ptr, &mut *ptr)
    };
    let fields = match &mut item_stt.fields {
      Fields::Named(named) => collect_fields(named.named.iter_mut()),
      Fields::Unnamed(unnamed) => collect_fields(unnamed.unnamed.iter_mut()),
      Fields::Unit => vec![],
    };

    Ok(Declarer { name, fields, original, validate, simple, stateless })
  }

  pub fn all_members(&self) -> impl Iterator<Item = &Ident> {
    self.fields.iter().map(|f| f.member())
  }

  pub fn no_skip_fields(&self) -> impl Iterator<Item = &DeclareField<'_>> {
    self.fields.iter().filter(|f| f.is_not_skip())
  }

  pub fn host(&self) -> &Ident { &self.original.ident }

  pub fn builder_field_ty(&self, f: &DeclareField) -> TokenStream {
    let ty = &f.field.ty;
    if self.simple || self.stateless {
      quote! { #ty }
    } else {
      quote! { PipeValue<#ty> }
    }
  }

  pub fn build_widget(&self, values: impl Iterator<Item = TokenStream>) -> TokenStream {
    let host = self.host();
    let finish_obj = match &self.original.fields {
      Fields::Named(_) => {
        let members = self.all_members();
        quote!(#host { #(#members: #values),* })
      }
      Fields::Unnamed(_) => quote!(#host(#(#values),*)),
      Fields::Unit => quote!(#host),
    };
    if self.validate {
      quote! { #finish_obj.declare_validate().expect("Validation failed") }
    } else {
      finish_obj
    }
  }
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
    if let Some(DefaultMeta { value: Some(value), .. }) = attr.default.as_ref() {
      Some(quote! { RFrom::r_from(#value) })
    } else if attr.default.is_some() || attr.skip.is_some() {
      Some(quote! { <_>::default() })
    } else {
      None
    }
  }

  pub fn set_method_name(&self) -> Ident {
    let name = self.field.ident.as_ref().unwrap();
    declare_init_method(name)
  }

  pub fn need_set_method(&self) -> bool {
    self
      .attr
      .as_ref()
      .is_none_or(|attr| attr.custom.is_none() && attr.skip.is_none())
  }

  pub fn doc_attr(&self) -> Option<&Attribute> { doc_attr(self.field) }

  pub fn setter_name(&self) -> Option<&Ident> {
    self
      .attr
      .as_ref()
      .and_then(|attr| attr.setter.as_ref())
      .map(|meta| &meta.method_name)
  }

  pub fn setter_ty(&self) -> Option<&syn::Type> {
    self
      .attr
      .as_ref()
      .and_then(|attr| attr.setter.as_ref())
      .and_then(|meta| meta.ty.as_ref())
  }
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
  custom_keyword!(default);
  custom_keyword!(custom);
  custom_keyword!(skip);
  custom_keyword!(strict);
  custom_keyword!(setter);
  custom_keyword!(validate);
  custom_keyword!(simple);
  custom_keyword!(stateless);
}

#[allow(dead_code)]
pub(crate) struct SetterMeta {
  pub(crate) setter_kw: kw::setter,
  pub(crate) eq_token: syn::Token![=],
  pub(crate) method_name: Ident,
  pub(crate) ty: Option<syn::Type>,
}

pub(crate) struct DefaultMeta {
  _default_kw: kw::default,
  _eq_token: Option<syn::token::Eq>,
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
  pub(crate) validate: Option<kw::validate>,
  pub(crate) simple: Option<kw::simple>,
  pub(crate) stateless: Option<kw::stateless>,
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

fn finish_values_simple<'a>(declarer: &'a Declarer) -> impl Iterator<Item = TokenStream> + 'a {
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

// Redundant declarer_set_methods_simple and declarer_set_methods_full removed.

fn field_values_full<'a>(declarer: &'a Declarer) -> impl Iterator<Item = TokenStream> + 'a {
  let host = declarer.host();
  declarer.fields.iter().map(move |f| {
    let f_name = f.member();
    let ty = &f.field.ty;

    let v = if f.is_not_skip() {
      if let Some(df) = f.default_value() {
        quote! {
          Option::take(&mut self.#f_name).map_or_else(
            || (#df, None),
            |v| v.unzip()
          )
        }
      } else {
        let err = format!("Required field `{host}::{f_name}` not set");
        quote! { Option::take(&mut self.#f_name).expect(#err).unzip() }
      }
    } else {
      // skip field must have default value.
      let df = f.default_value().unwrap();
      quote! { (#df, None) }
    };
    quote_spanned! { f.field.span() =>
      #[allow(clippy::type_complexity)]
      let #f_name: (#ty, Option<ValueStream<#ty>>) = #v;
    }
  })
}

fn deref_fat_obj(declarer: &Declarer) -> TokenStream {
  let (g_impl, g_ty, g_where) = declarer.original.generics.split_for_impl();
  let name = &declarer.name;

  quote! {
    impl #g_impl std::ops::Deref for #name #g_ty #g_where {
      type Target = FatObj<()>;
      #[inline]
      fn deref(&self) -> &Self::Target {
        &self.fat_ಠ_ಠ
      }
    }

    impl #g_impl std::ops::DerefMut for #name #g_ty #g_where {
      #[inline]
      fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.fat_ಠ_ಠ
      }
    }
  }
}

fn widget_macro_to_tokens(name: &Ident, vis: &Visibility) -> TokenStream {
  let macro_name = name.to_string().to_snake_case();
  let doc =
    format!("Macro used to generate a function widget using `{}` as the root widget.", macro_name);
  let macro_name = Ident::new(&macro_name, name.span());
  let export_attr = if matches!(vis, Visibility::Public(_)) {
    quote! { #[macro_export] }
  } else {
    quote! { #[allow(unused_macros)] }
  };
  quote! {
    #[allow(unused_macros)]
    #export_attr
    #[doc = #doc]
    macro_rules! #macro_name {
      ($($t: tt)*) => {
        fn_widget! { @ #name { $($t)* } }
      };
    }
    #[allow(unused_imports)]
    #vis use #macro_name;
  }
}
