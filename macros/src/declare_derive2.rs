use crate::util::data_struct_unwrap;
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

const DECLARE: &str = "Declare2";
pub const DECLARER: &str = "Declarer2";
const DECLARE_ATTR: &str = "declare";

struct DefaultMeta {
  _default_kw: kw::default,
  _eq_token: Option<syn::token::Eq>,
  value: Option<syn::Expr>,
}

#[derive(Default)]
struct DeclareAttr {
  rename: Option<syn::Ident>,
  builtin: Option<kw::builtin>,
  default: Option<DefaultMeta>,
  custom: Option<kw::custom>,
  // field with `skip` attr, will not generate setter method and use default to init value.
  skip: Option<kw::skip>,
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
  custom_keyword!(custom);
  custom_keyword!(skip);
  // todo: tmp code, only for compatibility.
  custom_keyword!(convert);
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
      } else if lookahead.peek(kw::custom) {
        attr.custom = Some(input.parse()?);
      } else if lookahead.peek(kw::default) {
        attr.default = Some(input.parse()?);
      } else if lookahead.peek(kw::skip) {
        attr.skip = Some(input.parse()?);
      } else if lookahead.peek(kw::convert) {
        input.parse::<kw::convert>()?;
        input.parse::<token::Eq>()?;
        input.parse::<Ident>()?;
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

  let stt = data_struct_unwrap(data, DECLARE)?;

  if stt.fields.is_empty() {
    let tokens = quote! {
        impl Declare2 for #name  {
          type Builder = #name;
          fn declare2_builder() -> Self::Builder { #name }
        }

        impl DeclareBuilder for #name {
          type Target = #name;
          fn build(self, _: &BuildCtx) -> Self::Target { self }
        }
    };
    Ok(tokens)
  } else {
    struct_with_fields_gen(stt, vis, generics, name)
  }
}

fn struct_with_fields_gen(
  stt: &mut DataStruct,
  vis: &syn::Visibility,
  generics: &syn::Generics,
  name: &syn::Ident,
) -> syn::Result<TokenStream> {
  let (g_impl, g_ty, g_where) = generics.split_for_impl();

  let mut builder_fields = collect_filed_and_attrs(stt)?;

  // reverse name check.
  builder_fields
    .iter_mut()
    .for_each(DeclareField::check_reserve);

  let declarer = Ident::new(&format!("{name}{DECLARER}"), name.span());

  let mut builder_methods = quote! {};
  builder_fields
    .iter()
    .filter(|f| f.need_set_method())
    .for_each(|f| {
      let field_name = f.field.ident.as_ref().unwrap();
      let ty = &f.field.ty;
      let set_method = f.set_method_name();

      builder_methods.extend(quote! {
        #[inline]
        #vis fn #set_method<_M, _V>(mut self, v: _V) -> Self
          where DeclareInit<#ty>: DeclareFrom<_V, _M>
        {
          self.#field_name = Some(DeclareInit::declare_from(v));
          self
        }
      });
    });

  // builder define
  let def_fields = builder_fields.pairs().map(|p| {
    let (f, c) = p.into_tuple();
    let mut f = f.field.clone();
    let ty = &f.ty;
    f.ty = parse_quote!(Option<DeclareInit<#ty>>);
    syn::punctuated::Pair::new(f, c)
  });

  // implement declare trait

  let fill_default = builder_fields.iter().filter_map(|f| {
    let attr = f.attr.as_ref()?;
    let field_name = f.member();

    let set_default_value = match (&attr.default, &attr.skip) {
      (Some(df), None) if df.value.is_some() => {
        let v = df.value.as_ref();
        let method = f.set_method_name();
        Some(quote! { self = self.#method(#v); })
      }
      (Some(df), Some(_)) if df.value.is_some() => {
        let v = df.value.as_ref();
        Some(quote! { self.#field_name = Some(DeclareInit::declare_from(#v)); })
      }
      (Some(_), _) | (_, Some(_)) => {
        Some(quote! { self.#field_name = Some(DeclareInit::default()); })
      }
      (None, None) => None,
    };
    set_default_value.map(|set_default_value| {
      quote! {
        if self.#field_name.is_none() {
          #set_default_value
        }
      }
    })
  });

  let unzip_fields = builder_fields.iter().map(|df| {
    let field_name = df.field.ident.as_ref().unwrap();
    let method = df.set_method_name();
    quote_spanned! { field_name.span() =>
      let #field_name = self.#field_name.expect(&format!(
        "Required field `{}::{}` not set, use method `{}` init it",
        stringify!(#name), stringify!(#field_name), stringify!(#method)
      )).unzip();
    }
  });

  let field_names = builder_fields.iter().map(|f| f.field.ident.as_ref());
  let field_names2 = field_names.clone();
  let field_names3 = field_names.clone();

  let tokens = quote! {
      #vis struct #declarer #g_impl #g_where {
        #(#def_fields)*
      }

      impl #g_impl Declare2 for #name #g_ty #g_where {
        type Builder = #declarer #g_ty;

        fn declare2_builder() -> Self::Builder {
          #declarer { #(#field_names : None ),*}
        }
      }

      impl #g_impl #declarer #g_ty #g_where {
        #builder_methods
      }

      impl #g_impl DeclareBuilder for #declarer #g_ty #g_where {
        type Target = State<#name #g_ty>;
        fn build(mut self, ctx: &BuildCtx) -> Self::Target {
          set_build_ctx!(ctx);

          #(#fill_default)*
          #(#unzip_fields)*
          let mut _ribir = State::Stateless(#name {
            #(#field_names2 : #field_names2.0),*
          });
          #(
            if let Some(u) = #field_names3.1 {
              let mut _ribir2 = _ribir.clone_stateful();
              let h = u.subscribe(move |v| _ribir2.state_ref().#field_names3 = v)
              .unsubscribe_when_dropped();
              _ribir.to_stateful().own_data(h);
            }
          );*

          _ribir
        }
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
            builder_fields.push_punct(*c);
          }

          Ok(())
        })?;
    }
    Fields::Unit => <_>::default(),
    Fields::Unnamed(unnamed) => {
      let err = syn::Error::new(
        unnamed.span(),
        format!("`{DECLARE}` not be supported to derive for tuple struct"),
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

  fn need_set_method(&self) -> bool {
    self
      .attr
      .as_ref()
      .map_or(true, |attr| attr.custom.is_none() && attr.skip.is_none())
  }

  fn check_reserve(&mut self) {
    // reverse name check.
    let reserve_ident = &crate::widget_macro::RESERVE_IDENT;

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
